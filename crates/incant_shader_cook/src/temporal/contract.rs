use super::Kind;
use crate::{CookError, Result};
use naga::{
    AddressSpace, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind, ShaderStage, TypeInner,
};
fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}
fn numeric(m: &naga::Module, t: naga::Handle<naga::Type>, lanes: u8, kind: ScalarKind) -> bool {
    let scalar = naga::Scalar { kind, width: 4 };
    match m.types[t].inner {
        TypeInner::Scalar(s) => lanes == 1 && s == scalar,
        TypeInner::Vector { size, scalar: s } => size as u8 == lanes && s == scalar,
        _ => false,
    }
}
#[derive(Debug, PartialEq)]
enum Io {
    Position,
    Index,
    Float(u32, u8, bool),
    Uint(u32, u8, bool),
}
fn io(
    m: &naga::Module,
    t: naga::Handle<naga::Type>,
    b: Option<&Binding>,
    out: &mut Vec<Io>,
    interstage: bool,
) -> Result<()> {
    match b {
        Some(Binding::BuiltIn(BuiltIn::Position { invariant: false }))
            if numeric(m, t, 4, ScalarKind::Float) =>
        {
            out.push(Io::Position)
        }
        Some(Binding::BuiltIn(BuiltIn::VertexIndex)) if numeric(m, t, 1, ScalarKind::Uint) => {
            out.push(Io::Index)
        }
        Some(Binding::Location {
            location,
            blend_src: None,
            per_primitive: false,
            interpolation,
            sampling,
        }) if interpolation.is_none_or(|i| {
            matches!(
                i,
                naga::Interpolation::Perspective | naga::Interpolation::Flat
            )
        }) && sampling
            .is_none_or(|s| s == naga::Sampling::Center || s == naga::Sampling::First) =>
        {
            let uint = (1..=4).any(|n| numeric(m, t, n, ScalarKind::Uint));
            let n = (1..=4)
                .find(|n| {
                    numeric(
                        m,
                        t,
                        *n,
                        if uint {
                            ScalarKind::Uint
                        } else {
                            ScalarKind::Float
                        },
                    )
                })
                .ok_or(CookError::Contract("temporal IO type"))?;
            let flat = *interpolation == Some(naga::Interpolation::Flat);
            require(
                sampling.is_none_or(|s| {
                    s == if flat {
                        naga::Sampling::First
                    } else {
                        naga::Sampling::Center
                    }
                }),
                "temporal IO sampling",
            )?;
            if uint {
                require(
                    flat && if interstage {
                        *sampling == Some(naga::Sampling::First)
                    } else {
                        sampling.is_none()
                    },
                    "temporal integer interpolation",
                )?;
                out.push(Io::Uint(*location, n, interstage));
            } else {
                out.push(Io::Float(*location, n, flat));
            }
        }
        None => {
            if let TypeInner::Struct { members, .. } = &m.types[t].inner {
                for x in members {
                    io(m, x.ty, x.binding.as_ref(), out, interstage)?;
                }
            } else {
                return Err(CookError::Contract("temporal unbound IO"));
            }
        }
        _ => return Err(CookError::Contract("temporal IO binding")),
    }
    Ok(())
}
fn uniform(
    m: &naga::Module,
    t: naga::Handle<naga::Type>,
    names: &[&str],
    motion: bool,
    material: bool,
) -> Result<()> {
    let TypeInner::Struct { members, span } = &m.types[t].inner else {
        return Err(CookError::Contract("temporal uniform type"));
    };
    require(
        members.len() == names.len() && *span == if motion { 144 } else { names.len() as u32 * 16 },
        "temporal uniform span",
    )?;
    for (i, (member, name)) in members.iter().zip(names).enumerate() {
        let matrix = motion && i < 2;
        require(
            member.name.as_deref() == Some(*name)
                && member.offset == if motion { i as u32 * 64 } else { i as u32 * 16 },
            "temporal uniform member",
        )?;
        require(
            if matrix {
                matches!(
                    m.types[member.ty].inner,
                    TypeInner::Matrix {
                        columns: naga::VectorSize::Quad,
                        rows: naga::VectorSize::Quad,
                        scalar: naga::Scalar {
                            kind: ScalarKind::Float,
                            width: 4
                        }
                    }
                )
            } else {
                numeric(
                    m,
                    member.ty,
                    4,
                    if material && i == 3 {
                        ScalarKind::Uint
                    } else {
                        ScalarKind::Float
                    },
                )
            },
            "temporal uniform member type",
        )?;
    }
    Ok(())
}
pub(super) fn validate(kind: Kind, m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    let motion = matches!(kind, Kind::Motion | Kind::Depth | Kind::MotionOwner);
    let owner = kind == Kind::MotionOwner;
    require(
        m.overrides.is_empty() && m.entry_points.len() == if motion && !owner { 3 } else { 2 },
        "temporal entry/override count",
    )?;
    let vary = || {
        let mut result = vec![
            Io::Position,
            Io::Float(0, 4, false),
            Io::Float(1, 2, false),
            Io::Float(2, 1, true),
        ];
        if owner {
            result.push(Io::Uint(3, 1, true));
        }
        result
    };
    let mut seen = [false; 3];
    for (n, e) in m.entry_points.iter().enumerate() {
        let depth_only = motion && !owner && e.name == "depth_only";
        let vertex = match (e.stage, e.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") if motion => false,
            (ShaderStage::Fragment, "depth_only") if motion && !owner => false,
            (ShaderStage::Fragment, name) if name == kind.entry() => false,
            _ => return Err(CookError::Contract("temporal entry identity")),
        };
        let slot = if depth_only { 2 } else { usize::from(!vertex) };
        require(!seen[slot], "temporal duplicate stage")?;
        seen[slot] = true;
        let mut inputs = Vec::new();
        for a in &e.function.arguments {
            io(m, a.ty, a.binding.as_ref(), &mut inputs, !vertex)?;
        }
        let expected = if vertex && motion {
            (0..if owner { 13 } else { 12 })
                .map(|i| {
                    if owner && i == 12 {
                        return Io::Uint(12, 1, false);
                    }
                    if owner && i == 11 {
                        return Io::Float(11, 1, false);
                    }
                    Io::Float(
                        i,
                        match i {
                            0 | 2 => 3,
                            1 => 2,
                            _ => 4,
                        },
                        false,
                    )
                })
                .collect()
        } else if vertex {
            vec![Io::Index]
        } else if motion {
            vary()
        } else {
            vec![Io::Position]
        };
        require(inputs == expected, "temporal inputs")?;
        let mut outputs = Vec::new();
        if let Some(result) = &e.function.result {
            require(!depth_only, "temporal depth fragment output")?;
            io(m, result.ty, result.binding.as_ref(), &mut outputs, vertex)?;
        } else {
            require(depth_only, "temporal missing output")?;
        }
        let expected = if vertex && motion {
            vary()
        } else if vertex {
            vec![Io::Position]
        } else if depth_only {
            vec![]
        } else if owner {
            vec![Io::Float(0, 4, false), Io::Uint(1, 1, false)]
        } else {
            vec![Io::Float(0, 4, false)]
        };
        require(outputs == expected, "temporal outputs/MRT")?;
        for (h, g) in m.global_variables.iter() {
            let expected = (!vertex && !(depth_only && g.binding.is_some_and(|b| b.group == 0)))
                || (vertex
                    && motion
                    && g.binding
                        == Some(naga::ResourceBinding {
                            group: 0,
                            binding: 0,
                        }));
            require(
                info.get_entry_point(n)[h].is_empty() != expected,
                "temporal stage visibility",
            )?;
        }
    }
    let count = match kind {
        Kind::Motion | Kind::Depth | Kind::MotionOwner => 4,
        Kind::Resolve => 5,
        Kind::Copy => 2,
    };
    require(m.global_variables.len() == count, "temporal resource count")?;
    let mut bindings = std::collections::BTreeSet::new();
    for (_, g) in m.global_variables.iter() {
        require(g.init.is_none(), "temporal initialized global")?;
        let b = g
            .binding
            .ok_or(CookError::Contract("temporal unbound global"))?;
        require(
            bindings.insert((b.group, b.binding)),
            "temporal duplicate binding",
        )?;
        let frame = b.group == 0 && b.binding == u32::from(kind == Kind::Copy);
        let material = motion && b.group == 1 && b.binding == 0;
        if frame || material {
            require(g.space == AddressSpace::Uniform, "temporal uniform space")?;
            let names: &[&str] = if material {
                &["base", "emissive_roughness", "factors", "flags"]
            } else {
                match kind {
                    Kind::Motion | Kind::Depth | Kind::MotionOwner => {
                        &["current", "previous", "rect"]
                    }
                    Kind::Resolve | Kind::Copy => &["rect", "depth", "options", "reactive"],
                }
            };
            uniform(m, g.ty, names, frame && motion, material)?;
        } else if motion && b.group == 1 && b.binding == 2 {
            require(
                g.space == AddressSpace::Handle
                    && matches!(
                        m.types[g.ty].inner,
                        TypeInner::Sampler { comparison: false }
                    ),
                "temporal sampler",
            )?;
        } else {
            require(
                if motion {
                    b.group == 1 && b.binding == 1
                } else {
                    b.group == 0 && b.binding < count as u32
                },
                "temporal texture binding",
            )?;
            let class = if b.binding == 3 && !motion {
                ImageClass::Depth { multi: false }
            } else {
                ImageClass::Sampled {
                    kind: ScalarKind::Float,
                    multi: false,
                }
            };
            require(
                g.space == AddressSpace::Handle
                    && matches!(m.types[g.ty].inner,TypeInner::Image{dim:ImageDimension::D2,arrayed:false,class:c} if c==class),
                "temporal texture type",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn refuses_valid_change(kind: Kind, source: &str, old: &str, new: &str) {
        let changed = source.replace(old, new);
        assert_ne!(
            changed, source,
            "negative control must actually change source"
        );
        let m = naga::front::wgsl::parse_str(&changed).unwrap();
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
        assert!(validate(kind, &m, &info).is_err());
    }
    #[test]
    fn fixed_copy_and_depth_only_interfaces_fail_closed() {
        let motion = include_str!("../../../incant_render/src/motion/shader.wgsl");
        refuses_valid_change(
            Kind::Depth,
            motion,
            "@fragment fn depth_only(in:Output) {mask(in);}",
            "@fragment fn depth_only(in:Output)->@location(0) vec4f {mask(in);return vec4f(0.0);}",
        );
        let copy = include_str!("../../../incant_render/src/temporal/copy.wgsl");
        refuses_valid_change(Kind::Copy, copy, "@binding(1)", "@binding(2)");
    }
    #[test]
    fn structurally_valid_ir_changes_are_refused_independently_of_source_hash() {
        let source = include_str!("../../../incant_render/src/motion/shader.wgsl");
        for changed in [
            source.replace("@location(2) @interpolate(flat)", "@location(2)"),
            source.replace("@binding(2)", "@binding(3)"),
            source.replace(
                "current:mat4x4f, previous:mat4x4f, rect:vec4f",
                "previous:mat4x4f, current:mat4x4f, rect:vec4f",
            ),
            source.replace("@location(11)", "@location(12)"),
        ] {
            let m = naga::front::wgsl::parse_str(&changed).unwrap();
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&m)
            .unwrap();
            assert!(validate(Kind::Motion, &m, &info).is_err());
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    #[test]
    fn integer_owner_contract_rejects_float_missing_nonflat_and_extra_resources() {
        let source = include_str!("../../../incant_render/src/motion/owner.wgsl");
        let validate_source = |source: &str| {
            let module = naga::front::wgsl::parse_str(source).unwrap();
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module);
            info.is_ok_and(|info| validate(Kind::MotionOwner, &module, &info).is_ok())
        };
        assert!(validate_source(source));
        for changed in [
            source.replace("owner:u32", "owner:f32"),
            source.replace("@location(12)", "@location(13)"),
            source
                .replace("@location(11) valid:f32", "@location(11) valid:vec4f")
                .replace("out.valid=valid;", "out.valid=valid.x;"),
            source.replace(
                "@location(3) @interpolate(flat, first)",
                "@location(3) @interpolate(perspective)",
            ),
            source.replace("@location(1) owner:u32", "@location(2) owner:u32"),
            format!("{source}\n@group(2) @binding(0) var extra:texture_2d<f32>;"),
        ] {
            assert_ne!(changed, source);
            assert!(!validate_source(&changed));
        }
        assert!(super::super::cook(Kind::MotionOwner, &format!("{source}\n")).is_err());
        assert!(super::super::cook(Kind::Motion, source).is_err());
    }
    #[test]
    fn owner_math_and_mask_are_the_unchanged_motion_equations() {
        let old = include_str!("../../../incant_render/src/motion/shader.wgsl");
        let owner = include_str!("../../../incant_render/src/motion/owner.wgsl");
        let body = |s: &str, start: &str| {
            s.split_once(start)
                .unwrap()
                .1
                .split_once("\n}")
                .unwrap()
                .0
                .to_owned()
        };
        assert_eq!(
            body(old, "fn mask(in:Output) {"),
            body(owner, "fn mask(in:Output) {")
        );
        assert_eq!(
            body(old, "    // Comparisons also"),
            body(owner, "    // Comparisons also")
        );
        assert!(owner.contains("return Surface(motion(in),in.owner);"));
        assert!(!owner.contains("fn depth_only"));
        assert!(old.contains("@location(11) valid:vec4f"));
        assert_eq!(
            super::super::Kind::Motion.source_hash(),
            super::super::Kind::Depth.source_hash()
        );
    }
}
