//! Pinned hal29 layouts. Metal compacts buffers per stage; DX12 counts the full
//! declared layout (including material sampler-index SRV after five textures).
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
fn error(e: impl std::fmt::Display) -> CookError {
    CookError::Emit(e.to_string())
}
fn one(mut names: impl Iterator<Item = Result<String>>) -> Result<String> {
    let name = names
        .next()
        .ok_or(CookError::Contract("temporal emitted entry"))??;
    if names.next().is_some() {
        return Err(CookError::Contract("temporal extra entry"));
    }
    Ok(name)
}
fn motion_buffers(owner: bool) -> Vec<msl::VertexBufferMapping> {
    use msl::{
        AttributeMapping as A, VertexBufferMapping as B, VertexBufferStepMode as S,
        VertexFormat as F,
    };
    let a = |shader_location, offset, format| A {
        shader_location,
        offset,
        format,
    };
    vec![
        B {
            id: 15,
            stride: 48,
            step_mode: S::ByVertex,
            attributes: vec![a(0, 0, F::Float32x3), a(1, 24, F::Float32x2)],
        },
        B {
            id: 14,
            stride: 48,
            step_mode: S::ByVertex,
            attributes: vec![a(2, 0, F::Float32x3)],
        },
        B {
            id: 13,
            stride: 112,
            step_mode: S::ByInstance,
            attributes: (3..=6).map(|i| a(i, (i - 3) * 16, F::Float32x4)).collect(),
        },
        B {
            id: 12,
            stride: 80,
            step_mode: S::ByInstance,
            attributes: if owner {
                (7..=10)
                    .map(|i| a(i, (i - 7) * 16, F::Float32x4))
                    .chain([a(11, 64, F::Float32), a(12, 68, F::Uint32)])
                    .collect()
            } else {
                (7..=11).map(|i| a(i, (i - 7) * 16, F::Float32x4)).collect()
            },
        },
    ]
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
    let motion = matches!(kind, Kind::Motion | Kind::Depth | Kind::MotionOwner);
    let copy = kind == Kind::Copy;
    let vertex = stage == ShaderStage::Vertex;
    match backend {
        Backend::Metal => {
            let mut resources = msl::EntryPointResources::default();
            if vertex && motion {
                resources.sizes_buffer = Some(1);
            }
            for (_, g) in m.global_variables.iter() {
                let b = g
                    .binding
                    .ok_or(CookError::Contract("temporal emitted binding"))?;
                let mut target = msl::BindTarget::default();
                if g.space == naga::AddressSpace::Uniform {
                    target.buffer = Some(if motion && b.group == 1 { 1 } else { 0 });
                } else if motion && b.binding == 2 {
                    target.sampler = Some(msl::BindSamplerTarget::Resource(0));
                } else {
                    target.texture = Some(if motion || copy {
                        0
                    } else {
                        (b.binding - 1) as u8
                    });
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
                vertex_pulling_transform: motion,
                vertex_buffer_mappings: if vertex && motion {
                    motion_buffers(kind == Kind::MotionOwner)
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
                special_constants_binding: Some(bind(if motion { 2 } else { 1 })),
                restrict_indexing: true,
                force_loop_bounding: true,
                ..Default::default()
            };
            for (_, g) in m.global_variables.iter() {
                let b = g
                    .binding
                    .ok_or(CookError::Contract("temporal emitted binding"))?;
                let target = if g.space == naga::AddressSpace::Uniform {
                    bind(u32::from(motion && b.group == 1))
                } else if motion && b.binding == 2 {
                    hlsl::BindTarget {
                        space: 255,
                        register: 0,
                        ..Default::default()
                    }
                } else {
                    bind(if motion || copy { 0 } else { b.binding - 1 })
                };
                options.binding_map.insert(b, target);
            }
            if motion {
                options
                    .sampler_buffer_binding_map
                    .insert(hlsl::SamplerIndexBufferKey { group: 1 }, bind(5));
                options.sampler_heap_target = hlsl::SamplerHeapBindTargets {
                    standard_samplers: bind(0),
                    comparison_samplers: bind(2048),
                };
            }
            let pipeline = hlsl::PipelineOptions {
                entry_point: Some((stage, entry.to_owned())),
            };
            let fragment = hlsl::FragmentEntryPoint::new(full, kind.entry())
                .ok_or(CookError::Contract("temporal paired fragment"))?;
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

#[cfg(test)]
mod owner_tests {
    use super::*;
    #[test]
    fn exact_native_owner_pull_keeps_integer_word_and_old_layout() {
        let old = motion_buffers(false);
        let owner = motion_buffers(true);
        assert_eq!(owner.len(), 4);
        for (a, b) in old[..3].iter().zip(&owner[..3]) {
            assert_eq!(format!("{a:?}"), format!("{b:?}"));
        }
        assert_eq!(owner[3].id, 12);
        assert_eq!(owner[3].stride, 80);
        let a = &owner[3].attributes;
        assert_eq!(a.len(), 6);
        for (i, a) in a[..4].iter().enumerate() {
            assert_eq!(
                (a.shader_location, a.offset, a.format),
                (7 + i as u32, i as u32 * 16, msl::VertexFormat::Float32x4)
            );
        }
        assert_eq!(
            (a[4].shader_location, a[4].offset, a[4].format),
            (11, 64, msl::VertexFormat::Float32)
        );
        assert_eq!(
            (a[5].shader_location, a[5].offset, a[5].format),
            (12, 68, msl::VertexFormat::Uint32)
        );
        assert_eq!(old[3].attributes.len(), 5);
        assert_eq!(old[3].attributes[4].format, msl::VertexFormat::Float32x4);
    }
}
