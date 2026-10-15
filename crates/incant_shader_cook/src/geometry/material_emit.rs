//! Match wgpu-hal 29's fixed four-group material layout on each backend.
use super::material::Variant;
use super::{
    Kind,
    emit::{bounds, vertex_mappings},
};
use crate::{Backend, CookError, Result};
use naga::{
    Module, ShaderStage,
    back::{hlsl, msl, spv},
    valid::ModuleInfo,
};

fn resource(group: u32, binding: u32) -> naga::ResourceBinding {
    naga::ResourceBinding { group, binding }
}
fn error(e: impl std::fmt::Display) -> CookError {
    CookError::Emit(e.to_string())
}
fn one(mut entries: impl Iterator<Item = Result<String>>) -> Result<String> {
    let entry = entries
        .next()
        .ok_or(CookError::Contract("material emitted entry"))??;
    if entries.next().is_some() {
        return Err(CookError::Contract("material extra entry"));
    }
    Ok(entry)
}
pub(super) fn emit(
    m: &Module,
    info: &ModuleInfo,
    full: &Module,
    stage: ShaderStage,
    entry: &str,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    emit_variant(Variant::Ordinary, m, info, full, stage, entry, backend)
}
pub(crate) fn emit_variant(
    variant: Variant,
    m: &Module,
    info: &ModuleInfo,
    full: &Module,
    stage: ShaderStage,
    entry: &str,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    let vertex = stage == ShaderStage::Vertex;
    let offset = u8::from(variant == Variant::Occlusion && !vertex);
    match backend {
        Backend::Metal => {
            let dynamic: Vec<_> = m
                .global_variables
                .iter()
                .filter_map(|(_, g)| {
                    matches!(
                        m.types[g.ty].inner,
                        naga::TypeInner::Array {
                            size: naga::ArraySize::Dynamic,
                            ..
                        }
                    )
                    .then_some(g.binding)
                    .flatten()
                })
                .collect();
            let expected = if vertex || matches!(variant, Variant::Prepass | Variant::Fallback) {
                vec![]
            } else {
                vec![resource(3, 1), resource(3, 2), resource(3, 3)]
            };
            if dynamic != expected {
                return Err(CookError::Contract("material compact storage lengths"));
            }
            let mut resources = msl::EntryPointResources {
                sizes_buffer: Some(if vertex {
                    1
                } else if variant == Variant::Prepass {
                    2
                } else {
                    7
                }),
                ..Default::default()
            };
            for (_, g) in m.global_variables.iter() {
                let b = g
                    .binding
                    .ok_or(CookError::Contract("material emitted binding"))?;
                let mut target = msl::BindTarget::default();
                match (b.group, b.binding) {
                    (0, 0) => target.buffer = Some(0),
                    (0, 1) if variant == Variant::Occlusion => target.texture = Some(0),
                    (1, 0) => target.buffer = Some(1),
                    (1, n @ (1 | 3 | 5 | 7 | 9)) => {
                        target.texture = Some(((n - 1) / 2) as u8 + offset)
                    }
                    (1, n @ (2 | 4 | 6 | 8 | 10)) => {
                        target.sampler = Some(msl::BindSamplerTarget::Resource(((n - 2) / 2) as u8))
                    }
                    (2, n @ 0..=2) => target.texture = Some((5 + n) as u8 + offset),
                    (2, 3) => target.sampler = Some(msl::BindSamplerTarget::Resource(5)),
                    (3, n @ 0..=3) => target.buffer = Some((2 + n) as u8),
                    (3, 4) => target.texture = Some(8 + offset),
                    (3, 5) => target.sampler = Some(msl::BindSamplerTarget::Resource(6)),
                    (3, 6) => target.buffer = Some(6),
                    _ => return Err(CookError::Contract("material Metal resource")),
                }
                resources.resources.insert(b, target);
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
                vertex_pulling_transform: true,
                vertex_buffer_mappings: if vertex {
                    vertex_mappings(Kind::Material)
                } else {
                    vec![]
                },
                allow_and_force_point_size: false,
            };
            let (text, reflection) =
                msl::write_string(m, info, &options, &pipeline).map_err(error)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|x| x.map_err(error)))?,
            ))
        }
        Backend::Vulkan => {
            let options = spv::Options {
                lang_version: (1, 0),
                flags: spv::WriterFlags::FORCE_POINT_SIZE,
                fake_missing_bindings: false,
                binding_map: m
                    .global_variables
                    .iter()
                    .filter_map(|(_, g)| g.binding)
                    .map(|b| {
                        (
                            b,
                            spv::BindingInfo {
                                descriptor_set: b.group,
                                binding: b.binding,
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
                words.iter().flat_map(|w| w.to_le_bytes()).collect(),
                entry.to_owned(),
            ))
        }
        Backend::Dx12 => {
            let bind = |register| hlsl::BindTarget {
                space: 0,
                register,
                restrict_indexing: true,
                ..Default::default()
            };
            let mut options = hlsl::Options {
                shader_model: hlsl::ShaderModel::V6_0,
                fake_missing_bindings: false,
                special_constants_binding: Some(bind(if variant == Variant::Prepass {
                    2
                } else {
                    5
                })),
                restrict_indexing: true,
                force_loop_bounding: true,
                ..Default::default()
            };
            for (_, g) in m.global_variables.iter() {
                let b = g
                    .binding
                    .ok_or(CookError::Contract("material emitted binding"))?;
                let target = match (b.group, b.binding) {
                    (0, 0) => bind(0),
                    (0, 1) if variant == Variant::Occlusion => bind(0),
                    (1, 0) => bind(1),
                    (1, n @ (1 | 3 | 5 | 7 | 9)) => bind((n - 1) / 2 + u32::from(offset)),
                    (1, n @ (2 | 4 | 6 | 8 | 10)) => hlsl::BindTarget {
                        space: 255,
                        register: (n - 2) / 2,
                        ..Default::default()
                    },
                    (2, n @ 0..=2) => bind(6 + n + u32::from(offset)),
                    (2, 3) | (3, 5) => hlsl::BindTarget {
                        space: 255,
                        register: 0,
                        ..Default::default()
                    },
                    (3, 0) => bind(2),
                    (3, n @ 1..=3) => bind(9 + n + u32::from(offset)),
                    (3, 4) => bind(13 + u32::from(offset)),
                    (3, 6) => bind(3),
                    _ => return Err(CookError::Contract("material HLSL resource")),
                };
                options.binding_map.insert(b, target);
            }
            options.sampler_buffer_binding_map = [
                (
                    hlsl::SamplerIndexBufferKey { group: 1 },
                    bind(5 + u32::from(offset)),
                ),
                (
                    hlsl::SamplerIndexBufferKey { group: 2 },
                    bind(9 + u32::from(offset)),
                ),
                (
                    hlsl::SamplerIndexBufferKey { group: 3 },
                    bind(14 + u32::from(offset)),
                ),
            ]
            .into();
            options.sampler_heap_target = hlsl::SamplerHeapBindTargets {
                standard_samplers: bind(0),
                comparison_samplers: bind(2048),
            };
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
            };
            let fragment = hlsl::FragmentEntryPoint::new(full, "fragment")
                .ok_or(CookError::Contract("material paired fragment"))?;
            let mut text = String::new();
            let reflection = hlsl::Writer::new(&mut text, &options, &pipeline)
                .write(m, info, Some(&fragment))
                .map_err(error)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|x| x.map_err(error)))?,
            ))
        }
    }
}
