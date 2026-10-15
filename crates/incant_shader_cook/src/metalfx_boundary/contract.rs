use super::Kind;
use crate::{CookError, Result};
use naga::{
    AddressSpace, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind, ShaderStage, TypeInner,
};

fn require(ok: bool, why: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(why))
    }
}
fn numeric(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    lanes: u8,
    kind: ScalarKind,
) -> bool {
    let scalar = naga::Scalar { kind, width: 4 };
    match module.types[ty].inner {
        TypeInner::Scalar(actual) => lanes == 1 && actual == scalar,
        TypeInner::Vector {
            size,
            scalar: actual,
        } => size as u8 == lanes && actual == scalar,
        _ => false,
    }
}
fn position(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&Binding>,
) -> bool {
    matches!(
        binding,
        Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
    ) && numeric(module, ty, 4, ScalarKind::Float)
}
pub(super) fn validate(
    kind: Kind,
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
) -> Result<()> {
    require(
        module.overrides.is_empty() && module.entry_points.len() == 2,
        "MetalFX boundary entry/override count",
    )?;
    let mut seen = [false; 2];
    for (index, entry) in module.entry_points.iter().enumerate() {
        let vertex = match (entry.stage, entry.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, name) if name == kind.entry() => false,
            _ => return Err(CookError::Contract("MetalFX boundary entry identity")),
        };
        let slot = usize::from(!vertex);
        require(
            !seen[slot] && entry.function.arguments.len() == 1,
            "MetalFX boundary input count",
        )?;
        seen[slot] = true;
        let arg = &entry.function.arguments[0];
        require(
            if vertex {
                arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex))
                    && numeric(module, arg.ty, 1, ScalarKind::Uint)
            } else {
                position(module, arg.ty, arg.binding.as_ref())
            },
            "MetalFX boundary input ABI",
        )?;
        let result = entry
            .function
            .result
            .as_ref()
            .ok_or(CookError::Contract("MetalFX boundary missing output"))?;
        if vertex {
            require(
                position(module, result.ty, result.binding.as_ref()),
                "MetalFX boundary vertex output",
            )?;
        } else {
            require(result.binding.is_none(), "MetalFX boundary output struct")?;
            let TypeInner::Struct { members, .. } = &module.types[result.ty].inner else {
                return Err(CookError::Contract("MetalFX boundary output type"));
            };
            let lanes: &[u8] = match kind {
                Kind::Pack => &[4, 1, 2, 1],
                Kind::Normalize => &[4],
            };
            require(members.len() == lanes.len(), "MetalFX boundary MRT count")?;
            for (location, (member, &lanes)) in members.iter().zip(lanes).enumerate() {
                require(
                    matches!(member.binding, Some(Binding::Location { location: actual, blend_src: None, per_primitive: false, .. }) if actual == location as u32)
                        && numeric(module, member.ty, lanes, ScalarKind::Float),
                    "MetalFX boundary MRT ABI",
                )?;
            }
        }
        for (handle, _) in module.global_variables.iter() {
            require(
                info.get_entry_point(index)[handle].is_empty() == vertex,
                "MetalFX boundary stage visibility",
            )?;
        }
    }
    require(
        seen == [true; 2] && module.global_variables.len() == 4,
        "MetalFX boundary resource count",
    )?;
    let mut bindings = [false; 4];
    for (_, global) in module.global_variables.iter() {
        let binding = global
            .binding
            .ok_or(CookError::Contract("MetalFX boundary unbound global"))?;
        require(
            global.init.is_none()
                && binding.group == 0
                && binding.binding < 4
                && !bindings[binding.binding as usize],
            "MetalFX boundary binding identity",
        )?;
        bindings[binding.binding as usize] = true;
        if binding.binding == 0 {
            require(
                global.space == AddressSpace::Uniform,
                "MetalFX boundary uniform space",
            )?;
            let TypeInner::Struct { members, span } = &module.types[global.ty].inner else {
                return Err(CookError::Contract("MetalFX boundary uniform type"));
            };
            require(
                *span == 96 && members.len() == 6,
                "MetalFX boundary uniform span",
            )?;
            for (index, (member, name)) in members
                .iter()
                .zip(["source", "output", "depth", "options", "reactive", "jitter"])
                .enumerate()
            {
                require(
                    member.name.as_deref() == Some(name)
                        && member.offset == index as u32 * 16
                        && numeric(module, member.ty, 4, ScalarKind::Float),
                    "MetalFX boundary uniform member",
                )?;
            }
        } else {
            let class = if binding.binding == 3 {
                ImageClass::Depth { multi: false }
            } else {
                ImageClass::Sampled {
                    kind: ScalarKind::Float,
                    multi: false,
                }
            };
            require(
                global.space == AddressSpace::Handle
                    && matches!(module.types[global.ty].inner,
                TypeInner::Image { dim: ImageDimension::D2, arrayed: false, class: actual } if actual == class),
                "MetalFX boundary texture ABI",
            )?;
        }
    }
    require(bindings == [true; 4], "MetalFX boundary missing binding")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_ir_with_changed_binding_frame_entry_or_mrt_refuses_without_source_pin() {
        for kind in [Kind::Pack, Kind::Normalize] {
            let source = crate::metalfx_boundary::source_for_test(kind);
            for changed in [
                source.replace("@binding(3)", "@binding(4)"),
                source.replace("@group(0)", "@group(1)"),
                source.replace(
                    "reactive:vec4f, jitter:vec4f",
                    "reactive:vec4f, jitter:vec4f, extra:vec4f",
                ),
                source.replace(&format!("fn {}(", kind.entry()), "fn other("),
                source.replace("@location(0) color:vec4f", "@location(4) color:vec4f"),
                format!("{source}\n@group(0) @binding(4) var extra:texture_2d<f32>;"),
            ] {
                assert_ne!(source, changed);
                let module = naga::front::wgsl::parse_str(&changed).unwrap();
                let info = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::empty(),
                )
                .validate(&module)
                .unwrap();
                assert!(validate(kind, &module, &info).is_err());
            }
        }
    }
}
