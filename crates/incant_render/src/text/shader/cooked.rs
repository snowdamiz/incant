//! Private build artifacts only; project shaders cannot enter this path.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};

include!(concat!(env!("OUT_DIR"), "/text-shaders/manifest.rs"));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: "incant-text-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15i,stride32,sizes2,frame0,node1;spv=1.0,no-y-flip;hlsl=6.0,frame-b0,node-b1,texture-t0,sampler-indices-t1,special-b2;bounds=restrict;loops=bounded",
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};

pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    validate_host().map_err(ResourceError::CookedText)?;
    BUNDLE
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedText(e).into())
}

fn validate_host() -> std::result::Result<(), &'static str> {
    use super::super::{
        drawing::{self, FrameUniform},
        geometry::GlyphQuad,
    };
    use wgpu::{VertexFormat as F, VertexStepMode as S};
    let buffers = drawing::vertex_buffers();
    let b = &buffers[0];
    if b.array_stride != 32
        || b.step_mode != S::Instance
        || b.attributes
            .iter()
            .map(|a| (a.shader_location, a.offset, a.format))
            .collect::<Vec<_>>()
            != vec![(0, 0, F::Float32x4), (1, 16, F::Float32x4)]
    {
        return Err("text glyph instance layout");
    }
    if size_of::<FrameUniform>() != 80
        || size_of::<GlyphQuad>() != 32
        || std::mem::offset_of!(FrameUniform, matrix) != 0
        || std::mem::offset_of!(FrameUniform, color) != 64
        || std::mem::offset_of!(GlyphQuad, layout) != 0
        || std::mem::offset_of!(GlyphQuad, texels) != 16
    {
        return Err("text host member offsets");
    }
    // Compare the exact entries consumed by pipeline construction, including the dynamic node slot.
    for (actual, visibility, dynamic, size) in [
        (
            super::super::frame_entry(),
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            false,
            80,
        ),
        (
            super::super::node_entry(),
            wgpu::ShaderStages::VERTEX,
            true,
            64,
        ),
    ] {
        if actual
            != (wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: dynamic,
                    min_binding_size: std::num::NonZeroU64::new(size),
                },
            })
        {
            return Err("text uniform host layout");
        }
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
    if super::super::atlas_entries() != expected {
        return Err("text atlas host layout");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn embedded_text_stages_match_current_source_host_and_backends() {
        validate_host().unwrap();
        assert_eq!(
            BUNDLE.source,
            <[u8; 32]>::from(Sha256::digest(include_bytes!("../text.wgsl")))
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
