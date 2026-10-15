//! All five embedded SSAO families admitted before any shader/pipeline allocation.
use super::shader::Kind;
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{self, Artifact, Bundle, Stage},
    },
};
fn error(reason: &'static str) -> Box<dyn std::error::Error + Send + Sync> {
    ResourceError::CookedAmbientOcclusion(reason).into()
}
mod prepass {
    use super::*;
    const CONTRACT: &str = "incant-ssao-prepass-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1;hlsl=6.0,frame-b0,material-b1,base-t0,sampler-indices-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/ssao-prepass-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
    pub(super) const EXPECTED_SOURCE: [u8; 32] = [
        103, 182, 72, 163, 165, 188, 39, 192, 11, 26, 140, 32, 77, 44, 29, 64, 113, 215, 49, 156,
        165, 41, 5, 102, 197, 188, 106, 48, 248, 21, 201, 186,
    ];
}
mod forward {
    use super::*;
    const CONTRACT: &str = "incant-ssao-forward-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7,ao-t0,material-t1;hlsl=6.0,ao-t0,material-t1,sampler-indices-t6-t10-t15,special-b5;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/ssao-forward-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
    pub(super) const EXPECTED_SOURCE: [u8; 32] = [
        220, 98, 7, 41, 240, 157, 87, 127, 213, 109, 7, 90, 120, 0, 91, 46, 94, 23, 234, 145, 88,
        44, 195, 4, 216, 76, 175, 167, 170, 108, 249, 199,
    ];
}
mod sample {
    use super::*;
    const CONTRACT: &str = "incant-ssao-sample-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2;msl=2.4,b0,t0-t1;hlsl=6.0,b0,t0-t1,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(env!("OUT_DIR"), "/ssao-sample-shaders/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
    pub(super) const EXPECTED_SOURCE: [u8; 32] = [
        74, 170, 12, 192, 237, 55, 85, 85, 251, 0, 105, 169, 116, 199, 157, 115, 96, 51, 127, 62,
        235, 222, 190, 228, 58, 135, 222, 88, 190, 148, 198, 182,
    ];
}
mod horizontal {
    use super::*;
    const CONTRACT: &str = "incant-ssao-horizontal-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/ssao-horizontal-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
    pub(super) const EXPECTED_SOURCE: [u8; 32] = [
        105, 128, 82, 87, 70, 189, 199, 30, 123, 173, 49, 56, 120, 110, 41, 180, 12, 76, 3, 222, 0,
        250, 69, 140, 128, 154, 145, 187, 41, 200, 78, 208,
    ];
}
mod vertical {
    use super::*;
    const CONTRACT: &str = "incant-ssao-vertical-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
    include!(concat!(
        env!("OUT_DIR"),
        "/ssao-vertical-shaders/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: CONTRACT,
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
    pub(super) const EXPECTED_SOURCE: [u8; 32] = [
        105, 128, 82, 87, 70, 189, 199, 30, 123, 173, 49, 56, 120, 110, 41, 180, 12, 76, 3, 222, 0,
        250, 69, 140, 128, 154, 145, 187, 41, 200, 78, 208,
    ];
}
fn bundle(kind: Kind) -> (&'static Bundle, [u8; 32]) {
    match kind {
        Kind::Prepass => (&prepass::BUNDLE, prepass::EXPECTED_SOURCE),
        Kind::Forward => (&forward::BUNDLE, forward::EXPECTED_SOURCE),
        Kind::Sample => (&sample::BUNDLE, sample::EXPECTED_SOURCE),
        Kind::Horizontal => (&horizontal::BUNDLE, horizontal::EXPECTED_SOURCE),
        Kind::Vertical => (&vertical::BUNDLE, vertical::EXPECTED_SOURCE),
    }
}
pub(super) fn validate(backend: wgpu::Backend, features: wgpu::Features) -> Result<()> {
    validate_host().map_err(error)?;
    native::features(backend, features).map_err(error)?;
    for kind in [
        Kind::Prepass,
        Kind::Forward,
        Kind::Sample,
        Kind::Horizontal,
        Kind::Vertical,
    ] {
        let (b, expected) = bundle(kind);
        if b.source != expected {
            return Err(error("SSAO kind/source identity"));
        }
        for stage in [Stage::Vertex, Stage::Fragment] {
            native::descriptor(b.select(backend, stage).map_err(error)?).map_err(error)?;
        }
    }
    Ok(())
}
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    let (b, expected) = bundle(kind);
    if b.source != expected {
        return Err(error("SSAO kind/source identity"));
    }
    b.stages(device, backend).map_err(error)
}
fn validate_host() -> std::result::Result<(), &'static str> {
    crate::material_pipeline::validate_cooked_host()?;
    use super::math::Uniform;
    if size_of::<Uniform>() != 736
        || std::mem::offset_of!(Uniform, projection) != 0
        || std::mem::offset_of!(Uniform, inverse) != 64
        || std::mem::offset_of!(Uniform, view) != 128
        || std::mem::offset_of!(Uniform, viewport) != 192
        || std::mem::offset_of!(Uniform, parameters) != 208
        || std::mem::offset_of!(Uniform, directions) != 224
        || std::mem::offset_of!(Uniform, rotations) != 480
    {
        return Err("SSAO host uniform layout");
    }
    if super::pipelines::uniform_entry()
        != (wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(736),
            },
        })
    {
        return Err("SSAO sampling uniform binding");
    }
    for (binding, depth) in [(1, true), (1, false), (2, false), (3, false)] {
        if super::pipelines::texture(binding, depth)
            != (wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
                ty: wgpu::BindingType::Texture {
                    sample_type: if depth {
                        wgpu::TextureSampleType::Depth
                    } else {
                        wgpu::TextureSampleType::Float { filterable: false }
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            })
        {
            return Err("SSAO texture host binding");
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn complete_ssao_native_descriptors_match_each_source_and_host() {
        for backend in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            validate(backend, wgpu::Features::PASSTHROUGH_SHADERS).unwrap();
            for kind in [
                Kind::Prepass,
                Kind::Forward,
                Kind::Sample,
                Kind::Horizontal,
                Kind::Vertical,
            ] {
                let (b, expected) = bundle(kind);
                assert_eq!(
                    expected,
                    <[u8; 32]>::from(Sha256::digest(super::super::shader::source(kind).unwrap()))
                );
                for stage in [Stage::Vertex, Stage::Fragment] {
                    let artifact = b.select(backend, stage).unwrap();
                    let descriptor = native::descriptor(artifact).unwrap();
                    assert!(descriptor.wgsl.is_none());
                    assert_eq!(descriptor.msl.is_some(), backend == wgpu::Backend::Metal);
                    assert_eq!(descriptor.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                    assert_eq!(descriptor.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                    let mut bad = *artifact;
                    bad.hash[0] ^= 1;
                    assert!(native::descriptor(&bad).is_err());
                }
            }
        }
        assert!(validate(wgpu::Backend::Gl, wgpu::Features::PASSTHROUGH_SHADERS).is_err());
        assert!(validate(wgpu::Backend::Metal, wgpu::Features::empty()).is_err());
    }
}
