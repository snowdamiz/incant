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
fn vec4(module: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    matches!(
        module.types[ty].inner,
        TypeInner::Vector {
            size: VectorSize::Quad,
            scalar: naga::Scalar {
                kind: ScalarKind::Float,
                width: 4
            }
        }
    )
}

pub fn validate(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    require(m.overrides.is_empty(), "overrides")?;
    require(m.entry_points.len() == 2, "entry count")?;
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = entry.stage == ShaderStage::Vertex && entry.name == "vertex";
        let fragment = entry.stage == ShaderStage::Fragment && entry.name == "fragment";
        require(vertex || fragment, "entry stage/name")?;
        require(entry.function.arguments.len() == 1, "entry inputs")?;
        let arg = &entry.function.arguments[0];
        if vertex {
            require(
                arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex))
                    && matches!(
                        m.types[arg.ty].inner,
                        TypeInner::Scalar(naga::Scalar {
                            kind: ScalarKind::Uint,
                            width: 4
                        })
                    ),
                "vertex input",
            )?;
        } else {
            require(
                matches!(
                    arg.binding,
                    Some(Binding::BuiltIn(BuiltIn::Position { .. }))
                ) && vec4(m, arg.ty),
                "fragment input",
            )?;
        }
        let result = entry
            .function
            .result
            .as_ref()
            .ok_or(CookError::Contract("entry output"))?;
        require(vec4(m, result.ty), "entry output type")?;
        require(
            if vertex {
                matches!(
                    result.binding,
                    Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
                )
            } else {
                matches!(
                    result.binding,
                    Some(Binding::Location {
                        location: 0,
                        blend_src: None,
                        ..
                    })
                )
            },
            "entry output binding",
        )?;
        if vertex {
            require(
                m.global_variables
                    .iter()
                    .all(|(h, _)| info.get_entry_point(index)[h].is_empty()),
                "vertex resources",
            )?;
        }
    }
    require(m.global_variables.len() == 2, "global count")?;
    for (_, global) in m.global_variables.iter() {
        let binding = global
            .binding
            .ok_or(CookError::Contract("unbound global"))?;
        require(binding.group == 0, "bind group")?;
        match binding.binding {
            0 => require(
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
                "scene texture",
            )?,
            1 => {
                require(global.space == AddressSpace::Uniform, "uniform space")?;
                let TypeInner::Struct { members, span } = &m.types[global.ty].inner else {
                    return Err(CookError::Contract("uniform type"));
                };
                require(*span == 80 && members.len() == 5, "uniform size")?;
                for (i, (member, name)) in members
                    .iter()
                    .zip(["backdrop", "rect", "radii", "canvas", "pixel_grid"])
                    .enumerate()
                {
                    require(
                        member.offset == i as u32 * 16
                            && vec4(m, member.ty)
                            && member.name.as_deref() == Some(name),
                        "uniform member",
                    )?;
                }
            }
            _ => return Err(CookError::Contract("unknown binding")),
        }
    }
    Ok(())
}
