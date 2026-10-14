//! Trusted host-only vendor preference; never serialized in project/game data.
use crate::{GpuRenderSettings, MotionReset, MotionTargets, TargetProvider};
use incant_render::{
    Renderer,
    metalfx::{Config, Preference},
    resolution::ResolutionConfig,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalFxSetup {
    pub preference: Preference,
    pub config: Config,
    pub output_extent: [u32; 2],
}
pub struct MetalFxTargets<P> {
    inner: P,
    setup: MetalFxSetup,
}
pub type MetalFxSettings<P> = GpuRenderSettings<MetalFxTargets<MotionTargets<P>>>;
impl<P: TargetProvider> GpuRenderSettings<P> {
    /// Renderer::enable_metalfx prepares pipelines before the host is constructed.
    /// Returns the same explicit camera-cut control as the ordinary motion owner.
    pub fn with_metalfx(
        self,
        preference: Preference,
        config: Config,
        output_extent: [u32; 2],
    ) -> Result<(MetalFxSettings<P>, MotionReset), incant_render::metalfx::MetalFxError> {
        config.validate()?;
        let (settings, reset) = self.with_motion_history();
        Ok((
            GpuRenderSettings {
                renderer: settings.renderer,
                targets: MetalFxTargets {
                    inner: settings.targets,
                    setup: MetalFxSetup {
                        preference,
                        config,
                        output_extent,
                    },
                },
            },
            reset,
        ))
    }
}
impl<P: TargetProvider> TargetProvider for MetalFxTargets<P> {
    type Error = P::Error;
    type Frame = P::Frame;
    fn acquire(&mut self, r: &Renderer) -> Result<Self::Frame, Self::Error> {
        self.inner.acquire(r)
    }
    fn resolution_config(&self) -> Option<ResolutionConfig> {
        self.inner.resolution_config()
    }
    fn motion_epoch(&self) -> Option<u64> {
        self.inner.motion_epoch()
    }
    fn metalfx(&self) -> Option<MetalFxSetup> {
        Some(self.setup)
    }
}

impl<P: TargetProvider> crate::GpuRenderConsumer<P> {
    /// Explicit resize preparation; call outside presentation. No target is
    /// acquired/submitted, and older staged replacements become stale.
    pub fn prepare_metalfx_extent(
        &mut self,
        extent: [u32; 2],
    ) -> Result<(), crate::GpuHostError<P::Error>> {
        let live = self.live.as_mut().ok_or(crate::GpuHostError::Failed)?;
        let setup = live.metalfx.as_mut().ok_or_else(|| {
            crate::GpuHostError::Render(Box::new(incant_render::metalfx::MetalFxError::Disabled))
        })?;
        live.scene
            .prepare_metalfx_extent(&self.renderer, extent)
            .map_err(crate::GpuHostError::Render)?;
        setup.output_extent = extent;
        Ok(())
    }
}
