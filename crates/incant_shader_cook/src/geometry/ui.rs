//! Exact screen-space UI IO and resources; themes and shader math remain unchanged.
use super::sprite::io;
use crate::{CookError, Result};
use naga::{AddressSpace, Binding, BuiltIn, ScalarKind as K, ShaderStage, TypeInner};

fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}
fn vector(m: &naga::Module, ty: naga::Handle<naga::Type>, lanes: u8) -> bool {
    matches!(m.types[ty].inner, TypeInner::Vector { size, scalar }
        if size as u8 == lanes && scalar.kind == K::Float && scalar.width == 4)
}
pub(super) fn validate(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    require(m.overrides.is_empty(), "UI overrides")?;
    require(m.entry_points.len() == 2, "UI entry count")?;
    let quad = vec![
        (Some(0), K::Float, 4, false),
        (Some(1), K::Float, 4, false),
        (Some(2), K::Float, 4, false),
        (Some(3), K::Float, 2, false),
        (Some(4), K::Float, 4, false),
        (Some(5), K::Uint, 1, true),
    ];
    let varying = vec![
        (None, K::Float, 4, false),
        (Some(0), K::Float, 2, false),
        (Some(1), K::Float, 4, true),
        (Some(2), K::Float, 4, true),
        (Some(3), K::Float, 2, true),
        (Some(4), K::Float, 4, true),
        (Some(5), K::Uint, 1, true),
    ];
    let color = vec![(Some(0), K::Float, 4, false)];
    let mut stages = [false; 2];
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = match (entry.stage, entry.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") => false,
            _ => return Err(CookError::Contract("UI stage/name")),
        };
        let slot = usize::from(!vertex);
        require(!stages[slot], "UI repeated stage")?;
        stages[slot] = true;
        let mut inputs = Vec::new();
        let mut indices = 0;
        for arg in &entry.function.arguments {
            if arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex)) {
                require(
                    vertex
                        && matches!(
                            m.types[arg.ty].inner,
                            TypeInner::Scalar(naga::Scalar {
                                kind: K::Uint,
                                width: 4
                            })
                        ),
                    "UI vertex index",
                )?;
                indices += 1;
            } else {
                io(m, arg.ty, arg.binding.as_ref(), &mut inputs)?;
            }
        }
        require(indices == usize::from(vertex), "UI vertex index count")?;
        let mut outputs = Vec::new();
        if let Some(result) = &entry.function.result {
            io(m, result.ty, result.binding.as_ref(), &mut outputs)?;
        }
        require(
            inputs == *if vertex { &quad } else { &varying },
            "UI input interface",
        )?;
        require(
            outputs == *if vertex { &varying } else { &color },
            "UI output interface",
        )?;
        for (handle, global) in m.global_variables.iter() {
            let frame = global.binding
                == Some(naga::ResourceBinding {
                    group: 0,
                    binding: 0,
                });
            require(
                info.get_entry_point(index)[handle].is_empty() != (frame || !vertex),
                "UI stage visibility",
            )?;
        }
    }
    require(m.global_variables.len() == 3, "UI resource count")?;
    for (_, g) in m.global_variables.iter() {
        require(g.init.is_none(), "UI resource initializer")?;
        let b = g
            .binding
            .ok_or(CookError::Contract("UI unbound resource"))?;
        let valid = match (b.group, b.binding) {
            (0, 0) => {
                if let TypeInner::Struct { members, span } = &m.types[g.ty].inner {
                    g.space == AddressSpace::Uniform
                        && *span == 48
                        && members.len() == 4
                        && members.iter().zip([(0, 4), (16, 2), (24, 2), (32, 4)]).all(
                            |(member, (offset, lanes))| {
                                member.offset == offset
                                    && member.binding.is_none()
                                    && vector(m, member.ty, lanes)
                            },
                        )
                } else {
                    false
                }
            }
            (1, 0) => {
                g.space == AddressSpace::Handle
                    && matches!(
                        m.types[g.ty].inner,
                        TypeInner::Image {
                            dim: naga::ImageDimension::D2,
                            arrayed: false,
                            class: naga::ImageClass::Sampled {
                                kind: K::Float,
                                multi: false
                            }
                        }
                    )
            }
            (1, 1) => {
                g.space == AddressSpace::Handle
                    && matches!(
                        m.types[g.ty].inner,
                        TypeInner::Sampler { comparison: false }
                    )
            }
            _ => false,
        };
        require(valid, "UI resource binding/type")?;
    }
    Ok(())
}
