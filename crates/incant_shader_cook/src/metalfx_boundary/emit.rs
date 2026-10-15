//! Fixed hal29 fragment-only buffer/texture maps. No samplers or vertex buffers.
use super::Kind;
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
fn error(error: impl std::fmt::Display) -> CookError {
    CookError::Emit(error.to_string())
}
fn one(mut names: impl Iterator<Item = Result<String>>) -> Result<String> {
    let name = names
        .next()
        .ok_or(CookError::Contract("MetalFX boundary emitted entry"))??;
    if names.next().is_some() {
        return Err(CookError::Contract("MetalFX boundary extra emitted entry"));
    }
    Ok(name)
}
pub(super) fn emit(
    kind: Kind,
    module: &Module,
    info: &ModuleInfo,
    full: &Module,
    stage: ShaderStage,
    entry: &str,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    match backend {
        Backend::Metal => {
            let mut resources = msl::EntryPointResources::default();
            if stage == ShaderStage::Fragment {
                for binding in 0..4 {
                    resources.resources.insert(
                        resource(binding),
                        msl::BindTarget {
                            buffer: (binding == 0).then_some(0),
                            texture: (binding > 0).then_some(binding.saturating_sub(1) as u8),
                            ..Default::default()
                        },
                    );
                }
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
                vertex_pulling_transform: false,
                ..Default::default()
            };
            let (text, reflection) =
                msl::write_string(module, info, &options, &pipeline).map_err(error)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|name| name.map_err(error)))?,
            ))
        }
        Backend::Vulkan => {
            let options = spv::Options {
                lang_version: (1, 0),
                flags: spv::WriterFlags::FORCE_POINT_SIZE,
                fake_missing_bindings: false,
                binding_map: (0..4)
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
                module,
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
            let target = |register| hlsl::BindTarget {
                space: 0,
                register,
                restrict_indexing: true,
                ..Default::default()
            };
            let options = hlsl::Options {
                shader_model: hlsl::ShaderModel::V6_0,
                fake_missing_bindings: false,
                binding_map: (0..4)
                    .map(|binding| (resource(binding), target(binding.saturating_sub(1))))
                    .collect(),
                special_constants_binding: Some(target(1)),
                restrict_indexing: true,
                force_loop_bounding: true,
                ..Default::default()
            };
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
            };
            let fragment = hlsl::FragmentEntryPoint::new(full, kind.entry())
                .ok_or(CookError::Contract("MetalFX boundary paired fragment"))?;
            let mut text = String::new();
            let reflection = hlsl::Writer::new(&mut text, &options, &pipeline)
                .write(module, info, Some(&fragment))
                .map_err(error)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|name| name.map_err(error)))?,
            ))
        }
    }
}
