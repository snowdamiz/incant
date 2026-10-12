//! Mandatory GPU consumer for a hosted native game. Targets are acquired inside
//! each accepted sync, submitted immediately, then presented. No authoring path.
//! Screen-space UI is admitted before any target is consumed and drawn over the
//! finished world frame in the same target (incant_render `ui` contract).
use incant_render::{Renderer, RetainedScene, RetainedTarget, SceneStats, UiLayer, UiStats};
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::RenderInputConsumer;
use incant_render_native::UpdateKind;
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
}
struct Live<P> {
    scene: RetainedScene,
    targets: P,
    ui: UiLayer,
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
        let scene = settings
            .renderer
            .prepare_retained(input, camera)
            .map_err(GpuHostError::Render)?;
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
        self.live = Some(live);
        self.frames = next;
        Ok(())
    }
}
