//! Fixed output ABI selects only its build-generated embedded artifact family.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{self, Artifact, Bundle, Stage},
    },
};
#[cfg(test)]
use native::descriptor;
#[cfg(test)]
use sha2::{Digest, Sha256};
const CONTRACT: &str = "incant-output-v1;naga=29.0.4;wgpu=29.0.4;hal=e99f5305ded96ff7006f0714d043a7f735bd45c2;msl=2.4;spv=1.0,no-y-flip;hlsl=6.0;bounds=restrict;loops=bounded;layout=texture0,uniform80";
include!(concat!(env!("OUT_DIR"), "/output-shaders/manifest.rs"));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: CONTRACT,
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};
pub(super) fn features(
    backend: wgpu::Backend,
    available: wgpu::Features,
) -> Result<wgpu::Features> {
    native::features(backend, available).map_err(|e| ResourceError::CookedOutput(e).into())
}
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    BUNDLE
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedOutput(e).into())
}
#[cfg(test)]
fn select(backend: wgpu::Backend, stage: Stage) -> Result<&'static Artifact> {
    BUNDLE
        .select(backend, stage)
        .map_err(|e| ResourceError::CookedOutput(e).into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_embedded_stages_select_the_native_descriptor_without_wgsl() {
        assert_eq!(MANIFEST_CONTRACT, CONTRACT);
        let source = [
            include_str!("../output.wgsl"),
            include_str!("../tone_map.wgsl"),
        ]
        .join("\n");
        assert_eq!(SOURCE_HASH, <[u8; 32]>::from(Sha256::digest(source)));
        for backend in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            for stage in [Stage::Vertex, Stage::Fragment] {
                let a = select(backend, stage).unwrap();
                let d = descriptor(a).unwrap();
                assert!(d.wgsl.is_none());
                assert_eq!(d.msl.is_some(), backend == wgpu::Backend::Metal);
                assert_eq!(d.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                assert_eq!(d.hlsl.is_some(), backend == wgpu::Backend::Dx12);
            }
        }
    }
    #[test]
    fn corrupted_and_unsupported_inputs_refuse_before_driver_calls() {
        let mut artifact = *select(wgpu::Backend::Metal, Stage::Fragment).unwrap();
        artifact.hash[0] ^= 1;
        assert!(descriptor(&artifact).is_err());
        artifact = *select(wgpu::Backend::Vulkan, Stage::Vertex).unwrap();
        artifact.bytes = &[3, 2, 35, 7];
        artifact.hash = Sha256::digest(artifact.bytes).into();
        assert!(descriptor(&artifact).is_err());
        artifact = *select(wgpu::Backend::Vulkan, Stage::Vertex).unwrap();
        let mut wrong_version = artifact.bytes.to_vec();
        wrong_version[4..8].copy_from_slice(&0x00020000u32.to_le_bytes());
        artifact.bytes = Box::leak(wrong_version.into_boxed_slice());
        artifact.hash = Sha256::digest(artifact.bytes).into();
        assert!(descriptor(&artifact).is_err());
        artifact = *select(wgpu::Backend::Metal, Stage::Fragment).unwrap();
        artifact.entry = "invalid entry";
        assert!(descriptor(&artifact).is_err());
        assert!(select(wgpu::Backend::Gl, Stage::Vertex).is_err());
        assert!(features(wgpu::Backend::Metal, wgpu::Features::empty()).is_err());
        assert!(features(wgpu::Backend::Gl, wgpu::Features::PASSTHROUGH_SHADERS).is_err());
    }
}
