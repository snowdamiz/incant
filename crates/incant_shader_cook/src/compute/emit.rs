use crate::{Backend, CookError, Result};
use naga::{
    ShaderStage,
    back::{hlsl, msl, spv},
    proc::{BoundsCheckPolicies, BoundsCheckPolicy},
};
fn bounds() -> BoundsCheckPolicies {
    BoundsCheckPolicies {
        index: BoundsCheckPolicy::Restrict,
        buffer: BoundsCheckPolicy::Restrict,
        image_load: BoundsCheckPolicy::Restrict,
        binding_array: BoundsCheckPolicy::Restrict,
    }
}
fn err(e: impl std::fmt::Display) -> CookError {
    CookError::Emit(e.to_string())
}
fn one(mut entries: impl Iterator<Item = Result<String>>) -> Result<String> {
    let name = entries
        .next()
        .ok_or(CookError::Contract("cluster emitted entry"))??;
    if entries.next().is_some() {
        return Err(CookError::Contract("cluster extra entry"));
    }
    Ok(name)
}
pub(super) fn emit(
    m: &naga::Module,
    info: &naga::valid::ModuleInfo,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    match backend {
        Backend::Metal => {
            let (text, entry) = metal(m, info)?;
            Ok((
                super::static_workgroup::lower(&text, &entry)?.into_bytes(),
                entry,
            ))
        }
        Backend::Vulkan => {
            let words = spirv(m, info)?;
            Ok((
                super::spirv_workgroup::lower(&words)?
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect(),
                "build".into(),
            ))
        }
        Backend::Dx12 => {
            // hal29 allocates separate CBV/SRV/UAV counters. Host binding4 is
            // the explicit Metal-length uniform (b1), so special constants use b2.
            let bind = |register| hlsl::BindTarget {
                space: 0,
                register,
                restrict_indexing: true,
                ..Default::default()
            };
            let mut options = hlsl::Options {
                shader_model: hlsl::ShaderModel::V6_0,
                fake_missing_bindings: false,
                special_constants_binding: Some(bind(2)),
                restrict_indexing: true,
                force_loop_bounding: true,
                zero_initialize_workgroup_memory: true,
                ..Default::default()
            };
            for (_, g) in m.global_variables.iter() {
                if let Some(b) = g.binding {
                    options
                        .binding_map
                        .insert(b, bind(if b.binding == 3 { 1 } else { 0 }));
                }
            }
            let mut text = String::new();
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((ShaderStage::Compute, "build".into())),
            };
            let reflection = hlsl::Writer::new(&mut text, &options, &pipeline)
                .write(m, info, None)
                .map_err(err)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|x| x.map_err(err)))?,
            ))
        }
    }
}

fn metal(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<(String, String)> {
    let mut resources = msl::EntryPointResources {
        sizes_buffer: Some(4),
        ..Default::default()
    };
    for (_, g) in m.global_variables.iter() {
        if let Some(b) = g.binding {
            resources.resources.insert(
                b,
                msl::BindTarget {
                    buffer: Some(b.binding as u8),
                    mutable: b.binding >= 2,
                    ..Default::default()
                },
            );
        }
    }
    let options = msl::Options {
        lang_version: (2, 4),
        fake_missing_bindings: false,
        per_entry_point_map: [("build".into(), resources)].into(),
        bounds_check_policies: bounds(),
        force_loop_bounding: true,
        zero_initialize_workgroup_memory: true,
        ..Default::default()
    };
    let pipeline = msl::PipelineOptions {
        entry_point: Some((ShaderStage::Compute, "build".into())),
        ..Default::default()
    };
    let (text, reflection) = msl::write_string(m, info, &options, &pipeline).map_err(err)?;
    let entry = one(reflection
        .entry_point_names
        .into_iter()
        .map(|x| x.map_err(err)))?;
    Ok((text, entry))
}

fn spirv(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<Vec<u32>> {
    let options = spv::Options {
        lang_version: (1, 0),
        fake_missing_bindings: false,
        binding_map: m
            .global_variables
            .iter()
            .filter_map(|(_, g)| g.binding)
            .map(|b| {
                (
                    b,
                    spv::BindingInfo {
                        descriptor_set: 0,
                        binding: b.binding,
                        binding_array_size: None,
                    },
                )
            })
            .collect(),
        capabilities: Some([spv::Capability::Shader].into_iter().collect()),
        bounds_check_policies: bounds(),
        force_loop_bounding: true,
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::Polyfill,
        ..Default::default()
    };
    spv::write_vec(
        m,
        info,
        &options,
        Some(&spv::PipelineOptions {
            shader_stage: ShaderStage::Compute,
            entry_point: "build".into(),
        }),
    )
    .map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_spirv_correction_preserves_every_word_except_workgroup_stride() {
        let source = include_str!("../../../incant_render/src/lighting/grid.wgsl");
        let m = naga::front::wgsl::parse_str(source).unwrap();
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
        let original = spirv(&m, &info).unwrap();
        let corrected = super::super::spirv_workgroup::lower(&original).unwrap();
        assert_eq!(original.len(), corrected.len() + 4);
        let mut at = 5;
        let mut removed = 0;
        while at < original.len() {
            let n = (original[at] >> 16) as usize;
            if n == 4
                && original[at] & 0xffff == 71
                && original[at + 2..at + 4] == [6, 4]
                && original[..at] == corrected[..at.min(corrected.len())]
                && original[at + 4..] == corrected[at..]
            {
                removed += 1;
            }
            at += n;
        }
        assert_eq!(removed, 1);
    }
    #[test]
    fn actual_generated_metal_body_and_helpers_are_byte_exact_after_lowering() {
        let source = include_str!("../../../incant_render/src/lighting/grid.wgsl");
        let m = naga::front::wgsl::parse_str(source).unwrap();
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
        let (original, entry) = metal(&m, &info).unwrap();
        let lowered = super::super::static_workgroup::lower(&original, &entry).unwrap();
        assert_eq!(super::super::static_workgroup::restore(&lowered), original);
        assert_eq!(lowered.matches("metal::threadgroup_barrier").count(), 3);
    }
}
