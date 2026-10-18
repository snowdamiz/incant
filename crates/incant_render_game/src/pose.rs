//! Explicit static-pose diagnostic host. Ordinary NativeRenderHost is unchanged.
use crate::{NativeRenderSettings, RenderInputConsumer};
use incant_game_runtime::{SceneHost, SceneHostPresentation};
use incant_render_data::SceneInput;
use incant_render_native::{
    UpdateKind, VisualCaches,
    interpolation::{PoseError, PoseReceipt, PoseRenderInput, PoseSyncError},
};
use incant_runtime::{NativeWorld, RuntimeError};
use thiserror::Error;

/// A consumer must admit canonical states without drawing, and separately draw
/// each requested numeric pose. Preparation and either operation must actually
/// validate resources. A temporal/history consumer needs its own qualification.
pub trait PoseConsumer: RenderInputConsumer {
    fn admit_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        tick: u64,
    ) -> Result<(), Self::Error>;
    fn present_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        receipt: PoseReceipt,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug)]
pub enum PoseRequest {
    Display {
        fraction: f64,
    },
    /// Snap to the exact current world pose with a strictly greater epoch.
    /// After committing discontinuous motion, call this BEFORE displaying it.
    /// It resets current history immediately; it does not arm a future tick.
    /// Resume, resize and discarded simulation debt also require this reset.
    Reset {
        continuity_epoch: u64,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PoseReply {
    Displayed(PoseReceipt),
    Reset { continuity_epoch: u64 },
}
#[derive(Debug, Error)]
pub enum PoseHostError<E> {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Pose(#[from] PoseError),
    #[error("pose consumer failed: {0}")]
    Consumer(#[source] E),
    #[error("this initial pose host does not support text or screen-space UI")]
    Unsupported,
}
impl<E> From<PoseSyncError<E>> for PoseHostError<E> {
    fn from(value: PoseSyncError<E>) -> Self {
        match value {
            PoseSyncError::Pose(error) => Self::Pose(error),
            PoseSyncError::Consumer(error) => Self::Consumer(error),
        }
    }
}

/// Bounded composed fixture host. No script teleport/reset API or native window
/// selection is implied. Callers explicitly supply discontinuity requests.
/// Screen-space UI is refused pending integration of the existing UI owner.
pub struct PoseRenderHost<C: PoseConsumer> {
    input: PoseRenderInput,
    consumer: C,
    epoch: u64,
}
impl<C: PoseConsumer> PoseRenderHost<C> {
    pub fn committed(&self) -> &incant_render_native::NativeRenderInput {
        self.input.committed()
    }
    pub fn consumer(&self) -> &C {
        &self.consumer
    }
    pub fn continuity_epoch(&self) -> u64 {
        self.epoch
    }
    fn sync_epoch(
        &mut self,
        world: &NativeWorld,
        epoch: u64,
    ) -> Result<(), PoseHostError<C::Error>> {
        if world.requires_ui_checkpoint() {
            return Err(PoseHostError::Unsupported);
        }
        let view = world.spatial_view()?;
        let consumer = &mut self.consumer;
        self.input.try_sync(&view, epoch, |input, camera, kind| {
            consumer.admit_pose(input, camera, kind, view.tick())
        })?;
        self.epoch = epoch;
        Ok(())
    }
}
impl<C: PoseConsumer> SceneHost for PoseRenderHost<C> {
    type Settings = NativeRenderSettings<C::Settings>;
    type Error = PoseHostError<C::Error>;
    fn prepare(world: &NativeWorld, settings: Self::Settings) -> Result<Self, Self::Error> {
        if settings.text.is_some() || settings.ui.is_some() || world.requires_ui_checkpoint() {
            return Err(PoseHostError::Unsupported);
        }
        let input = PoseRenderInput::load(
            &world.spatial_view()?,
            VisualCaches {
                models: &settings.model_cache,
                textures: &settings.texture_cache,
            },
            settings.camera,
            0,
        )?;
        let committed = input.committed();
        let consumer = C::prepare_at_tick(
            committed.input(),
            committed.camera_label(),
            None,
            committed.tick(),
            settings.consumer,
        )
        .map_err(PoseHostError::Consumer)?;
        Ok(Self {
            input,
            consumer,
            epoch: 0,
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        self.sync_epoch(world, self.epoch)
    }
}
impl<C: PoseConsumer> SceneHostPresentation for PoseRenderHost<C> {
    type Presentation = PoseRequest;
    type Presented = PoseReply;
    fn present(
        &mut self,
        world: &NativeWorld,
        request: PoseRequest,
    ) -> Result<PoseReply, Self::Error> {
        if world.requires_ui_checkpoint() {
            return Err(PoseHostError::Unsupported);
        }
        match request {
            PoseRequest::Display { fraction } => {
                // Never render a stale host as though it represented this tick.
                self.input.validate_committed(&world.spatial_view()?)?;
                let consumer = &mut self.consumer;
                Ok(self.input.present(fraction, |input, camera, receipt| {
                    consumer.present_pose(input, camera, receipt)?;
                    Ok::<_, C::Error>(PoseReply::Displayed(receipt))
                })?)
            }
            PoseRequest::Reset { continuity_epoch } => {
                if continuity_epoch <= self.epoch {
                    return Err(PoseError::Sequence.into());
                }
                self.sync_epoch(world, continuity_epoch)?;
                Ok(PoseReply::Reset { continuity_epoch })
            }
        }
    }
}
