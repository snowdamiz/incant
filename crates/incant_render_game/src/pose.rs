//! Explicit pose diagnostic host. Ordinary NativeRenderHost is unchanged.
use crate::{NativeRenderSettings, RenderInputConsumer};
use incant_game_runtime::{SceneHost, SceneHostPresentation};
use incant_render_data::{SceneInput, UiInput};
use incant_render_native::{
    UpdateKind, VisualCaches,
    interpolation::{PoseError, PoseReceipt, PoseRenderInput, PoseSyncError},
};
use incant_runtime::{NativeWorld, RuntimeError};
use std::sync::Arc;
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
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error>;
    fn present_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        receipt: PoseReceipt,
        ui: Option<&Arc<UiInput>>,
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
    #[error(transparent)]
    Ui(#[from] incant_ui::UiError),
    #[error(transparent)]
    UiCheckpoint(#[from] incant_ui::UiCheckpointError),
    #[error("pose consumer cannot admit the required screen-space UI")]
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

/// Bounded composed host. Callers explicitly supply discontinuity requests.
/// Screen UI retains the same transactional Binding as the ordinary host; it
/// stays at the latest accepted fixed tick and is never interpolated.
pub struct PoseRenderHost<C: PoseConsumer> {
    input: PoseRenderInput,
    consumer: C,
    epoch: u64,
    ui: Option<crate::ui::Binding>,
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
    pub fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.ui.as_ref().map(|ui| &ui.runtime)
    }
    pub fn ui_input(&self) -> Option<&Arc<UiInput>> {
        self.ui.as_ref().map(|ui| &ui.input)
    }
    fn sync_epoch(
        &mut self,
        world: &NativeWorld,
        epoch: u64,
    ) -> Result<(), PoseHostError<C::Error>> {
        if world.requires_ui_checkpoint() && self.ui.is_none() {
            return Err(PoseHostError::Unsupported);
        }
        let next = self
            .ui
            .as_mut()
            .map(|ui| ui.prepare(world, ui.viewport()))
            .transpose()?;
        let view = world.spatial_view()?;
        let ui_input = next.as_ref().map(|next| next.input());
        let consumer = &mut self.consumer;
        let accept = |input: &SceneInput, camera: &str, kind| {
            consumer.admit_pose(input, camera, kind, view.tick(), ui_input)
        };
        if self.input.committed().has_text_binding() {
            self.input
                .try_sync_with_text(&view, &world.text_view()?, epoch, accept)?;
        } else {
            self.input.try_sync(&view, epoch, accept)?;
        }
        if let Some(next) = next {
            self.ui.as_mut().expect("UI owner").commit(next, world)?;
        }
        self.epoch = epoch;
        Ok(())
    }
    fn prepare_state(
        world: &NativeWorld,
        settings: NativeRenderSettings<C::Settings>,
        checkpoint: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> Result<Self, PoseHostError<C::Error>> {
        if ((settings.ui.is_some() || world.requires_ui_checkpoint()) && !C::SUPPORTS_UI)
            || ((world.requires_ui_checkpoint() || checkpoint.is_some()) && settings.ui.is_none())
        {
            return Err(PoseHostError::Unsupported);
        }
        let view = world.spatial_view()?;
        let caches = VisualCaches {
            models: &settings.model_cache,
            textures: &settings.texture_cache,
        };
        let input = if let Some(text) = settings.text {
            PoseRenderInput::load_with_text(
                &view,
                &world.text_view()?,
                caches,
                settings.camera,
                0,
                text,
            )?
        } else {
            PoseRenderInput::load(&view, caches, settings.camera, 0)?
        };
        let committed = input.committed();
        let ui = settings
            .ui
            .map(|settings| crate::ui::Binding::load(world, committed, settings, checkpoint))
            .transpose()?;
        let consumer = C::prepare_at_tick(
            committed.input(),
            committed.camera_label(),
            ui.as_ref().map(|ui| &ui.input),
            committed.tick(),
            settings.consumer,
        )
        .map_err(PoseHostError::Consumer)?;
        Ok(Self {
            input,
            consumer,
            epoch: 0,
            ui,
        })
    }
}
impl<C: PoseConsumer> SceneHost for PoseRenderHost<C> {
    const SUPPORTS_UI: bool = C::SUPPORTS_UI;
    type Settings = NativeRenderSettings<C::Settings>;
    type Error = PoseHostError<C::Error>;
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.ui()
    }
    fn stage_ui(
        &mut self,
        world: &NativeWorld,
        events: &[incant_input::InputEvent],
        receipts: &[incant_ui::UiEvent],
    ) -> Result<Option<incant_game_runtime::RoutedUiInput>, Self::Error> {
        self.ui
            .as_mut()
            .map(|ui| ui.stage(world, events, receipts))
            .transpose()
            .map_err(Into::into)
    }
    fn cancel_ui(&mut self) {
        if let Some(ui) = &mut self.ui {
            ui.cancel();
        }
    }
    fn resize_ui(&mut self, world: &NativeWorld, viewport: [f32; 2]) -> Result<(), Self::Error> {
        let ui = self.ui.as_mut().ok_or(PoseHostError::Unsupported)?;
        let next = ui.prepare(world, viewport)?;
        self.consumer
            .admit_ui(next.input())
            .map_err(PoseHostError::Consumer)?;
        ui.commit(next, world)?;
        Ok(())
    }
    fn prepare(world: &NativeWorld, settings: Self::Settings) -> Result<Self, Self::Error> {
        Self::prepare_state(world, settings, None)
    }
    fn prepare_checkpoint(
        world: &NativeWorld,
        settings: Self::Settings,
        checkpoint: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> incant_game_runtime::Result<Self, Self::Error> {
        Self::prepare_state(world, settings, checkpoint)
            .map_err(incant_game_runtime::GameError::Host)
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
        if world.requires_ui_checkpoint() && self.ui.is_none() {
            return Err(PoseHostError::Unsupported);
        }
        match request {
            PoseRequest::Display { fraction } => {
                if let Some(ui) = &self.ui {
                    ui.validate_committed(world)?;
                }
                // Never render a stale host as though it represented this tick.
                self.input.validate_committed(&world.spatial_view()?)?;
                let consumer = &mut self.consumer;
                let ui = self.ui.as_ref().map(|ui| &ui.input);
                Ok(self.input.present(fraction, |input, camera, receipt| {
                    consumer.present_pose(input, camera, receipt, ui)?;
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
