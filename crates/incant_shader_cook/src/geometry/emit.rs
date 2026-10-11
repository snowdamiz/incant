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
fn resource(group: u32, binding: u32) -> naga::ResourceBinding {
    naga::ResourceBinding { group, binding }
}
fn error(e: impl std::fmt::Display) -> CookError {
    CookError::Emit(e.to_string())
}
fn one(mut names: impl Iterator<Item = Result<String>>) -> Result<String> {
    let name = names
        .next()
        .ok_or(CookError::Contract("geometry emitted entry"))??;
    if names.next().is_some() {
        return Err(CookError::Contract("geometry extra entry"));
    }
    Ok(name)
}

fn vertex_mappings(kind: Kind) -> Vec<msl::VertexBufferMapping> {
    use msl::{
        AttributeMapping as A, VertexBufferMapping as B, VertexBufferStepMode as S,
        VertexFormat as F,
    };
    let attribute = |shader_location, offset, format| A {
        shader_location,
        offset,
        format,
    };
    match kind {
        Kind::Diagnostic => vec![B {
            id: 15,
            stride: 28,
            step_mode: S::ByVertex,
            attributes: vec![
                attribute(0, 0, F::Float32x4),
                attribute(1, 16, F::Float32x3),
            ],
        }],
        Kind::Shadow => vec![
            B {
                id: 15,
                stride: 48,
                step_mode: S::ByVertex,
                attributes: vec![
                    attribute(0, 0, F::Float32x3),
                    attribute(1, 12, F::Float32x3),
                    attribute(9, 24, F::Float32x2),
                    attribute(10, 32, F::Float32x4),
                ],
            },
            B {
                id: 14,
                stride: 112,
                step_mode: S::ByInstance,
                attributes: (2..=8)
                    .map(|n| attribute(n, (n - 2) * 16, F::Float32x4))
                    .collect(),
            },
        ],
    }
}

pub(super) fn emit(
    kind: Kind,
    m: &Module,
    info: &ModuleInfo,
    full: &Module,
    stage: ShaderStage,
    entry: &str,
    backend: Backend,
) -> Result<(Vec<u8>, String)> {
    let shadow = kind == Kind::Shadow;
    match backend {
        Backend::Metal => {
            let mut resources = msl::EntryPointResources::default();
            if stage == ShaderStage::Vertex {
                // hal29 reserves one vertex lengths buffer after ordinary buffers.
                resources.sizes_buffer = Some(u8::from(shadow));
                if shadow {
                    resources.resources.insert(
                        resource(0, 0),
                        msl::BindTarget {
                            buffer: Some(0),
                            ..Default::default()
                        },
                    );
                }
            } else if shadow {
                resources.resources.insert(
                    resource(1, 0),
                    msl::BindTarget {
                        buffer: Some(0),
                        ..Default::default()
                    },
                );
                resources.resources.insert(
                    resource(1, 1),
                    msl::BindTarget {
                        texture: Some(0),
                        ..Default::default()
                    },
                );
                resources.resources.insert(
                    resource(1, 2),
                    msl::BindTarget {
                        sampler: Some(msl::BindSamplerTarget::Resource(0)),
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
                vertex_pulling_transform: true,
                vertex_buffer_mappings: if stage == ShaderStage::Vertex {
                    vertex_mappings(kind)
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
                    .map(|r| r.map_err(error)))?,
            ))
        }
        Backend::Vulkan => {
            let options = spv::Options {
                lang_version: (1, 0),
                flags: spv::WriterFlags::FORCE_POINT_SIZE,
                fake_missing_bindings: false,
                binding_map: if shadow {
                    [(0, 0), (1, 0), (1, 1), (1, 2)]
                        .into_iter()
                        .map(|(group, binding)| {
                            (
                                resource(group, binding),
                                spv::BindingInfo {
                                    descriptor_set: group,
                                    binding,
                                    binding_array_size: None,
                                },
                            )
                        })
                        .collect()
                } else {
                    Default::default()
                },
                capabilities: Some([spv::Capability::Shader].into_iter().collect()),
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
            let binding = |register| hlsl::BindTarget {
                space: 0,
                register,
                restrict_indexing: true,
                ..Default::default()
            };
            let mut options = hlsl::Options {
                shader_model: hlsl::ShaderModel::V6_0,
                fake_missing_bindings: false,
                special_constants_binding: Some(binding(if shadow { 2 } else { 0 })),
                restrict_indexing: true,
                force_loop_bounding: true,
                ..Default::default()
            };
            if shadow {
                options.binding_map = [
                    (resource(0, 0), binding(0)),
                    (resource(1, 0), binding(1)),
                    (resource(1, 1), binding(0)),
                    (
                        resource(1, 2),
                        hlsl::BindTarget {
                            space: 255,
                            register: 0,
                            ..Default::default()
                        },
                    ),
                ]
                .into();
                // Full material layout has five textures before its sampler-index
                // SRV, even though this shader uses only the first texture/sampler.
                options
                    .sampler_buffer_binding_map
                    .insert(hlsl::SamplerIndexBufferKey { group: 1 }, binding(5));
                options.sampler_heap_target = hlsl::SamplerHeapBindTargets {
                    standard_samplers: binding(0),
                    comparison_samplers: binding(2048),
                };
            }
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
            };
            let fragment = hlsl::FragmentEntryPoint::new(full, "fragment")
                .ok_or(CookError::Contract("geometry paired fragment"))?;
            let mut text = String::new();
            let reflection = hlsl::Writer::new(&mut text, &options, &pipeline)
                .write(m, info, Some(&fragment))
                .map_err(error)?;
            Ok((
                text.into_bytes(),
                one(reflection
                    .entry_point_names
                    .into_iter()
                    .map(|r| r.map_err(error)))?,
            ))
        }
    }
}
