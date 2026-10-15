//! Four complete embedded engine-owned bloom families; never a runtime loader.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{self, Artifact, Bundle, Stage},
    },
};
fn error(reason: &'static str) -> Box<dyn std::error::Error + Send + Sync> {
    ResourceError::CookedBloom(reason).into()
}
mod prefilter {
    use super::*;
    const CONTRACT: &str = "incant-bloom-prefilter-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/bloom-prefilter-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod downsample {
    use super::*;
    const CONTRACT: &str = "incant-bloom-downsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/bloom-downsample-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod upsample {
    use super::*;
    const CONTRACT: &str = "incant-bloom-upsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/bloom-upsample-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod display {
    use super::*;
    const CONTRACT: &str = "incant-bloom-display-v1;naga=29.0.4;wgpu=29.0.4;textures0-2,uniform80@1,uniform16@3;msl=2.4,t0-t1,b0-b1;hlsl=6.0,t0-t1,b0-b1,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/bloom-display-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
const BUNDLES: [&Bundle; 4] = [
    &prefilter::BUNDLE,
    &downsample::BUNDLE,
    &upsample::BUNDLE,
    &display::BUNDLE,
];
pub(super) fn validate(backend: wgpu::Backend, features: wgpu::Features) -> Result<()> {
    native::features(backend, features).map_err(error)?;
    for bundle in BUNDLES {
        for stage in [Stage::Vertex, Stage::Fragment] {
            native::descriptor(bundle.select(backend, stage).map_err(error)?).map_err(error)?;
        }
    }
    Ok(())
}
pub(super) fn filters(device: &wgpu::Device, backend: wgpu::Backend) -> Result<[Stages; 3]> {
    Ok([
        prefilter::BUNDLE.stages(device, backend).map_err(error)?,
        downsample::BUNDLE.stages(device, backend).map_err(error)?,
        upsample::BUNDLE.stages(device, backend).map_err(error)?,
    ])
}
pub(crate) fn display(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    display::BUNDLE.stages(device, backend).map_err(error)
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn complete_embedded_bloom_selects_native_only_and_refuses_unsupported_capability() {
        for (index, bundle) in BUNDLES.iter().enumerate() {
            let source = if index == 3 {
                super::super::output_source()
            } else {
                include_str!("filter.wgsl").to_owned()
            };
            assert_eq!(bundle.source, <[u8; 32]>::from(Sha256::digest(source)));
            for backend in [
                wgpu::Backend::Metal,
                wgpu::Backend::Vulkan,
                wgpu::Backend::Dx12,
            ] {
                validate(backend, wgpu::Features::PASSTHROUGH_SHADERS).unwrap();
                for stage in [Stage::Vertex, Stage::Fragment] {
                    let descriptor =
                        native::descriptor(bundle.select(backend, stage).unwrap()).unwrap();
                    assert!(descriptor.wgsl.is_none());
                    assert_eq!(descriptor.msl.is_some(), backend == wgpu::Backend::Metal);
                    assert_eq!(descriptor.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                    assert_eq!(descriptor.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                }
            }
        }
        assert!(validate(wgpu::Backend::Gl, wgpu::Features::PASSTHROUGH_SHADERS).is_err());
        assert!(validate(wgpu::Backend::Metal, wgpu::Features::empty()).is_err());
    }
}
