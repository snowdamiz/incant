//! Private build artifacts only; project shaders cannot enter this path.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};

include!(concat!(env!("OUT_DIR"), "/ui-shaders/manifest.rs"));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: "incant-ui-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15i,stride64,sizes1,frame0;spv=1.0,no-y-flip;hlsl=6.0,frame-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded",
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};

pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    validate_host().map_err(ResourceError::CookedUi)?;
    BUNDLE
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedUi(e).into())
}

fn validate_host() -> std::result::Result<(), &'static str> {
    use wgpu::{VertexFormat as F, VertexStepMode as S};
    let buffers = super::super::vertex_buffers();
    let expected = [(
        64,
        vec![
            (0, 0, F::Float32x4),
            (1, 16, F::Float32x4),
            (2, 32, F::Float32x4),
            (3, 48, F::Float32x2),
            (4, 56, F::Unorm8x4),
            (5, 60, F::Uint32),
        ],
    )];
    for (buffer, (stride, attributes)) in buffers.iter().zip(expected) {
        let actual: Vec<_> = buffer
            .attributes
            .iter()
            .map(|a| (a.shader_location, a.offset, a.format))
            .collect();
        if buffer.array_stride != stride || buffer.step_mode != S::Instance || actual != attributes
        {
            return Err("UI vertex host layout");
        }
    }
    // Check Rust field offsets as well as the descriptors consumed by HAL.
    use super::super::{FrameUniform, geometry::UiQuad};
    if size_of::<FrameUniform>() != 48
        || [
            std::mem::offset_of!(FrameUniform, region),
            std::mem::offset_of!(FrameUniform, scale),
            std::mem::offset_of!(FrameUniform, surface),
            std::mem::offset_of!(FrameUniform, encode),
        ] != [0, 16, 24, 32]
        || [
            std::mem::offset_of!(UiQuad, rect),
            std::mem::offset_of!(UiQuad, clip),
            std::mem::offset_of!(UiQuad, extra),
            std::mem::offset_of!(UiQuad, pen),
            std::mem::offset_of!(UiQuad, color),
            std::mem::offset_of!(UiQuad, kind),
        ] != [0, 16, 32, 48, 56, 60]
    {
        return Err("UI uniform/instance host offsets");
    }
    let frame = super::super::frame_entry();
    if frame
        != (wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(48),
            },
        })
    {
        return Err("UI frame host layout");
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
        return Err("UI texture host layout");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn embedded_ui_stages_match_current_source_host_and_backends() {
        validate_host().unwrap();
        assert_eq!(
            BUNDLE.source,
            <[u8; 32]>::from(Sha256::digest(include_bytes!("../ui.wgsl")))
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
