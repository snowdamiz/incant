//! Fixed imported-material ABI. This validates resources, not appearance.
use super::sprite::io;
use crate::{CookError, Result};
use naga::{
    AddressSpace, ArraySize, Binding, BuiltIn, ImageClass, ImageDimension, ScalarKind as K,
    ShaderStage, TypeInner,
};

fn require(value: bool, reason: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}
fn vector(m: &naga::Module, ty: naga::Handle<naga::Type>, kind: K) -> bool {
    matches!(m.types[ty].inner, TypeInner::Vector { size: naga::VectorSize::Quad, scalar } if scalar.kind == kind && scalar.width == 4)
}
fn matrix(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    matches!(m.types[ty].inner, TypeInner::Matrix { columns: naga::VectorSize::Quad, rows: naga::VectorSize::Quad, scalar } if scalar.kind == K::Float && scalar.width == 4)
}
fn fields(
    m: &naga::Module,
    ty: naga::Handle<naga::Type>,
    bytes: u32,
    names: &[(&str, u32, u8)],
) -> bool {
    let TypeInner::Struct { members, span } = &m.types[ty].inner else {
        return false;
    };
    *span == bytes
        && members.len() == names.len()
        && members
            .iter()
            .zip(names)
            .all(|(member, (name, offset, shape))| {
                member.name.as_deref() == Some(*name)
                    && member.offset == *offset
                    && member.binding.is_none()
                    && match shape {
                        0 => matrix(m, member.ty),
                        1 => vector(m, member.ty, K::Float),
                        2 => vector(m, member.ty, K::Uint),
                        _ => false,
                    }
            })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Variant {
    Ordinary,
    Prepass,
    Occlusion,
    Fallback,
}
pub(super) fn validate(m: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    validate_variant(Variant::Ordinary, m, info)
}
pub(crate) fn validate_variant(
    variant: Variant,
    m: &naga::Module,
    info: &naga::valid::ModuleInfo,
) -> Result<()> {
    require(
        m.overrides.is_empty() && m.entry_points.len() == 2,
        "material stages/overrides",
    )?;
    let varying = vec![
        (None, K::Float, 4, false),
        (Some(0), K::Float, 3, false),
        (Some(1), K::Float, 3, false),
        (Some(2), K::Float, 4, false),
        (Some(3), K::Float, 2, false),
    ];
    let vertices: Vec<_> = (0..=10)
        .map(|n| {
            (
                Some(n),
                K::Float,
                if n < 2 {
                    3
                } else if n == 9 {
                    2
                } else {
                    4
                },
                false,
            )
        })
        .collect();
    let mut seen = [false; 2];
    for (index, ep) in m.entry_points.iter().enumerate() {
        let vertex = match (ep.stage, ep.name.as_str()) {
            (ShaderStage::Vertex, "vertex") => true,
            (ShaderStage::Fragment, "fragment") => false,
            _ => return Err(CookError::Contract("material stage/name")),
        };
        let slot = usize::from(!vertex);
        require(!seen[slot], "material repeated stage")?;
        seen[slot] = true;
        let mut input = Vec::new();
        let mut facing = 0;
        for arg in &ep.function.arguments {
            if arg.binding == Some(Binding::BuiltIn(BuiltIn::FrontFacing)) {
                require(
                    !vertex
                        && matches!(
                            m.types[arg.ty].inner,
                            TypeInner::Scalar(naga::Scalar {
                                kind: K::Bool,
                                width: 1
                            })
                        ),
                    "material front-facing input",
                )?;
                facing += 1;
            } else {
                io(m, arg.ty, arg.binding.as_ref(), &mut input)?;
            }
        }
        require(
            facing == usize::from(!vertex),
            "material front-facing count",
        )?;
        require(
            input == *if vertex { &vertices } else { &varying },
            "material input interface",
        )?;
        let mut output = Vec::new();
        if let Some(result) = &ep.function.result {
            io(m, result.ty, result.binding.as_ref(), &mut output)?;
        }
        require(
            output
                == if vertex {
                    varying.clone()
                } else {
                    vec![(Some(0), K::Float, 4, false)]
                },
            "material output interface",
        )?;
        for (handle, g) in m.global_variables.iter() {
            let fragment_used = !matches!(variant, Variant::Prepass | Variant::Fallback)
                || g.binding.is_some_and(|b| b.group == 1 && b.binding <= 2);
            let frame = g.binding
                == Some(naga::ResourceBinding {
                    group: 0,
                    binding: 0,
                });
            let used = if vertex { frame } else { fragment_used };
            require(
                info.get_entry_point(index)[handle].is_empty() != used,
                "material stage visibility",
            )?;
        }
    }
    require(
        m.global_variables.len()
            == match variant {
                Variant::Ordinary => 23,
                Variant::Prepass | Variant::Fallback => 16,
                Variant::Occlusion => 24,
            },
        "material resource count",
    )?;
    let mut storage = Vec::new();
    for (_, g) in m.global_variables.iter() {
        require(g.init.is_none(), "material resource initializer")?;
        let b = g
            .binding
            .ok_or(CookError::Contract("material unbound resource"))?;
        let uniform = g.space == AddressSpace::Uniform;
        let valid = match (b.group, b.binding) {
            (0, 0) => {
                uniform
                    && fields(
                        m,
                        g.ty,
                        96,
                        &[
                            ("view_projection", 0, 0),
                            ("eye", 64, 1),
                            ("environment", 80, 1),
                        ],
                    )
            }
            (0, 1) if variant == Variant::Occlusion => {
                g.space == AddressSpace::Handle
                    && m.types[g.ty].inner
                        == TypeInner::Image {
                            dim: ImageDimension::D2,
                            arrayed: false,
                            class: ImageClass::Sampled {
                                kind: K::Float,
                                multi: false,
                            },
                        }
            }
            (1, 0) => {
                uniform
                    && fields(
                        m,
                        g.ty,
                        64,
                        &[
                            ("base", 0, 1),
                            ("emissive_roughness", 16, 1),
                            ("factors", 32, 1),
                            ("flags", 48, 2),
                        ],
                    )
            }
            (3, 0) => {
                uniform
                    && fields(
                        m,
                        g.ty,
                        128,
                        &[
                            ("view", 0, 0),
                            ("viewport", 64, 1),
                            ("dimensions", 80, 2),
                            ("depth", 96, 1),
                            ("lights", 112, 2),
                        ],
                    )
            }
            (3, 6) => {
                if let TypeInner::Struct { members, span } = &m.types[g.ty].inner {
                    uniform
                        && *span == 1280
                        && members.len() == 1
                        && members[0].offset == 0
                        && members[0].name.as_deref() == Some("cascades")
                        && members[0].binding.is_none()
                        && matches!(m.types[members[0].ty].inner,TypeInner::Array {base,size:ArraySize::Constant(n),stride:80} if n.get()==16 && fields(m,base,80,&[("matrix",0,0),("parameters",64,1)]))
                } else {
                    false
                }
            }
            (3, 1..=3) => {
                storage.push((b.group, b.binding));
                g.space
                    == AddressSpace::Storage {
                        access: naga::StorageAccess::LOAD,
                    }
                    && matches!(m.types[g.ty].inner,TypeInner::Array {base,size:ArraySize::Dynamic,stride} if
                    if b.binding==1 {stride==64 && fields(m,base,64,&[("position_range",0,1),("direction_outer",16,1),("radiance_inner",32,1),("kind",48,2)])}
                    else {stride==4 && matches!(m.types[base].inner,TypeInner::Scalar(naga::Scalar {kind:K::Uint,width:4}))})
            }
            (1, 1 | 3 | 5 | 7 | 9) | (2, 0..=2) | (3, 4) => {
                let (dim, arrayed, class) = if b.group == 3 {
                    (ImageDimension::D2, true, ImageClass::Depth { multi: false })
                } else {
                    (
                        if b.group == 2 && b.binding < 2 {
                            ImageDimension::Cube
                        } else {
                            ImageDimension::D2
                        },
                        false,
                        ImageClass::Sampled {
                            kind: K::Float,
                            multi: false,
                        },
                    )
                };
                g.space == AddressSpace::Handle
                    && m.types[g.ty].inner
                        == TypeInner::Image {
                            dim,
                            arrayed,
                            class,
                        }
            }
            (1, 2 | 4 | 6 | 8 | 10) | (2, 3) | (3, 5) => {
                g.space == AddressSpace::Handle
                    && m.types[g.ty].inner
                        == TypeInner::Sampler {
                            comparison: b.group == 3,
                        }
            }
            _ => false,
        };
        require(valid, "material resource binding/type")?;
    }
    // This order is the compact fragment MSL lengths struct's ABI, supplied by
    // an explicit engine uniform because passthrough has no Naga reflection.
    require(
        storage
            == if matches!(variant, Variant::Prepass | Variant::Fallback) {
                vec![]
            } else {
                vec![(3, 1), (3, 2), (3, 3)]
            },
        "material storage length order",
    )
}
