use crate::{Backend, CookError, Result};
use naga::{
    Module, ShaderStage,
    back::{hlsl, msl, spv},
    proc::{BoundsCheckPolicies, BoundsCheckPolicy},
    valid::ModuleInfo,
};

fn bounds() -> BoundsCheckPolicies {
    BoundsCheckPolicies {
        index: BoundsCheckPolicy::Restrict,
        buffer: BoundsCheckPolicy::Restrict,
        image_load: BoundsCheckPolicy::Restrict,
        binding_array: BoundsCheckPolicy::Restrict,
    }
}
fn resource(binding: u32) -> naga::ResourceBinding {
    naga::ResourceBinding { group: 0, binding }
}
fn error(e: impl std::fmt::Display) -> CookError {
    CookError::Emit(e.to_string())
}

pub fn emit(
    m: &Module,
    info: &ModuleInfo,
    full: &Module,
    stage: ShaderStage,
    entry: &str,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    match backend {
        Backend::Metal => {
            // wgpu-hal 29.0.4 metal/device.rs::create_pipeline_layout allocates
            // independent texture/buffer counters, filtered by stage visibility.
            let mut resources = msl::EntryPointResources::default();
            if stage == ShaderStage::Fragment {
                resources.resources.insert(
                    resource(0),
                    msl::BindTarget {
                        texture: Some(0),
                        ..Default::default()
                    },
                );
                resources.resources.insert(
                    resource(1),
                    msl::BindTarget {
                        buffer: Some(0),
                        ..Default::default()
                    },
                );
            }
            let options = msl::Options {
                lang_version: (2, 4),
                fake_missing_bindings: false,
                per_entry_point_map: [(entry.to_owned(), resources)].into(),
                bounds_check_policies: bounds(),
                force_loop_bounding: true,
                ..Default::default()
            };
            let pipeline = msl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
                // The contract refuses vertex inputs/storage and workgroup memory.
                // No vertex pulling or hidden sizes/workgroup arguments are needed.
                vertex_pulling_transform: false,
                ..Default::default()
            };
            let (text, reflection) =
                msl::write_string(m, info, &options, &pipeline).map_err(error)?;
            let name = one_entry(
                reflection
                    .entry_point_names
                    .into_iter()
                    .map(|r| r.map_err(error)),
            )?;
            Ok((text.into_bytes(), name))
        }
        Backend::Vulkan => {
            let options = spv::Options {
                lang_version: (1, 0),
                // hal uses a negative-height viewport. Naga's default Y adjustment
                // would invert twice. No profile-dependent DEBUG/default flags.
                flags: spv::WriterFlags::FORCE_POINT_SIZE,
                fake_missing_bindings: false,
                binding_map: (0..2)
                    .map(|binding| {
                        (
                            resource(binding),
                            spv::BindingInfo {
                                descriptor_set: 0,
                                binding,
                                binding_array_size: None,
                            },
                        )
                    })
                    .collect(),
                capabilities: Some(
                    [spv::Capability::Shader, spv::Capability::ImageQuery]
                        .into_iter()
                        .collect(),
                ),
                bounds_check_policies: bounds(),
                force_loop_bounding: true,
                ..Default::default()
            };
            let words = spv::write_vec(
                m,
                info,
                &options,
                Some(&spv::PipelineOptions {
                    shader_stage: stage,
                    entry_point: entry.to_owned(),
                }),
            )
            .map_err(error)?;
            Ok((
                words.iter().flat_map(|word| word.to_le_bytes()).collect(),
                entry.to_owned(),
            ))
        }
        Backend::Dx12 => {
            // hal allocates SRV t0 and CBV b0 in space0; no samplers/immediates.
            // Its unconditional FIRST_VERTEX_INSTANCE layout flag appends b1.
            let options = hlsl::Options {
                shader_model: hlsl::ShaderModel::V6_0,
                fake_missing_bindings: false,
                binding_map: (0..2)
                    .map(|binding| {
                        (
                            resource(binding),
                            hlsl::BindTarget {
                                space: 0,
                                register: 0,
                                restrict_indexing: true,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                special_constants_binding: Some(hlsl::BindTarget {
                    space: 0,
                    register: 1,
                    ..Default::default()
                }),
                restrict_indexing: true,
                force_loop_bounding: true,
                ..Default::default()
            };
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
            };
            let fragment = hlsl::FragmentEntryPoint::new(full, "fragment")
                .ok_or(CookError::Contract("paired fragment"))?;
            let mut text = String::new();
            let reflection = hlsl::Writer::new(&mut text, &options, &pipeline)
                .write(m, info, Some(&fragment))
                .map_err(error)?;
            let name = one_entry(
                reflection
                    .entry_point_names
                    .into_iter()
                    .map(|r| r.map_err(error)),
            )?;
            Ok((text.into_bytes(), name))
        }
    }
}
fn one_entry(mut names: impl Iterator<Item = Result<String>>) -> Result<String> {
    let name = names
        .next()
        .ok_or(CookError::Contract("missing emitted entry"))??;
    if names.next().is_some() {
        return Err(CookError::Contract("extra emitted entry"));
    }
    Ok(name)
}
