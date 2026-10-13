use super::Kind;
use crate::{CookError, Result};
use naga::{
    AddressSpace, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind, ShaderStage, TypeInner,
    VectorSize,
};

fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}
fn float(m: &naga::Module, ty: naga::Handle<naga::Type>, lanes: u8) -> bool {
    let scalar = naga::Scalar {
        kind: ScalarKind::Float,
        width: 4,
    };
    match m.types[ty].inner {
        TypeInner::Scalar(s) => lanes == 1 && s == scalar,
        TypeInner::Vector { size, scalar: s } => size as u8 == lanes && s == scalar,
        _ => false,
    }
}
// None denotes the single Position builtin, Some(n) a user location.
type Io = (Option<u32>, u8);
fn io(
    m: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&Binding>,
    out: &mut Vec<Io>,
) -> Result<()> {
    if let Some(binding) = binding {
        let location = match binding {
            Binding::BuiltIn(BuiltIn::Position { invariant: false }) => None,
            Binding::Location {
                location,
                blend_src: None,
                per_primitive: false,
                interpolation,
                sampling,
            } if interpolation.is_none_or(|i| i == naga::Interpolation::Perspective)
                && sampling.is_none_or(|s| s == naga::Sampling::Center) =>
            {
                Some(*location)
            }
            _ => return Err(CookError::Contract("geometry IO binding")),
        };
        let lanes = (1..=4)
            .find(|n| float(m, ty, *n))
            .ok_or(CookError::Contract("geometry IO type"))?;
        out.push((location, lanes));
    } else if let TypeInner::Struct { members, .. } = &m.types[ty].inner {
        for member in members {
            io(m, member.ty, member.binding.as_ref(), out)?;
        }
    } else {
        return Err(CookError::Contract("geometry unbound IO"));
    }
    Ok(())
}

pub(super) fn validate(kind: Kind, m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    if kind == Kind::Text {
        return super::text::validate(m, info);
    }
    if kind == Kind::Ui {
        return super::ui::validate(m, info);
    }
    if kind == Kind::Sprite {
        return super::sprite::validate(m, info);
    }
    require(m.overrides.is_empty(), "geometry overrides")?;
    require(m.entry_points.len() == 2, "geometry entry count")?;
    let varying = if kind == Kind::Diagnostic { 1 } else { 2 };
    let mut stages = [false; 2];
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = match (entry.stage, entry.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") => false,
            _ => return Err(CookError::Contract("geometry stage/name")),
        };
        let slot = usize::from(!vertex);
        require(!stages[slot], "geometry repeated stage")?;
        stages[slot] = true;
        let mut inputs = Vec::new();
        for arg in &entry.function.arguments {
            io(m, arg.ty, arg.binding.as_ref(), &mut inputs)?;
        }
        let expected = if vertex {
            match kind {
                Kind::Diagnostic => vec![(Some(0), 4), (Some(1), 3)],
                Kind::Shadow => vec![
                    (Some(0), 3),
                    (Some(2), 4),
                    (Some(3), 4),
                    (Some(4), 4),
                    (Some(5), 4),
                    (Some(9), 2),
                ],
                Kind::Sprite | Kind::Ui | Kind::Text => {
                    unreachable!("textured 2D contract validated separately")
                }
            }
        } else {
            vec![(None, 4), (Some(0), varying)]
        };
        require(inputs == expected, "geometry input interface")?;
        let mut outputs = Vec::new();
        if let Some(result) = &entry.function.result {
            io(m, result.ty, result.binding.as_ref(), &mut outputs)?;
        }
        let expected = if vertex {
            vec![(None, 4), (Some(0), varying)]
        } else if kind == Kind::Diagnostic {
            vec![(Some(0), 4)]
        } else {
            vec![]
        };
        require(outputs == expected, "geometry output interface")?;
        for (handle, global) in m.global_variables.iter() {
            let camera = global.binding
                == Some(naga::ResourceBinding {
                    group: 0,
                    binding: 0,
                });
            require(
                info.get_entry_point(index)[handle].is_empty() != (vertex == camera),
                "geometry stage visibility",
            )?;
        }
    }
    if kind == Kind::Diagnostic {
        return require(m.global_variables.is_empty(), "diagnostic resources");
    }
    require(m.global_variables.len() == 4, "shadow resource count")?;
    for (_, global) in m.global_variables.iter() {
        require(global.init.is_none(), "shadow resource initializer")?;
        let binding = global
            .binding
            .ok_or(CookError::Contract("shadow unbound resource"))?;
        match (binding.group, binding.binding) {
            (0, 0) => require(
                global.space == AddressSpace::Uniform
                    && matches!(
                        m.types[global.ty].inner,
                        TypeInner::Matrix {
                            columns: VectorSize::Quad,
                            rows: VectorSize::Quad,
                            scalar: naga::Scalar {
                                kind: ScalarKind::Float,
                                width: 4
                            }
                        }
                    ),
                "shadow camera",
            )?,
            (1, 0) => {
                require(
                    global.space == AddressSpace::Uniform,
                    "shadow material address space",
                )?;
                let TypeInner::Struct { members, span } = &m.types[global.ty].inner else {
                    return Err(CookError::Contract("shadow material type"));
                };
                require(*span == 64 && members.len() == 4, "shadow material size")?;
                for (i, (member, name)) in members
                    .iter()
                    .zip(["base", "emissive_roughness", "factors", "flags"])
                    .enumerate()
                {
                    require(
                        member.offset == i as u32 * 16 && member.name.as_deref() == Some(name),
                        "shadow material member",
                    )?;
                    require(
                        if i == 3 {
                            matches!(
                                m.types[member.ty].inner,
                                TypeInner::Vector {
                                    size: VectorSize::Quad,
                                    scalar: naga::Scalar {
                                        kind: ScalarKind::Uint,
                                        width: 4
                                    }
                                }
                            )
                        } else {
                            float(m, member.ty, 4)
                        },
                        "shadow material field type",
                    )?;
                }
            }
            (1, 1) => require(
                global.space == AddressSpace::Handle
                    && matches!(
                        m.types[global.ty].inner,
                        TypeInner::Image {
                            dim: ImageDimension::D2,
                            arrayed: false,
                            class: ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false
                            }
                        }
                    ),
                "shadow texture",
            )?,
            (1, 2) => require(
                global.space == AddressSpace::Handle
                    && matches!(
                        m.types[global.ty].inner,
                        TypeInner::Sampler { comparison: false }
                    ),
                "shadow sampler",
            )?,
            _ => return Err(CookError::Contract("shadow unknown resource")),
        }
    }
    Ok(())
}
