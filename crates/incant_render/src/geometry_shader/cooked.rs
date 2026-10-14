use super::Kind;
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};

mod diagnostic {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/diagnostic-shaders/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-diagnostic-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15,stride28,sizes0;spv=1.0,no-y-flip;hlsl=6.0,special-b0;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod shadow {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/shadow-shaders/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-shadow-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,sizes1;spv=1.0,no-y-flip;hlsl=6.0,camera-b0,material-b1,texture-t0,sampler-indices-t5,special-b2;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
fn bundle(kind: Kind) -> &'static Bundle {
    match kind {
        Kind::Diagnostic => &diagnostic::BUNDLE,
        Kind::Shadow => &shadow::BUNDLE,
    }
}
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    validate_host(kind).map_err(ResourceError::CookedGeometry)?;
    bundle(kind)
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedGeometry(e).into())
}

pub(crate) fn validate_host(kind: Kind) -> std::result::Result<(), &'static str> {
    use wgpu::{VertexFormat as F, VertexStepMode as S};
    let expected = match kind {
        Kind::Diagnostic => vec![(
            28,
            S::Vertex,
            vec![(0, 0, F::Float32x4), (1, 16, F::Float32x3)],
        )],
        Kind::Shadow => vec![
            (
                48,
                S::Vertex,
                vec![
                    (0, 0, F::Float32x3),
                    (1, 12, F::Float32x3),
                    (9, 24, F::Float32x2),
                    (10, 32, F::Float32x4),
                ],
            ),
            (
                112,
                S::Instance,
                (2..=8)
                    .map(|n| (n, u64::from(n - 2) * 16, F::Float32x4))
                    .collect(),
            ),
        ],
    };
    let actual = match kind {
        Kind::Diagnostic => super::diagnostic_buffers().to_vec(),
        Kind::Shadow => super::shadow_buffers().to_vec(),
    };
    if actual.len() != expected.len() {
        return Err("vertex host layout count");
    }
    for (a, (stride, step, attributes)) in actual.iter().zip(expected) {
        let got: Vec<_> = a
            .attributes
            .iter()
            .map(|x| (x.shader_location, x.offset, x.format))
            .collect();
        if a.array_stride != stride || a.step_mode != step || got != attributes {
            return Err("vertex host layout mismatch");
        }
    }
    if matches!(kind, Kind::Shadow) {
        if crate::shadows::camera_layout_entry()
            != (wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
        {
            return Err("camera host layout mismatch");
        }
        let entries = crate::materials::layout_entries();
        if entries.len() != 11 {
            return Err("material layout count");
        }
        for (i, e) in entries.iter().enumerate() {
            let ty = match i {
                0 => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                n if n % 2 == 1 => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                _ => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            };
            if e.binding != i as u32
                || e.visibility != wgpu::ShaderStages::FRAGMENT
                || e.count.is_some()
                || e.ty != ty
            {
                return Err("material host layout mismatch");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::native::descriptor;
    use sha2::{Digest, Sha256};
    #[test]
    fn embedded_geometry_families_match_sources_host_layouts_and_native_descriptors() {
        for (kind, source) in [
            (Kind::Diagnostic, include_str!("../fixture.wgsl")),
            (Kind::Shadow, include_str!("../shadows/caster.wgsl")),
        ] {
            validate_host(kind).unwrap();
            let b = bundle(kind);
            assert_eq!(
                b.source,
                <[u8; 32]>::from(Sha256::digest(source.as_bytes()))
            );
            for backend in [
                wgpu::Backend::Metal,
                wgpu::Backend::Vulkan,
                wgpu::Backend::Dx12,
            ] {
                for stage in [Stage::Vertex, Stage::Fragment] {
                    let d = descriptor(b.select(backend, stage).unwrap()).unwrap();
                    assert!(d.wgsl.is_none());
                    assert_eq!(d.msl.is_some(), backend == wgpu::Backend::Metal);
                    assert_eq!(d.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                    assert_eq!(d.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                }
            }
        }
    }
    #[test]
    fn family_mismatch_is_refused_before_native_creation() {
        let b = Bundle {
            expected: shadow::BUNDLE.expected,
            ..diagnostic::BUNDLE
        };
        assert!(b.select(wgpu::Backend::Metal, Stage::Vertex).is_err());
    }
}
