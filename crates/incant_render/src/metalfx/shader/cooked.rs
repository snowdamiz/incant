use super::Kind;
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};
mod pack {
    use super::*;
    include!(concat!(
        env!("OUT_DIR"),
        "/metalfx-pack-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-metalfx-pack-v1;naga=29.0.4;wgpu=29.0.4;frame96-f@0,textures2@1-2,depth@3;mrt=rgba16f-r32f-rg16f-r8;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod normalize {
    use super::*;
    include!(concat!(
        env!("OUT_DIR"),
        "/metalfx-normalize-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-metalfx-normalize-v1;naga=29.0.4;wgpu=29.0.4;frame96-f@0,textures2@1-2,depth@3;mrt=rgba16f;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
fn bundle(kind: Kind) -> &'static Bundle {
    match kind {
        Kind::Pack => &pack::BUNDLE,
        Kind::Normalize => &normalize::BUNDLE,
    }
}
fn validate_host(kind: Kind) -> std::result::Result<(), &'static str> {
    let expected = std::array::from_fn::<_, 4, _>(|binding| wgpu::BindGroupLayoutEntry {
        binding: binding as u32,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: if binding == 0 {
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(96),
            }
        } else {
            wgpu::BindingType::Texture {
                sample_type: if binding == 3 {
                    wgpu::TextureSampleType::Depth
                } else {
                    wgpu::TextureSampleType::Float { filterable: false }
                },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            }
        },
        count: None,
    });
    let formats = match kind {
        Kind::Pack => vec![
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::R8Unorm,
        ],
        Kind::Normalize => vec![wgpu::TextureFormat::Rgba16Float],
    };
    if super::super::system::layout_entries() != expected
        || super::super::system::formats(kind) != formats
    {
        return Err("MetalFX boundary frame/texture/MRT host ABI");
    }
    Ok(())
}
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    validate_host(kind).map_err(ResourceError::CookedMetalFxBoundary)?;
    bundle(kind)
        .stages(device, backend)
        .map_err(|reason| ResourceError::CookedMetalFxBoundary(reason).into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn both_boundary_programs_match_host_source_and_all_backend_stages() {
        for kind in [Kind::Pack, Kind::Normalize] {
            validate_host(kind).unwrap();
            let b = bundle(kind);
            assert_eq!(
                b.source,
                <[u8; 32]>::from(Sha256::digest(super::super::super::source::source(
                    kind == Kind::Normalize
                )))
            );
            for backend in [
                wgpu::Backend::Metal,
                wgpu::Backend::Vulkan,
                wgpu::Backend::Dx12,
            ] {
                for stage in [Stage::Vertex, Stage::Fragment] {
                    let artifact = b.select(backend, stage).unwrap();
                    let descriptor = crate::shader::native::descriptor(artifact).unwrap();
                    assert!(descriptor.wgsl.is_none());
                    assert_eq!(descriptor.msl.is_some(), backend == wgpu::Backend::Metal);
                    assert_eq!(descriptor.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                    assert_eq!(descriptor.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                    let mut invalid = *artifact;
                    invalid.hash[0] ^= 1;
                    assert!(crate::shader::native::descriptor(&invalid).is_err());
                }
            }
            for backend in [wgpu::Backend::Gl, wgpu::Backend::BrowserWebGpu] {
                assert!(b.select(backend, Stage::Vertex).is_err());
            }
        }
        let wrong = Bundle {
            expected: pack::BUNDLE.expected,
            ..normalize::BUNDLE
        };
        assert!(wrong.select(wgpu::Backend::Metal, Stage::Fragment).is_err());
    }
}
