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
fn select(backend: wgpu::Backend) -> std::result::Result<&'static Artifact, &'static str> {
    if MANIFEST_CONTRACT != CONTRACT
        || SOURCE_HASH == [0; 32]
        || PRODUCER_HASH == [0; 32]
        || ARTIFACTS.len() != 3
    {
        return Err("cluster manifest identity");
    }
    let mut found = ARTIFACTS
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
    validate_host().map_err(ResourceError::CookedCluster)?;
    validate_limits(&device.limits()).map_err(ResourceError::CookedCluster)?;
    native::features(backend, device.features()).map_err(ResourceError::CookedCluster)?;
    let a = select(backend).map_err(ResourceError::CookedCluster)?;
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
            let a = select(b).unwrap();
            let d = native::descriptor(a).unwrap();
            assert_eq!(d.num_workgroups, (64, 1, 1));
            assert_eq!(d.msl.is_some(), b == wgpu::Backend::Metal);
            assert_eq!(d.hlsl.is_some(), b == wgpu::Backend::Dx12);
            assert_eq!(d.spirv.is_some(), b == wgpu::Backend::Vulkan);
            let mut bad = *a;
            bad.hash[0] ^= 1;
            assert!(native::descriptor(&bad).is_err());
        }
        assert!(select(wgpu::Backend::Gl).is_err());
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
