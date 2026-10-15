//! Exact SSAO host layouts and stage visibility, independent of the source pin.
use super::Kind;
use crate::{CookError, Result};
use naga::{
    AddressSpace, ArraySize, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind, ShaderStage,
    TypeInner, VectorSize,
};

fn require(ok: bool, why: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(why))
    }
}
fn vector(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    matches!(
        m.types[ty].inner,
        TypeInner::Vector {
            size: VectorSize::Quad,
            scalar: naga::Scalar {
                kind: ScalarKind::Float,
                width: 4
            }
        }
    )
}
fn scalar(m: &naga::Module, ty: naga::Handle<naga::Type>, kind: ScalarKind) -> bool {
    matches!(m.types[ty].inner,TypeInner::Scalar(naga::Scalar{kind:k,width:4})if k==kind)
}
fn uniform(m: &naga::Module, ty: naga::Handle<naga::Type>) -> Result<()> {
    let TypeInner::Struct { members, span } = &m.types[ty].inner else {
        return Err(CookError::Contract("SSAO uniform type"));
    };
    require(*span == 736 && members.len() == 7, "SSAO uniform span")?;
    for (member, (name, offset)) in members.iter().zip([
        ("projection", 0),
        ("inverse", 64),
        ("view", 128),
        ("viewport", 192),
        ("values", 208),
        ("directions", 224),
        ("rotations", 480),
    ]) {
        let shape = match offset {
            0 | 64 | 128 => matches!(
                m.types[member.ty].inner,
                TypeInner::Matrix {
                    columns: VectorSize::Quad,
                    rows: VectorSize::Quad,
                    scalar: naga::Scalar {
                        kind: ScalarKind::Float,
                        width: 4
                    }
                }
            ),
            192 | 208 => vector(m, member.ty),
            _ => {
                matches!(m.types[member.ty].inner,TypeInner::Array{base,size:ArraySize::Constant(n),stride:16}if n.get()==16 && vector(m,base))
            }
        };
        require(
            member.name.as_deref() == Some(name)
                && member.offset == offset
                && member.binding.is_none()
                && shape,
            "SSAO uniform field",
        )?;
    }
    Ok(())
}
pub(super) fn validate(kind: Kind, m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    if let Some(variant) = kind.material_variant() {
        crate::geometry::interpolation::validate(crate::geometry::Kind::Material, m)?;
        return crate::geometry::material::validate_variant(variant, m, info);
    }
    let filtered = matches!(kind, Kind::Horizontal | Kind::Vertical);
    require(
        m.overrides.is_empty() && m.entry_points.len() == if filtered { 3 } else { 2 },
        "SSAO entry/override count",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = entry.stage == ShaderStage::Vertex && entry.name == "vertex";
        require(
            vertex
                || entry.stage == ShaderStage::Fragment
                    && if filtered {
                        ["horizontal", "vertical"].contains(&entry.name.as_str())
                    } else {
                        entry.name == "fragment"
                    },
            "SSAO stage identity",
        )?;
        require(
            seen.insert(entry.name.as_str()) && entry.function.arguments.len() == 1,
            "SSAO stage inputs",
        )?;
        let arg = &entry.function.arguments[0];
        require(
            if vertex {
                arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex))
                    && scalar(m, arg.ty, ScalarKind::Uint)
            } else {
                arg.binding == Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
                    && vector(m, arg.ty)
            },
            "SSAO input ABI",
        )?;
        let result = entry
            .function
            .result
            .as_ref()
            .ok_or(CookError::Contract("SSAO output missing"))?;
        require(
            if vertex {
                result.binding == Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
                    && vector(m, result.ty)
            } else {
                scalar(m, result.ty, ScalarKind::Float)
                    && matches!(
                        result.binding,
                        Some(Binding::Location {
                            location: 0,
                            blend_src: None,
                            per_primitive: false,
                            ..
                        })
                    )
            },
            "SSAO output ABI",
        )?;
        for (handle, _) in m.global_variables.iter() {
            require(
                info.get_entry_point(index)[handle].is_empty() == vertex,
                "SSAO resource stage visibility",
            )?;
        }
    }
    require(
        m.global_variables.len() == kind.bindings() as usize,
        "SSAO resource count",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    for (_, global) in m.global_variables.iter() {
        let b = global
            .binding
            .ok_or(CookError::Contract("SSAO unbound global"))?;
        require(
            global.init.is_none()
                && b.group == 0
                && b.binding < kind.bindings()
                && seen.insert(b.binding),
            "SSAO binding identity",
        )?;
        if b.binding == 0 {
            require(
                global.space == AddressSpace::Uniform,
                "SSAO uniform address space",
            )?;
            uniform(m, global.ty)?;
        } else {
            require(
                global.space == AddressSpace::Handle
                    && m.types[global.ty].inner
                        == TypeInner::Image {
                            dim: ImageDimension::D2,
                            arrayed: false,
                            class: if b.binding == 1 {
                                ImageClass::Depth { multi: false }
                            } else {
                                ImageClass::Sampled {
                                    kind: ScalarKind::Float,
                                    multi: false,
                                }
                            },
                        },
                "SSAO texture ABI",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../incant_render/src/ambient_occlusion/source.rs"]
mod source;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_ir_with_changed_uniform_or_resource_layout_refuses_without_source_pin() {
        let source = include_str!("../../../incant_render/src/ambient_occlusion/ao.wgsl");
        for changed in [
            source.replace("@binding(2)", "@binding(3)"),
            source.replace("@group(0)", "@group(1)"),
            source.replace("array<vec4f,16>", "array<vec4f,17>"),
            source
                .replace("viewport:vec4f", "bounds:vec4f")
                .replace("ao.viewport", "ao.bounds"),
            format!("{source}\n@group(0) @binding(4) var extra:texture_2d<f32>;"),
            source.replace("fn fragment(", "fn other("),
        ] {
            assert_ne!(changed, source);
            let m = naga::front::wgsl::parse_str(&changed).unwrap();
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&m)
            .unwrap();
            assert!(validate(Kind::Sample, &m, &info).is_err());
        }
    }
    #[test]
    fn material_variants_refuse_valid_ir_with_changed_bindings_or_interpolation() {
        for (kind, prepass) in [(Kind::Prepass, true), (Kind::Forward, false)] {
            let original = source::material(prepass).unwrap();
            let mut changed = vec![
                original.replace("@group(1) @binding(1)", "@group(1) @binding(11)"),
                original.replace(
                    "@interpolate(perspective, centroid)",
                    "@interpolate(perspective, center)",
                ),
                original
                    .replace("flags:vec4u", "options:vec4u")
                    .replace("material.flags", "material.options"),
            ];
            if !prepass {
                changed.push(original.replace("@group(0) @binding(1)", "@group(0) @binding(2)"));
            }
            for text in changed {
                assert_ne!(text, original);
                let m = naga::front::wgsl::parse_str(&text).unwrap();
                let info = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::empty(),
                )
                .validate(&m)
                .unwrap();
                assert!(validate(kind, &m, &info).is_err());
            }
        }
    }
}
