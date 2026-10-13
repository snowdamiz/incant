//! Trusted embedded compute only; no project-supplied passthrough artifacts.
use crate::{
    ResourceError, Result,
    shader::native::{self, Artifact, Stage},
};
include!(concat!(env!("OUT_DIR"), "/cluster-shaders/manifest.rs"));
const CONTRACT: &str = "incant-cluster-v1;naga=29.0.4;wgpu=29.0.4;group64x1x1;shared=atomic4+128x4;msl=2.4,static-threadgroup,sizes4;spv=1.0,implicit-workgroup-layout;hlsl=6.0,grid-b0,light-t0,counts-u0,masks-u1,sizes-b1,special-b2;bounds=restrict;loops=bounded";
const _: () = {
    assert!(super::MAX_MASK_WORDS == 128);
    assert!(super::scene::MAX_LOCAL == 4096);
    assert!(super::DEPTH_SLICES == 24);
};
#[cfg(feature = "cooked-temporal")]
mod temporal {
    use super::*;
    include!(concat!(
        env!("OUT_DIR"),
        "/temporal-cluster-shaders/manifest.rs"
    ));
    pub(super) fn manifest() -> (&'static str, [u8; 32], [u8; 32], &'static [Artifact]) {
        (MANIFEST_CONTRACT, SOURCE_HASH, PRODUCER_HASH, &ARTIFACTS)
    }
}
fn select_variant(
    backend: wgpu::Backend,
    temporal: bool,
) -> std::result::Result<&'static Artifact, &'static str> {
    let (contract, source, producer, artifacts) = if temporal {
        #[cfg(feature = "cooked-temporal")]
        {
            temporal::manifest()
        }
        #[cfg(not(feature = "cooked-temporal"))]
        {
            return Err("temporal cluster variant unavailable");
        }
    } else {
        (
            MANIFEST_CONTRACT,
            SOURCE_HASH,
            PRODUCER_HASH,
            &ARTIFACTS[..],
        )
    };
    if contract != CONTRACT || source == [0; 32] || producer == [0; 32] || artifacts.len() != 3 {
        return Err("cluster manifest identity");
    }
    #[cfg(feature = "cooked-temporal")]
    if temporal {
        use sha2::{Digest, Sha256};
        let expected =
            super::temporal_grid_source().map_err(|_| "temporal cluster guarded source")?;
        if source != <[u8; 32]>::from(Sha256::digest(expected)) {
            return Err("temporal cluster source identity");
        }
    }
    let mut found = artifacts
        .iter()
        .filter(|a| a.backend == backend && a.stage == Stage::Compute);
    let a = found.next().ok_or("missing cluster backend")?;
    if found.next().is_some() {
        return Err("duplicate cluster backend");
    }
    Ok(a)
}
fn validate_host() -> std::result::Result<(), &'static str> {
    let actual = super::layout_entries(true);
    let expected: Vec<_> = (0..=4)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 0 || binding == 4 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage {
                        read_only: binding == 1,
                    }
                },
                has_dynamic_offset: false,
                min_binding_size: if binding == 4 {
                    wgpu::BufferSize::new(16)
                } else {
                    None
                },
            },
        })
        .collect();
    if actual != expected {
        return Err("cluster host layout");
    };
    Ok(())
}
fn validate_limits(l: &wgpu::Limits) -> std::result::Result<(), &'static str> {
    if l.max_compute_invocations_per_workgroup < 64
        || l.max_compute_workgroup_size_x < 64
        || l.max_compute_workgroup_size_y < 1
        || l.max_compute_workgroup_size_z < 1
        || l.max_compute_workgroup_storage_size < 516
    {
        return Err("cluster workgroup limits");
    }
    Ok(())
}
pub(super) fn shader(
    device: &wgpu::Device,
    backend: wgpu::Backend,
) -> Result<(wgpu::ShaderModule, &'static str)> {
    shader_variant(device, backend, false)
}
#[cfg(feature = "cooked-temporal")]
pub(super) fn temporal_shader(
    device: &wgpu::Device,
    backend: wgpu::Backend,
) -> Result<(wgpu::ShaderModule, &'static str)> {
    shader_variant(device, backend, true)
}
fn shader_variant(
    device: &wgpu::Device,
    backend: wgpu::Backend,
    temporal: bool,
) -> Result<(wgpu::ShaderModule, &'static str)> {
    validate_host().map_err(ResourceError::CookedCluster)?;
    validate_limits(&device.limits()).map_err(ResourceError::CookedCluster)?;
    native::features(backend, device.features()).map_err(ResourceError::CookedCluster)?;
    let a = select_variant(backend, temporal).map_err(ResourceError::CookedCluster)?;
    let descriptor = native::descriptor(a).map_err(ResourceError::CookedCluster)?;
    // SAFETY: private embedded source has exact validated IR/host layout, bounded
    // storage accesses and loops, fixed64x1x1 and 516B static workgroup storage.
    // The MSL lowering changes declarations only; zero-init/barriers/body remain.
    let module = unsafe { device.create_shader_module_passthrough(descriptor) };
    Ok((module, a.entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[cfg(feature = "cooked-temporal")]
    #[test]
    fn guarded_temporal_variant_is_distinct_and_has_identical_native_compute_abi() {
        let (_, hash, _, _) = temporal::manifest();
        assert_eq!(
            hash,
            <[u8; 32]>::from(Sha256::digest(
                super::super::temporal_grid_source().unwrap()
            ))
        );
        assert_ne!(hash, SOURCE_HASH);
        for b in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            let a = select_variant(b, true).unwrap();
            let original = select_variant(b, false).unwrap();
            assert_ne!(a.hash, original.hash);
            assert_eq!(native::descriptor(a).unwrap().num_workgroups, (64, 1, 1));
        }
    }
    #[test]
    fn exact_compute_descriptors_match_the_fixed_host_and_workgroup() {
        validate_host().unwrap();
        validate_limits(&wgpu::Limits::default()).unwrap();
        assert_eq!(
            SOURCE_HASH,
            <[u8; 32]>::from(Sha256::digest(include_str!("grid.wgsl")))
        );
        for b in [
            wgpu::Backend::Metal,
            wgpu::Backend::Vulkan,
            wgpu::Backend::Dx12,
        ] {
            let a = select_variant(b, false).unwrap();
            let d = native::descriptor(a).unwrap();
            assert_eq!(d.num_workgroups, (64, 1, 1));
            assert_eq!(d.msl.is_some(), b == wgpu::Backend::Metal);
            assert_eq!(d.hlsl.is_some(), b == wgpu::Backend::Dx12);
            assert_eq!(d.spirv.is_some(), b == wgpu::Backend::Vulkan);
            let mut bad = *a;
            bad.hash[0] ^= 1;
            assert!(native::descriptor(&bad).is_err());
        }
        assert!(select_variant(wgpu::Backend::Gl, false).is_err());
        let l = wgpu::Limits {
            max_compute_workgroup_storage_size: 515,
            ..Default::default()
        };
        assert!(validate_limits(&l).is_err());
        let l = wgpu::Limits {
            max_compute_invocations_per_workgroup: 63,
            ..Default::default()
        };
        assert!(validate_limits(&l).is_err());
    }
}
