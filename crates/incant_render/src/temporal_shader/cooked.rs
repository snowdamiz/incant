use super::Kind;
use crate::{
    ResourceError, Result,
    shader::{
        Stages,
        native::{Artifact, Bundle, Stage},
    },
};
mod host;
mod motion {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/temporal-motion/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-temporal-motion-v1;naga=29.0.4;wgpu=29.0.4;frame144-vf,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod motion_owner {
    use super::*;
    include!(concat!(
        env!("OUT_DIR"),
        "/temporal-motion-owner/manifest.rs"
    ));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-temporal-motion-owner-v1;naga=29.0.4;wgpu=29.0.4;frame144-vf,material64-f,5textures-samplers;mrt=rgba32f-r32u;owner=uint-loc12-offset68-flat3;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod resolve {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/temporal-resolve/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-temporal-resolve-v1;naga=29.0.4;wgpu=29.0.4;frame64-f,textures4,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod copy {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/temporal-copy/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-temporal-copy-v2;naga=29.0.4;wgpu=29.0.4;frame64-f@1,textures1@0,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
mod depth {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/temporal-depth/manifest.rs"));
    pub(super) const BUNDLE: Bundle = Bundle {
        contract: MANIFEST_CONTRACT,
        expected: "incant-temporal-depth-v1;naga=29.0.4;wgpu=29.0.4;frame144-v,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
        source: SOURCE_HASH,
        producer: PRODUCER_HASH,
        artifacts: &ARTIFACTS,
    };
}
fn bundle(kind: Kind) -> &'static Bundle {
    match kind {
        Kind::Depth => &depth::BUNDLE,
        Kind::Motion => &motion::BUNDLE,
        Kind::MotionOwner => &motion_owner::BUNDLE,
        Kind::Resolve => &resolve::BUNDLE,
        Kind::Copy => &copy::BUNDLE,
    }
}
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    host::validate(kind).map_err(ResourceError::CookedTemporal)?;
    bundle(kind)
        .stages(device, backend)
        .map_err(|e| ResourceError::CookedTemporal(e).into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn every_fixed_temporal_program_has_exact_host_and_all_native_stages() {
        for kind in [
            Kind::Depth,
            Kind::Motion,
            Kind::MotionOwner,
            Kind::Resolve,
            Kind::Copy,
        ] {
            host::validate(kind).unwrap();
            let b = bundle(kind);
            assert_eq!(
                b.source,
                <[u8; 32]>::from(Sha256::digest(super::super::source(kind).as_bytes()))
            );
            for backend in [
                wgpu::Backend::Metal,
                wgpu::Backend::Vulkan,
                wgpu::Backend::Dx12,
            ] {
                for stage in [Stage::Vertex, Stage::Fragment] {
                    let a = b.select(backend, stage).unwrap();
                    let d = crate::shader::native::descriptor(a).unwrap();
                    assert!(d.wgsl.is_none());
                    assert_eq!(d.msl.is_some(), backend == wgpu::Backend::Metal);
                    assert_eq!(d.spirv.is_some(), backend == wgpu::Backend::Vulkan);
                    assert_eq!(d.hlsl.is_some(), backend == wgpu::Backend::Dx12);
                    let mut corrupt = *a;
                    corrupt.hash[0] ^= 1;
                    assert!(crate::shader::native::descriptor(&corrupt).is_err());
                }
            }
            assert!(b.select(wgpu::Backend::Gl, Stage::Vertex).is_err());
            assert!(
                b.select(wgpu::Backend::BrowserWebGpu, Stage::Vertex)
                    .is_err()
            );
        }
        let wrong = Bundle {
            expected: motion::BUNDLE.expected,
            ..resolve::BUNDLE
        };
        assert!(wrong.select(wgpu::Backend::Metal, Stage::Vertex).is_err());
    }
}
