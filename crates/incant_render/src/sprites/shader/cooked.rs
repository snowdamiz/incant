//! Private build artifacts only; project shaders cannot enter this path.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};

include!(concat!(env!("OUT_DIR"), "/sprite-shaders/manifest.rs"));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: "incant-sprite-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14v,stride12-52,sizes1;spv=1.0,no-y-flip;hlsl=6.0,camera-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded",
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};

pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    validate_host().map_err(ResourceError::CookedSprite)?;
    BUNDLE
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedSprite(e).into())
}

fn validate_host() -> std::result::Result<(), &'static str> {
    use wgpu::{VertexFormat as F, VertexStepMode as S};
    let buffers = super::super::vertex_buffers();
    let expected = [
        (12, vec![(0, 0, F::Float32x3)]),
        (
            52,
            vec![
                (1, 0, F::Float32x2),
                (2, 8, F::Float32x4),
                (3, 24, F::Float32x4),
                (4, 40, F::Float32x2),
                (5, 48, F::Uint32),
            ],
        ),
    ];
    for (buffer, (stride, attributes)) in buffers.iter().zip(expected) {
        let actual: Vec<_> = buffer
            .attributes
            .iter()
            .map(|a| (a.shader_location, a.offset, a.format))
            .collect();
        if buffer.array_stride != stride || buffer.step_mode != S::Vertex || actual != attributes {
            return Err("sprite vertex host layout");
        }
    }
    let camera = super::super::camera_entry();
    if camera
        != (wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        })
    {
        return Err("sprite camera host layout");
    }
    let expected = [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            count: None,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            count: None,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        },
    ];
    if super::super::texture_entries() != expected {
        return Err("sprite texture host layout");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn embedded_sprite_stages_match_current_source_host_and_backends() {
        validate_host().unwrap();
        assert_eq!(
            BUNDLE.source,
            <[u8; 32]>::from(Sha256::digest(include_bytes!("../sprite.wgsl")))
        );
        for backend in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            for stage in [Stage::Vertex, Stage::Fragment] {
                let descriptor =
                    crate::shader::native::descriptor(BUNDLE.select(backend, stage).unwrap())
                        .unwrap();
                assert!(descriptor.wgsl.is_none());
                assert_eq!(descriptor.msl.is_some(), backend == wgpu::Backend::Metal);
                assert_eq!(descriptor.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                assert_eq!(descriptor.hlsl.is_some(), backend == wgpu::Backend::Dx12);
            }
        }
    }
}
