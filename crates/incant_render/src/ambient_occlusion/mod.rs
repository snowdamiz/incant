//! Opt-in screen-space diffuse ambient accessibility. No temporal history.
#[cfg(test)]
mod device_tests;
mod draw;
#[cfg(test)]
mod filter_device_tests;
mod math;
mod pipelines;
mod raster;
#[cfg(feature = "ssao-stage-readback")]
mod readback;
#[cfg(feature = "ssao-stage-readback")]
pub use readback::{AmbientOcclusionCaptureError, AmbientOcclusionStages};
#[cfg(test)]
mod quality_geometry;
#[cfg(test)]
mod quality_metrics;
#[cfg(test)]
mod quality_oracle;
#[cfg(test)]
mod quality_sphere;
#[cfg(test)]
mod quality_stripes;
#[cfg(test)]
mod quality_tests;
mod targets;
#[cfg(test)]
mod tests;

pub(crate) use draw::Prepared;
pub use math::{AmbientOcclusion, AmbientOcclusionError};
pub(crate) use pipelines::System;

impl crate::Renderer {
    /// Prepare the complete SSAO pipeline catalog before rendering. Default is
    /// disabled. Only diffuse environment light on opaque/MASK models is affected;
    /// direct light, specular IBL, emissive, BLEND, sprites, text and UI are unchanged.
    /// This portable WGSL path does not yet have a cooked-material variant.
    /// Failure leaves the previous setting/resources intact.
    pub async fn enable_ambient_occlusion(
        &mut self,
        settings: AmbientOcclusion,
    ) -> crate::Result<()> {
        settings.validate()?;
        if cfg!(all(
            feature = "cooked-materials",
            not(target_arch = "wasm32")
        )) {
            return Err(AmbientOcclusionError::CookedMaterials.into());
        }
        self.check_device()?;
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = System::new(self, settings);
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.ambient_occlusion = Some(candidate?);
        Ok(())
    }

    /// Future draws use the original material path; encoded frames retain their
    /// own resources until completion, as with ordinary frame attachments.
    pub fn disable_ambient_occlusion(&mut self) {
        self.ambient_occlusion = None;
    }
}
