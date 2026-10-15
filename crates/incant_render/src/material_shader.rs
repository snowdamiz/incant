//! Fixed material passthrough; every native binding has an engine-owned layout.
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};
include!(concat!(env!("OUT_DIR"), "/material-shaders/manifest.rs"));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: "incant-material-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7;spv=1.0,no-y-flip;hlsl=6.0,special-b5;bounds=restrict;loops=bounded",
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Stages> {
    validate_host().map_err(ResourceError::CookedMaterial)?;
    BUNDLE
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedMaterial(e).into())
}
pub(crate) fn validate_host() -> std::result::Result<(), &'static str> {
    use wgpu::{
        BindingType as T, BufferBindingType as B, SamplerBindingType as S, ShaderStages as V,
        TextureSampleType as P, TextureViewDimension as D,
    };
    crate::geometry_shader::cooked::validate_host(crate::geometry_shader::Kind::Shadow)?;
    let uniform = T::Buffer {
        ty: B::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let store = T::Buffer {
        ty: B::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let texture = |dim, sample_type| T::Texture {
        sample_type,
        view_dimension: dim,
        multisampled: false,
    };
    let entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty,
        count: None,
    };
    if crate::materials::frame_entry() != entry(0, V::VERTEX_FRAGMENT, uniform) {
        return Err("material frame layout");
    }
    let env = vec![
        entry(
            0,
            V::FRAGMENT,
            texture(D::Cube, P::Float { filterable: true }),
        ),
        entry(
            1,
            V::FRAGMENT,
            texture(D::Cube, P::Float { filterable: true }),
        ),
        entry(
            2,
            V::FRAGMENT,
            texture(D::D2, P::Float { filterable: true }),
        ),
        entry(3, V::FRAGMENT, T::Sampler(S::Filtering)),
    ];
    if crate::environment::layout_entries() != env {
        return Err("material environment layout");
    }
    let lighting = vec![
        entry(0, V::FRAGMENT, uniform),
        entry(1, V::FRAGMENT, store),
        entry(2, V::FRAGMENT, store),
        entry(3, V::FRAGMENT, store),
        entry(4, V::FRAGMENT, texture(D::D2Array, P::Depth)),
        entry(5, V::FRAGMENT, T::Sampler(S::Comparison)),
        entry(6, V::FRAGMENT, uniform),
        entry(
            7,
            V::FRAGMENT,
            T::Buffer {
                ty: B::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(16),
            },
        ),
    ];
    if crate::lighting::layout_entries(false) != lighting {
        return Err("material lighting/lengths layout");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_native_material_bundle_matches_host_and_rejects_tampering() {
        validate_host().unwrap();
        for backend in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            for stage in [Stage::Vertex, Stage::Fragment] {
                let artifact = BUNDLE.select(backend, stage).unwrap();
                let descriptor = crate::shader::native::descriptor(artifact).unwrap();
                assert_eq!(descriptor.msl.is_some(), backend == wgpu::Backend::Metal);
                assert_eq!(descriptor.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                assert_eq!(descriptor.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                let mut bad = *artifact;
                bad.hash[0] ^= 1;
                assert!(crate::shader::native::descriptor(&bad).is_err());
            }
        }
        assert!(BUNDLE.select(wgpu::Backend::Gl, Stage::Vertex).is_err());
    }
}
