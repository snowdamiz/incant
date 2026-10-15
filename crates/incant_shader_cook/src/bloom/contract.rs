use super::Kind;
use crate::{CookError, Result};
use naga::{
    AddressSpace, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind, ShaderStage, TypeInner,
    VectorSize,
};

fn require(ok: bool, why: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(why))
    }
}
fn vec4(m: &naga::Module, ty: naga::Handle<naga::Type>, kind: ScalarKind) -> bool {
    matches!(m.types[ty].inner, TypeInner::Vector { size: VectorSize::Quad, scalar: naga::Scalar { kind: k, width: 4 } } if k == kind)
}
fn uniform(
    m: &naga::Module,
    ty: naga::Handle<naga::Type>,
    names: &[&str],
    sizes: bool,
) -> Result<()> {
    let TypeInner::Struct { members, span } = &m.types[ty].inner else {
        return Err(CookError::Contract("bloom uniform type"));
    };
    require(
        *span == names.len() as u32 * 16 && members.len() == names.len(),
        "bloom uniform span",
    )?;
    for (i, (member, name)) in members.iter().zip(names).enumerate() {
        require(
            member.name.as_deref() == Some(*name)
                && member.offset == i as u32 * 16
                && vec4(
                    m,
                    member.ty,
                    if sizes && i == 1 {
                        ScalarKind::Uint
                    } else {
                        ScalarKind::Float
                    },
                ),
            "bloom uniform member",
        )?;
    }
    Ok(())
}
pub(super) fn validate(kind: Kind, m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    let display = kind == Kind::Display;
    require(
        m.overrides.is_empty() && m.entry_points.len() == if display { 2 } else { 4 },
        "bloom entry/override count",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = entry.stage == ShaderStage::Vertex && entry.name == "vertex";
        require(
            vertex
                || entry.stage == ShaderStage::Fragment
                    && if display {
                        entry.name == "fragment"
                    } else {
                        ["first", "down", "up"].contains(&entry.name.as_str())
                    },
            "bloom entry identity",
        )?;
        require(
            seen.insert(entry.name.as_str()) && entry.function.arguments.len() == 1,
            "bloom entry inputs",
        )?;
        let arg = &entry.function.arguments[0];
        require(
            if vertex {
                arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex))
                    && matches!(
                        m.types[arg.ty].inner,
                        TypeInner::Scalar(naga::Scalar {
                            kind: ScalarKind::Uint,
                            width: 4
                        })
                    )
            } else {
                matches!(
                    arg.binding,
                    Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
                ) && vec4(m, arg.ty, ScalarKind::Float)
            },
            "bloom input ABI",
        )?;
        let result = entry
            .function
            .result
            .as_ref()
            .ok_or(CookError::Contract("bloom missing output"))?;
        require(
            vec4(m, result.ty, ScalarKind::Float)
                && if vertex {
                    result.binding == Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
                } else {
                    matches!(
                        result.binding,
                        Some(Binding::Location {
                            location: 0,
                            blend_src: None,
                            per_primitive: false,
                            ..
                        })
                    )
                },
            "bloom output ABI",
        )?;
        for (handle, global) in m.global_variables.iter() {
            let expected = !vertex
                && (display
                    || global.binding.is_some_and(|b| b.binding != 1)
                    || entry.name == "up");
            require(
                info.get_entry_point(index)[handle].is_empty() != expected,
                "bloom stage visibility",
            )?;
        }
    }
    require(
        m.global_variables.len() == kind.bindings() as usize,
        "bloom resource count",
    )?;
    let mut bindings = std::collections::BTreeSet::new();
    for (_, global) in m.global_variables.iter() {
        let binding = global
            .binding
            .ok_or(CookError::Contract("bloom unbound global"))?;
        require(
            global.init.is_none() && binding.group == 0 && bindings.insert(binding.binding),
            "bloom binding identity",
        )?;
        let slot = binding.binding;
        if kind.texture(slot).is_some() {
            require(
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
                "bloom texture ABI",
            )?;
        } else if kind.uniform(slot).is_some() {
            require(global.space == AddressSpace::Uniform, "bloom uniform space")?;
            if display && slot == 3 {
                require(vec4(m, global.ty, ScalarKind::Float), "bloom gain ABI")?;
            } else if display {
                uniform(
                    m,
                    global.ty,
                    &["backdrop", "rect", "radii", "canvas", "pixel_grid"],
                    false,
                )?;
            } else {
                uniform(m, global.ty, &["rect", "sizes", "params"], true)?;
            }
        } else {
            return Err(CookError::Contract("bloom unknown binding"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_ir_with_changed_layout_or_entry_contract_refuses() {
        let source = include_str!("../../../incant_render/src/bloom/filter.wgsl");
        for changed in [
            source.replace("@binding(2)", "@binding(3)"),
            source.replace("@group(0)", "@group(1)"),
            source.replace("fn down(", "fn other("),
            source.replace("sizes: vec4u", "sizes: vec4u, extra: vec4f"),
            format!("{source}\n@group(0) @binding(4) var extra: texture_2d<f32>;"),
            source
                .replace("sizes: vec4u", "dimensions: vec4u")
                .replace("settings.sizes", "settings.dimensions"),
        ] {
            assert_ne!(changed, source);
            let module = naga::front::wgsl::parse_str(&changed).unwrap();
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap();
            assert!(validate(Kind::Prefilter, &module, &info).is_err());
        }
    }

    #[test]
    fn display_extra_resource_and_gain_type_refuse_independently_of_source_pin() {
        let source = [
            include_str!("../../../incant_render/src/output.wgsl").replace(
                "var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);",
                "var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }",
            ),
            include_str!("../../../incant_render/src/tone_map.wgsl").to_owned(),
            include_str!("../../../incant_render/src/bloom/output.wgsl").to_owned(),
        ].join("\n");
        for changed in [source.replace("@binding(3)", "@binding(4)"),
            source.replace("backdrop: vec4f", "backdrop: vec4f, extra: vec4f"),
            format!("{source}\n@group(0) @binding(4) var extra: texture_2d<f32>;"),
            source.replace("@group(0) @binding(3) var<uniform> bloom_gain: vec4f;", "struct Gain { value: vec4f }\n@group(0) @binding(3) var<uniform> changed_gain: Gain;").replace("bloom_gain.", "changed_gain.value.")
        ] {
            assert_ne!(changed, source);
            let module = naga::front::wgsl::parse_str(&changed).unwrap();
            let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty()).validate(&module).unwrap();
            assert!(validate(Kind::Display, &module, &info).is_err());
        }
    }
}
