//! Opt-in recorded ModelMaterialV1 preparation. Fixed-family eager preparation
//! remains the default. Native persistence occurs only at explicit cold seams.
#[cfg(not(target_arch = "wasm32"))]
mod system;
#[cfg(not(target_arch = "wasm32"))]
mod worker;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use system::System;
#[cfg(not(target_arch = "wasm32"))]
pub use system::{MaterialKeyReport, MaterialPrewarmReport};
#[cfg(not(target_arch = "wasm32"))]
pub use worker::{CompileFailure, FailureKind, WorkerError};
#[cfg(all(feature = "cooked-materials", not(target_arch = "wasm32")))]
mod cooked;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod gpu_tests;
#[cfg(any(
    test,
    all(not(feature = "cooked-materials"), not(target_arch = "wasm32"))
))]
mod source;
#[derive(Debug, thiserror::Error)]
pub enum MaterialPrewarmError {
    #[error("recorded material compilation requires the native renderer")]
    Unsupported,
    #[error("recorded material program/sample profile does not match this renderer")]
    Profile,
    #[error("ModelMaterialV1 recording does not support optional renderer effects")]
    Effect,
    #[error("fixed pending material source contract changed")]
    Source,
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn fallback_shader(
    device: &wgpu::Device,
    backend: wgpu::Backend,
) -> crate::Result<crate::shader::Stages> {
    #[cfg(feature = "cooked-materials")]
    return cooked::stages(device, backend);
    #[cfg(not(feature = "cooked-materials"))]
    {
        let _ = backend;
        let source = source::fallback().ok_or(MaterialPrewarmError::Source)?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pending model material"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        Ok(crate::shader::Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        })
    }
}
impl crate::Renderer {
    pub fn material_program() -> incant_render_data::prewarm::MaterialProgram {
        if cfg!(all(
            feature = "cooked-materials",
            not(target_arch = "wasm32")
        )) {
            incant_render_data::prewarm::MaterialProgram::Cooked
        } else {
            incant_render_data::prewarm::MaterialProgram::Wgsl
        }
    }
    /// Explicit cold constructor: only admitted observed real keys are prewarmed;
    /// every supported key has a coverage/depth-compatible pending shader.
    pub async fn new_with_recorded_materials(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        admitted: incant_render_data::prewarm::AdmittedMaterials,
    ) -> crate::Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (instance, surface, admitted);
            Err(MaterialPrewarmError::Unsupported.into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let samples = match admitted.profile().samples {
                incant_render_data::material_keys::MaterialSamples::One => crate::SceneMsaa::One,
                incant_render_data::material_keys::MaterialSamples::Four => crate::SceneMsaa::Four,
            };
            Self::new_configured(
                instance,
                surface,
                false,
                samples,
                Some(admitted),
                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                None,
                #[cfg(all(
                    feature = "metal-binary-archives",
                    feature = "native-compilation",
                    target_os = "macos"
                ))]
                wgpu_hal::metal::binary_archive::Mode::Capture,
            )
            .await
        }
    }
    /// Trusted diagnostic companion: startup stays Catalog; each actual worker
    /// compile owns a Background scope whose segment is the finite key plus one.
    /// Do not simultaneously open a caller-thread scope on this same observer.
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    pub async fn new_recorded_materials_with_compilation_capture(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        admitted: incant_render_data::prewarm::AdmittedMaterials,
        capture: &mut crate::compilation::Capture,
    ) -> crate::Result<Self> {
        let samples = match admitted.profile().samples {
            incant_render_data::material_keys::MaterialSamples::One => crate::SceneMsaa::One,
            incant_render_data::material_keys::MaterialSamples::Four => crate::SceneMsaa::Four,
        };
        Self::new_configured(
            instance,
            surface,
            false,
            samples,
            Some(admitted),
            Some(capture),
            #[cfg(feature = "metal-binary-archives")]
            wgpu_hal::metal::binary_archive::Mode::Capture,
        )
        .await
    }
    /// Data-only snapshot; no native cache query, driver call or persistence.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn material_prewarm_report(&self) -> Option<MaterialPrewarmReport> {
        self.materials.recorded.as_ref().map(System::report)
    }
    /// Cancel queued keys and wait a bounded cold interval for the one in-flight
    /// compile. Timeout keeps its quarantine permit; future recorded renderers
    /// remain refused until its compilation and captured-resource teardown finish.
    /// Successful close also joins the thread. Closed renderers cannot draw.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn close_recorded_materials(&mut self, timeout: std::time::Duration) -> crate::Result<()> {
        if let Some(recorded) = &mut self.materials.recorded {
            recorded.close(timeout)?;
        }
        // No worker can now mutate native archives. Persist only at this explicit
        // cold boundary; frame selection and Drop never read/write the cache.
        self.complete_pipeline_preparation(Ok(()))
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn fallback_keeps_vertex_interface_and_exact_base_alpha_rules() {
        let source = super::source::fallback().unwrap();
        let original = include_str!("../model_material.wgsl");
        assert_eq!(
            source
                .split_once("@fragment fn fragment")
                .unwrap()
                .0
                .trim_end(),
            original
                .split_once("@fragment fn fragment")
                .unwrap()
                .0
                .trim_end()
        );
        for line in [
            "material.base*textureSample(base_map,base_sampler,in.uv)",
            "material.flags.x==1u && base.a<material.factors.w",
            "select(1.0,base.a,material.flags.x==2u)",
        ] {
            assert!(source.contains(line) && original.contains(line));
        }
    }
}
