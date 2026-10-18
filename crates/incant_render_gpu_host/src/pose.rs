//! Explicit target adapter for canonical admission followed by pose display.
use crate::{GpuRenderConsumer, PresentationMode, TargetProvider};
use incant_render::Renderer;
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::{RenderInputConsumer, pose::PoseConsumer};
use incant_render_native::{UpdateKind, interpolation::PoseReceipt};
use std::sync::Arc;

pub struct PoseTargets<P> {
    inner: P,
    mode: PresentationMode,
}
impl<P> PoseTargets<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            mode: PresentationMode::AdmitOnly,
        }
    }
}
impl<P: TargetProvider> TargetProvider for PoseTargets<P> {
    type Error = P::Error;
    type Frame = P::Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Self::Frame, Self::Error> {
        self.inner.acquire(renderer)
    }
    fn presentation_mode(&self) -> PresentationMode {
        self.mode
    }
    fn resolution_config(&self) -> Option<incant_render::resolution::ResolutionConfig> {
        self.inner.resolution_config()
    }
    fn motion_epoch(&self) -> Option<u64> {
        self.inner.motion_epoch()
    }
    #[cfg(feature = "native-metalfx")]
    fn metalfx(&self) -> Option<crate::MetalFxSetup> {
        self.inner.metalfx()
    }
}
impl<P: TargetProvider> PoseConsumer for GpuRenderConsumer<PoseTargets<P>> {
    fn admit_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        tick: u64,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        if let Some(live) = &mut self.live {
            live.targets.mode = PresentationMode::AdmitOnly;
        }
        self.accept_at_tick(input, camera, kind, ui, tick)
    }
    fn present_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        receipt: PoseReceipt,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        if let Err(error) = crate::presentation::validate_policy(
            self.live.as_ref().ok_or(crate::GpuHostError::Failed)?,
        ) {
            self.live = None;
            return Err(error);
        }
        // Canonical admission has already retained any replacement resources.
        // Present updates their numeric camera/model/light poses consistently.
        if let Some(live) = &mut self.live {
            live.targets.mode = PresentationMode::Present;
        }
        let result =
            self.accept_at_tick(input, camera, UpdateKind::Motion, ui, receipt.current_tick);
        if let Some(live) = &mut self.live {
            live.targets.mode = PresentationMode::AdmitOnly;
        }
        result
    }
}
