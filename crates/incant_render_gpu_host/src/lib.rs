//! Mandatory GPU consumer for a hosted native game. Targets are acquired inside
//! each accepted sync, submitted immediately, then presented. No authoring path.
//! Screen-space UI is admitted before any target is consumed and drawn over the
//! finished world frame in the same target (incant_render `ui` contract).
#[cfg(feature = "native-metalfx")]
mod metalfx;
#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]
mod metalfx_tests;
mod motion;
#[cfg(feature = "native-metalfx")]
pub use metalfx::{MetalFxSettings, MetalFxSetup, MetalFxTargets};
pub use motion::{MotionReset, MotionTargets};
mod resolution;
mod resolution_mailbox;
use incant_render::resolution::{ResolutionConfig, ResolutionError, ResolutionFrame};
use incant_render::{Renderer, RetainedScene, RetainedTarget, SceneStats, UiLayer, UiStats};
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::RenderInputConsumer;
use incant_render_game::TextureRefreshConsumer;
use incant_render_native::UpdateKind;
pub use resolution::ResolutionTargets;
use std::time::Instant;
use std::{error::Error, sync::Arc};
use thiserror::Error;

/// Trusted host-owned output frame. Dropping it before presentation must release
/// an acquired surface frame. Present is called only after submission; failure
/// cannot undo GPU work and terminalizes the consumer/game composition.
pub trait RenderFrame: Sized {
    type Error: Error + 'static;
    fn target(&self) -> RetainedTarget<'_>;
    fn present(self) -> Result<(), Self::Error>;
}
/// Surface/headless output ownership belongs to the native host. A frame must use
/// the supplied renderer's device. Recoverable surface resize/reacquisition policy
/// belongs inside this method; returning Err fails the already accepted game tick.
pub trait TargetProvider {
    type Error: Error + 'static;
    type Frame: RenderFrame<Error = Self::Error>;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Self::Frame, Self::Error>;
    /// Explicit opt-in policy, sampled once when this consumer is prepared.
    fn resolution_config(&self) -> Option<ResolutionConfig> {
        None
    }
    /// Explicit opt-in plus monotonic trusted camera-cut epoch. Simulation ticks
    /// and arbitrary clock discontinuities never implicitly select history.
    #[cfg(feature = "native-metalfx")]
    fn metalfx(&self) -> Option<crate::MetalFxSetup> {
        None
    }
    fn motion_epoch(&self) -> Option<u64> {
        None
    }
}
pub struct GpuRenderSettings<P: TargetProvider> {
    pub renderer: Arc<Renderer>,
    pub targets: P,
}
#[derive(Debug, Error)]
pub enum GpuHostError<E> {
    #[error("native renderer failed: {0}")]
    Render(#[source] Box<dyn Error + Send + Sync>),
    #[error("native render target failed: {0}")]
    Target(#[source] E),
    #[error("native GPU consumer is terminal after an earlier failure")]
    Failed,
    #[error("native GPU frame counter exhausted")]
    FrameLimit,
    #[error("dynamic resolution failed: {0}")]
    Resolution(#[from] ResolutionError),
}
struct Live<P> {
    scene: RetainedScene,
    targets: P,
    ui: UiLayer,
    completion: Option<resolution::Completion>,
    resolution_config: Option<ResolutionConfig>,
    motion_epoch: Option<u64>,
    #[cfg(feature = "native-metalfx")]
    metalfx: Option<crate::MetalFxSetup>,
}
/// Owns the actual GPU bindings plus its target provider. NativeRenderHost owns
/// the sole CPU adapter and calls this consumer inside every mandatory sync.
/// Read-only metrics escape; mutable scene/target/command-buffer access does not.
pub struct GpuRenderConsumer<P: TargetProvider> {
    renderer: Arc<Renderer>,
    live: Option<Live<P>>,
    frames: u64,
}
impl<P: TargetProvider> GpuRenderConsumer<P> {
    /// Frames successfully submitted and presented. A failed presentation may
    /// follow GPU submission; it is terminal and does not increment this count.
    pub fn accepted_frames(&self) -> u64 {
        self.frames
    }
    pub fn is_failed(&self) -> bool {
        self.live.is_none()
    }
    pub fn stats(&self) -> Option<&SceneStats> {
        self.live.as_ref().map(|live| live.scene.stats())
    }
    /// Last submitted sizing receipt, independent of output/UI dimensions.
    pub fn resolution_frame(&self) -> Option<ResolutionFrame> {
        self.live
            .as_ref()
            .and_then(|live| live.scene.resolution_frame())
    }
    #[cfg(feature = "native-metalfx")]
    pub fn metalfx_selection(&self) -> Option<incant_render::metalfx::Selection> {
        self.live.as_ref().and_then(|l| l.scene.metalfx_selection())
    }
    pub fn motion_frame(&self) -> Option<incant_render::motion::MotionFrame> {
        self.live
            .as_ref()
            .and_then(|live| live.scene.motion_frame())
    }
    pub fn discarded_resolution_samples(&self) -> u64 {
        self.live
            .as_ref()
            .and_then(|live| live.completion.as_ref())
            .map_or(0, |c| c.discarded)
    }
    /// The accepted screen-space UI resources, if a UI frame was admitted.
    pub fn ui_stats(&self) -> Option<UiStats> {
        self.live.as_ref().and_then(|live| live.ui.stats())
    }
}
impl<P: TargetProvider> RenderInputConsumer for GpuRenderConsumer<P> {
    const SUPPORTS_UI: bool = true;
    type Settings = GpuRenderSettings<P>;
    type Error = GpuHostError<P::Error>;
    fn prepare(
        input: &SceneInput,
        camera: &str,
        ui: Option<&Arc<UiInput>>,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error> {
        let config = settings.targets.resolution_config();
        if let Some(config) = config {
            resolution::validate(config)?;
        }
        let mut scene = settings
            .renderer
            .prepare_retained(input, camera)
            .map_err(GpuHostError::Render)?;
        scene.set_dynamic_resolution(config)?;
        let motion_epoch = settings.targets.motion_epoch();
        #[cfg(feature = "native-metalfx")]
        let metalfx = settings.targets.metalfx();
        #[cfg(feature = "native-metalfx")]
        let ordinary_motion = metalfx.is_none();
        #[cfg(not(feature = "native-metalfx"))]
        let ordinary_motion = true;
        if motion_epoch.is_some() && ordinary_motion {
            scene
                .enable_motion_history(&settings.renderer)
                .map_err(GpuHostError::Render)?;
        }
        #[cfg(feature = "native-metalfx")]
        if let Some(setup) = metalfx {
            scene
                .enable_metalfx(&settings.renderer, setup.preference, setup.config)
                .map_err(GpuHostError::Render)?;
            scene
                .prepare_metalfx_extent(&settings.renderer, setup.output_extent)
                .map_err(GpuHostError::Render)?;
        }
        let mut layer = settings.renderer.ui_layer();
        if let Some(ui) = ui {
            let stage = layer
                .stage(&settings.renderer, ui)
                .map_err(GpuHostError::Render)?;
            layer.accept(stage).map_err(GpuHostError::Render)?;
        }
        // Startup prepares real GPU resources but does not present a game frame
        // before guest/audio setup succeeds. First presentation follows tick 1.
        Ok(Self {
            renderer: settings.renderer,
            live: Some(Live {
                scene,
                targets: settings.targets,
                ui: layer,
                completion: config.map(|_| resolution::Completion::default()),
                resolution_config: config,
                motion_epoch,
                #[cfg(feature = "native-metalfx")]
                metalfx,
            }),
            frames: 0,
        })
    }
    fn admit_ui(&mut self, ui: &Arc<UiInput>) -> Result<(), Self::Error> {
        let live = self.live.as_mut().ok_or(GpuHostError::Failed)?;
        let stage = live
            .ui
            .stage(&self.renderer, ui)
            .map_err(GpuHostError::Render)?;
        live.ui.accept(stage).map_err(GpuHostError::Render)
    }
    fn accept(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        // UI admission precedes taking the live state or a target: an ordinary
        // refusal keeps every accepted resource and the consumer usable.
        let ui = match (ui, self.live.as_ref()) {
            (Some(ui), Some(live)) => Some(
                live.ui
                    .stage(&self.renderer, ui)
                    .map_err(GpuHostError::Render)?,
            ),
            _ => None,
        };
        // Taking ownership makes both ordinary errors and Rust unwinds terminal,
        // even for direct trusted callers outside HostedGameSession. Abort cannot
        // run cleanup. The composite separately silences offline audio on failure.
        let mut live = self.live.take().ok_or(GpuHostError::Failed)?;
        self.renderer.check_device().map_err(GpuHostError::Render)?;
        if let Some(completion) = &mut live.completion
            && let Some((frame, elapsed)) = completion.take()?
            && !live
                .scene
                .record_frame_time(&frame, elapsed)
                .unwrap_or(false)
        {
            completion.discarded = completion.discarded.saturating_add(1);
        }
        let motion_epoch = live.targets.motion_epoch();
        if motion_epoch.is_some() != live.motion_epoch.is_some()
            || motion_epoch
                .zip(live.motion_epoch)
                .is_some_and(|(next, last)| next < last)
        {
            return Err(GpuHostError::Render(Box::new(
                incant_render::motion::MotionError::Sequence,
            )));
        }
        if motion_epoch != live.motion_epoch {
            live.scene.invalidate_motion_history();
        }
        let start = live.completion.as_ref().map(|_| Instant::now());
        let next = self.frames.checked_add(1).ok_or(GpuHostError::FrameLimit)?;
        let mut replacement = if kind == UpdateKind::Rebuilt {
            Some(
                self.renderer
                    .prepare_retained(input, camera)
                    .map_err(GpuHostError::Render)?,
            )
        } else {
            None
        };
        if let Some(scene) = &mut replacement {
            // Replacement has fresh timing ownership. Outstanding old callbacks
            // are retained in the bounded slot and discarded by the new owner.
            scene.set_dynamic_resolution(live.resolution_config)?;
            #[cfg(feature = "native-metalfx")]
            let ordinary_motion = live.metalfx.is_none();
            #[cfg(not(feature = "native-metalfx"))]
            let ordinary_motion = true;
            if motion_epoch.is_some() && ordinary_motion {
                scene
                    .enable_motion_history(&self.renderer)
                    .map_err(GpuHostError::Render)?;
            }
        }
        #[cfg(feature = "native-metalfx")]
        if let (Some(scene), Some(setup)) = (&mut replacement, live.metalfx) {
            scene
                .enable_metalfx(&self.renderer, setup.preference, setup.config)
                .map_err(GpuHostError::Render)?;
            // Automatic topology replacement occurs during presentation. It
            // cannot synchronously prewarm: prefer falls back; require refuses
            // fail-closed. Required-native topology must be staged beforehand.
        }
        let frame = live
            .targets
            .acquire(&self.renderer)
            .map_err(GpuHostError::Target)?;
        replacement
            .as_mut()
            .unwrap_or(&mut live.scene)
            .submit(&self.renderer, input, frame.target())
            .map_err(GpuHostError::Render)?;
        if let Some(stage) = ui {
            live.ui
                .submit(&self.renderer, stage, &frame.target())
                .map_err(GpuHostError::Render)?;
        }
        self.renderer.check_device().map_err(GpuHostError::Render)?;
        frame.present().map_err(GpuHostError::Target)?;
        if let Some(scene) = replacement {
            live.scene = scene;
        }
        if let (Some(completion), Some(start), Some(receipt)) =
            (&live.completion, start, live.scene.resolution_frame())
        {
            completion.register(&self.renderer, start, receipt)?;
        }
        live.motion_epoch = motion_epoch;
        self.live = Some(live);
        self.frames = next;
        Ok(())
    }
}

/// Opaque resources prepared without taking a target or drawing.
pub struct PreparedGpuTextures {
    scene: RetainedScene,
    motion_epoch: Option<u64>,
    #[cfg(feature = "native-metalfx")]
    metalfx: Option<MetalFxSetup>,
}
impl<P: TargetProvider> TextureRefreshConsumer for GpuRenderConsumer<P> {
    type Prepared = PreparedGpuTextures;
    fn prepare_textures(
        &self,
        input: &SceneInput,
        camera: &str,
    ) -> Result<Self::Prepared, Self::Error> {
        let live = self.live.as_ref().ok_or(GpuHostError::Failed)?;
        self.renderer.check_device().map_err(GpuHostError::Render)?;
        let mut scene = self
            .renderer
            .prepare_retained(input, camera)
            .map_err(GpuHostError::Render)?;
        scene.set_dynamic_resolution(live.resolution_config)?;
        #[cfg(feature = "native-metalfx")]
        let ordinary_motion = live.metalfx.is_none();
        #[cfg(not(feature = "native-metalfx"))]
        let ordinary_motion = true;
        if live.motion_epoch.is_some() && ordinary_motion {
            scene
                .enable_motion_history(&self.renderer)
                .map_err(GpuHostError::Render)?;
        }
        #[cfg(feature = "native-metalfx")]
        if let Some(setup) = live.metalfx {
            scene
                .enable_metalfx(&self.renderer, setup.preference, setup.config)
                .map_err(GpuHostError::Render)?;
            scene
                .prepare_metalfx_extent(&self.renderer, setup.output_extent)
                .map_err(GpuHostError::Render)?;
        }
        self.renderer.check_device().map_err(GpuHostError::Render)?;
        Ok(PreparedGpuTextures {
            scene,
            motion_epoch: live.motion_epoch,
            #[cfg(feature = "native-metalfx")]
            metalfx: live.metalfx,
        })
    }
    fn validate_textures(&self, prepared: &Self::Prepared) -> Result<(), Self::Error> {
        let live = self.live.as_ref().ok_or(GpuHostError::Failed)?;
        #[cfg(feature = "native-metalfx")]
        if prepared.metalfx != live.metalfx {
            return Err(GpuHostError::Render(Box::new(
                incant_render::metalfx::MetalFxError::Unavailable(
                    incant_render::metalfx::Fallback::UnpreparedExtent,
                ),
            )));
        }
        if prepared.motion_epoch != live.motion_epoch
            || prepared.motion_epoch != live.targets.motion_epoch()
        {
            return Err(GpuHostError::Render(Box::new(
                incant_render::motion::MotionError::Sequence,
            )));
        }
        self.renderer
            .check_device_status()
            .map_err(GpuHostError::Render)
    }
    fn publish_textures(&mut self, prepared: &mut Self::Prepared) {
        // The outer caller owns this retirement storage throughout the lock.
        std::mem::swap(
            &mut self.live.as_mut().expect("validated live consumer").scene,
            &mut prepared.scene,
        );
    }
}
