//! Private trusted-builtin boundary. Never accept project artifacts here.
use crate::{ResourceError, Result};
use sha2::{Digest, Sha256};
use std::borrow::Cow;

const CONTRACT: &str = "incant-output-v1;naga=29.0.4;wgpu=29.0.4;hal=e99f5305ded96ff7006f0714d043a7f735bd45c2;msl=2.4;spv=1.0,no-y-flip;hlsl=6.0;bounds=restrict;loops=bounded;layout=texture0,uniform80";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Vertex,
    Fragment,
}
#[derive(Clone, Copy)]
struct Artifact {
    backend: wgpu::Backend,
    stage: Stage,
    entry: &'static str,
    bytes: &'static [u8],
    hash: [u8; 32],
}
include!(concat!(env!("OUT_DIR"), "/output-shaders/manifest.rs"));

pub(super) fn features(
    backend: wgpu::Backend,
    available: wgpu::Features,
) -> Result<wgpu::Features> {
    if !matches!(
        backend,
        wgpu::Backend::Metal | wgpu::Backend::Vulkan | wgpu::Backend::Dx12
    ) {
        return Err(ResourceError::CookedOutput("unsupported native backend").into());
    }
    if !available.contains(wgpu::Features::PASSTHROUGH_SHADERS) {
        return Err(ResourceError::CookedOutput("passthrough shaders unavailable").into());
    }
    Ok(wgpu::Features::PASSTHROUGH_SHADERS)
}

pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<super::Stages> {
    features(backend, device.features())?;
    let vertex = select(backend, Stage::Vertex)?;
    let fragment = select(backend, Stage::Fragment)?;
    // Validate both artifacts before either driver call. This manifest and its
    // payloads are generated from the engine-owned, contract-checked WGSL by the
    // pinned build dependency. Hash checks detect damage; they do not establish
    // trust in external shader bytes. No such bytes can reach this function.
    let vertex_desc = descriptor(vertex)?;
    let fragment_desc = descriptor(fragment)?;
    // SAFETY: our cook validates Naga IR, enforces the exact output ABI and emits
    // bounds checks/loop bounds. Its fixed source has no storage, workgroup,
    // vertex-buffer or sampler side metadata. layout.rs and the fixed draw match
    // the pinned hal mapping/coordinate contract. Only these embedded artifacts
    // reach this private call; device support was checked above.
    let (vertex_module, fragment_module) = unsafe {
        (
            device.create_shader_module_passthrough(vertex_desc),
            device.create_shader_module_passthrough(fragment_desc),
        )
    };
    Ok(super::Stages {
        vertex: vertex_module,
        fragment: fragment_module,
        vertex_entry: vertex.entry,
        fragment_entry: fragment.entry,
    })
}
fn select(backend: wgpu::Backend, stage: Stage) -> Result<&'static Artifact> {
    if MANIFEST_CONTRACT != CONTRACT || SOURCE_HASH == [0; 32] || PRODUCER_HASH == [0; 32] {
        return Err(ResourceError::CookedOutput("manifest contract mismatch").into());
    }
    let mut matches = ARTIFACTS
        .iter()
        .filter(|a| a.backend == backend && a.stage == stage);
    let artifact = matches
        .next()
        .ok_or(ResourceError::CookedOutput("missing stage"))?;
    if matches.next().is_some() {
        return Err(ResourceError::CookedOutput("duplicate stage").into());
    }
    Ok(artifact)
}
fn descriptor(a: &Artifact) -> Result<wgpu::ShaderModuleDescriptorPassthrough<'static>> {
    if a.bytes.is_empty()
        || a.bytes.len() > 1024 * 1024
        || <[u8; 32]>::from(Sha256::digest(a.bytes)) != a.hash
        || a.entry.is_empty()
        || !a
            .entry
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(ResourceError::CookedOutput("invalid artifact").into());
    }
    let mut desc = wgpu::ShaderModuleDescriptorPassthrough {
        label: Some("Cooked HDR display transform"),
        ..Default::default()
    };
    match a.backend {
        wgpu::Backend::Metal | wgpu::Backend::Dx12 => {
            let text = std::str::from_utf8(a.bytes)
                .map_err(|_| ResourceError::CookedOutput("invalid shader text"))?;
            if a.backend == wgpu::Backend::Metal {
                desc.msl = Some(Cow::Borrowed(text));
            } else {
                desc.hlsl = Some(Cow::Borrowed(text));
            }
        }
        wgpu::Backend::Vulkan => {
            if a.bytes.len() < 20 || !a.bytes.len().is_multiple_of(4) {
                return Err(ResourceError::CookedOutput("invalid SPIR-V length").into());
            }
            let words: Vec<u32> = a
                .bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect();
            if words[0] != 0x07230203 || words[1] != 0x00010000 || words[4] != 0 {
                return Err(ResourceError::CookedOutput("invalid SPIR-V header").into());
            }
            desc.spirv = Some(Cow::Owned(words));
        }
        _ => return Err(ResourceError::CookedOutput("unsupported artifact backend").into()),
    }
    Ok(desc)
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
