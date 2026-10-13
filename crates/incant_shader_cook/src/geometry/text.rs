//! Exact world-space text IO and resources; themes and shader math remain unchanged.
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
    require(m.overrides.is_empty(), "text overrides")?;
    require(m.entry_points.len() == 2, "text entry count")?;
    let quad = vec![(Some(0), K::Float, 4, false), (Some(1), K::Float, 4, false)];
    let varying = vec![
        (None, K::Float, 4, false),
        (Some(0), K::Float, 2, false),
        (Some(1), K::Float, 4, true),
    ];
    let color = vec![(Some(0), K::Float, 4, false)];
    let mut stages = [false; 2];
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = match (entry.stage, entry.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") => false,
            _ => return Err(CookError::Contract("text stage/name")),
        };
        let slot = usize::from(!vertex);
        require(!stages[slot], "text repeated stage")?;
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
                    "text vertex index",
                )?;
                indices += 1;
            } else {
                io(m, arg.ty, arg.binding.as_ref(), &mut inputs)?;
            }
        }
        require(indices == usize::from(vertex), "text vertex index count")?;
        let mut outputs = Vec::new();
        if let Some(result) = &entry.function.result {
            io(m, result.ty, result.binding.as_ref(), &mut outputs)?;
        }
        require(
            inputs == *if vertex { &quad } else { &varying },
            "text input interface",
        )?;
        require(
            outputs == *if vertex { &varying } else { &color },
            "text output interface",
        )?;
        for (handle, global) in m.global_variables.iter() {
            let binding = global.binding.map(|b| (b.group, b.binding));
            let used = binding == Some((0, 0))
                || (vertex && binding == Some((1, 0)))
                || (!vertex && matches!(binding, Some((2, 0) | (2, 1))));
            require(
                info.get_entry_point(index)[handle].is_empty() != used,
                "text stage visibility",
            )?;
        }
    }
    require(m.global_variables.len() == 4, "text resource count")?;
    for (_, g) in m.global_variables.iter() {
        require(g.init.is_none(), "text resource initializer")?;
        let b = g
            .binding
            .ok_or(CookError::Contract("text unbound resource"))?;
        let valid = match (b.group, b.binding) {
            (0, 0) => {
                if let TypeInner::Struct { members, span } = &m.types[g.ty].inner {
                    g.space == AddressSpace::Uniform
                        && *span == 80
                        && members.len() == 2
                        && members[0].offset == 0
                        && members[0].binding.is_none()
                        && matrix(m, members[0].ty)
                        && members[1].offset == 64
                        && members[1].binding.is_none()
                        && vector(m, members[1].ty, 4)
                } else {
                    false
                }
            }
            (1, 0) => g.space == AddressSpace::Uniform && matrix(m, g.ty),
            (2, 0) => {
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
            (2, 1) => {
                g.space == AddressSpace::Handle
                    && matches!(
                        m.types[g.ty].inner,
                        TypeInner::Sampler { comparison: false }
                    )
            }
            _ => false,
        };
        require(valid, "text resource binding/type")?;
    }
    Ok(())
}

fn matrix(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    matches!(
        m.types[ty].inner,
        TypeInner::Matrix {
            columns: naga::VectorSize::Quad,
            rows: naga::VectorSize::Quad,
            scalar: naga::Scalar {
                kind: K::Float,
                width: 4
            }
        }
    )
}
