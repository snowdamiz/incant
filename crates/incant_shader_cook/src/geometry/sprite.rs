//! Exact sprite stage IO, resource types and visibility before native emission.
use crate::{CookError, Result};
use naga::{AddressSpace, Binding, BuiltIn, ScalarKind as K, ShaderStage, TypeInner};

fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}

// Location, scalar kind, lane count, flat interpolation. Position has no location.
pub(super) type Io = (Option<u32>, K, u8, bool);
pub(super) fn io(
    m: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&Binding>,
    out: &mut Vec<Io>,
) -> Result<()> {
    let Some(binding) = binding else {
        if let TypeInner::Struct { members, .. } = &m.types[ty].inner {
            for member in members {
                io(m, member.ty, member.binding.as_ref(), out)?;
            }
            return Ok(());
        }
        return Err(CookError::Contract("sprite unbound IO"));
    };
    let (location, flat) = match binding {
        Binding::BuiltIn(BuiltIn::Position { invariant: false }) => (None, false),
        Binding::Location {
            location,
            blend_src: None,
            per_primitive: false,
            interpolation,
            sampling,
        } if sampling.is_none_or(|s| s == naga::Sampling::Center || s == naga::Sampling::First)
            && interpolation.is_none_or(|i| {
                matches!(
                    i,
                    naga::Interpolation::Perspective | naga::Interpolation::Flat
                )
            }) =>
        {
            (
                Some(*location),
                *interpolation == Some(naga::Interpolation::Flat),
            )
        }
        _ => return Err(CookError::Contract("sprite IO binding")),
    };
    let (scalar, lanes) = match m.types[ty].inner {
        TypeInner::Scalar(s) => (s, 1),
        TypeInner::Vector { size, scalar } => (scalar, size as u8),
        _ => return Err(CookError::Contract("sprite IO type")),
    };
    require(scalar.width == 4, "sprite IO width")?;
    out.push((location, scalar.kind, lanes, flat));
    Ok(())
}

pub(super) fn validate(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    require(m.overrides.is_empty(), "sprite overrides")?;
    require(m.entry_points.len() == 2, "sprite entry count")?;
    let vertex_input = vec![
        (Some(0), K::Float, 3, false),
        (Some(1), K::Float, 2, false),
        (Some(2), K::Float, 4, false),
        (Some(3), K::Float, 4, false),
        (Some(4), K::Float, 2, false),
        (Some(5), K::Uint, 1, true),
    ];
    let varying = vec![
        (None, K::Float, 4, false),
        (Some(0), K::Float, 2, false),
        (Some(1), K::Float, 4, false),
        (Some(2), K::Float, 4, true),
        (Some(3), K::Float, 2, false),
        (Some(4), K::Uint, 1, true),
        (Some(5), K::Float, 2, true),
    ];
    let color = vec![(Some(0), K::Float, 4, false)];
    let mut stages = [false; 2];
    for (index, entry) in m.entry_points.iter().enumerate() {
        let vertex = match (entry.stage, entry.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") => false,
            _ => return Err(CookError::Contract("sprite stage/name")),
        };
        let slot = usize::from(!vertex);
        require(!stages[slot], "sprite repeated stage")?;
        stages[slot] = true;
        let mut inputs = Vec::new();
        for arg in &entry.function.arguments {
            io(m, arg.ty, arg.binding.as_ref(), &mut inputs)?;
        }
        let mut outputs = Vec::new();
        if let Some(result) = &entry.function.result {
            io(m, result.ty, result.binding.as_ref(), &mut outputs)?;
        }
        require(
            inputs == *if vertex { &vertex_input } else { &varying },
            "sprite input interface",
        )?;
        require(
            outputs == *if vertex { &varying } else { &color },
            "sprite output interface",
        )?;
        for (handle, global) in m.global_variables.iter() {
            let camera = global.binding
                == Some(naga::ResourceBinding {
                    group: 0,
                    binding: 0,
                });
            require(
                info.get_entry_point(index)[handle].is_empty() != (vertex == camera),
                "sprite stage visibility",
            )?;
        }
    }
    require(m.global_variables.len() == 3, "sprite resource count")?;
    for (_, g) in m.global_variables.iter() {
        require(g.init.is_none(), "sprite resource initializer")?;
        let b = g
            .binding
            .ok_or(CookError::Contract("sprite unbound resource"))?;
        let valid = match (b.group, b.binding) {
            (0, 0) => {
                g.space == AddressSpace::Uniform
                    && matches!(
                        m.types[g.ty].inner,
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
        require(valid, "sprite resource binding/type")?;
    }
    Ok(())
}
