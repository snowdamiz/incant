//! Only the fixed clustered-light assignment memory and invocation contract.
use crate::{CookError, Result};
use naga::{
    AddressSpace as A, ArraySize, Binding, BuiltIn, Module, ScalarKind as K, TypeInner as T,
};

fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(CookError::Contract(reason))
    }
}
fn scalar(m: &Module, t: naga::Handle<naga::Type>, atomic: bool) -> bool {
    let s = naga::Scalar {
        kind: K::Uint,
        width: 4,
    };
    m.types[t].inner == if atomic { T::Atomic(s) } else { T::Scalar(s) }
}
fn vector(m: &Module, t: naga::Handle<naga::Type>, kind: K, size: naga::VectorSize) -> bool {
    matches!(m.types[t].inner, T::Vector {size: n, scalar} if n == size && scalar.kind == kind && scalar.width == 4)
}
fn fields(m: &Module, t: naga::Handle<naga::Type>, bytes: u32, fields: &[(&str, u32, u8)]) -> bool {
    let T::Struct { members, span } = &m.types[t].inner else {
        return false;
    };
    *span == bytes && members.len() == fields.len() && members.iter().zip(fields).all(|(x,(name,offset,shape))| {
        x.name.as_deref() == Some(*name) && x.offset == *offset && x.binding.is_none() && match shape {
            0 => matches!(m.types[x.ty].inner, T::Matrix {columns:naga::VectorSize::Quad,rows:naga::VectorSize::Quad,scalar} if scalar.kind==K::Float && scalar.width==4),
            1 => vector(m,x.ty,K::Float,naga::VectorSize::Quad),
            2 => vector(m,x.ty,K::Uint,naga::VectorSize::Quad),
            _ => false,
        }
    })
}
pub(super) fn validate(m: &Module, info: &naga::valid::ModuleInfo) -> Result<()> {
    require(
        m.overrides.is_empty() && m.entry_points.len() == 1,
        "cluster entries/overrides",
    )?;
    let ep = &m.entry_points[0];
    require(
        ep.stage == naga::ShaderStage::Compute
            && ep.name == "build"
            && ep.workgroup_size == [64, 1, 1]
            && ep.workgroup_size_overrides.is_none()
            && ep.function.result.is_none(),
        "cluster invocation",
    )?;
    require(ep.function.arguments.len() == 2, "cluster input count")?;
    let args = &ep.function.arguments;
    require(
        args[0].binding == Some(Binding::BuiltIn(BuiltIn::WorkGroupId))
            && vector(m, args[0].ty, K::Uint, naga::VectorSize::Tri)
            && args[1].binding == Some(Binding::BuiltIn(BuiltIn::LocalInvocationIndex))
            && scalar(m, args[1].ty, false),
        "cluster builtin inputs",
    )?;
    require(m.global_variables.len() == 6, "cluster resource count")?;
    let expected = ["grid", "lights", "counts", "masks", "hits", "membership"];
    for ((h, g), name) in m.global_variables.iter().zip(expected) {
        require(
            g.name.as_deref() == Some(name)
                && g.init.is_none()
                && !info.get_entry_point(0)[h].is_empty(),
            "cluster resource identity/use",
        )?;
        let valid = match name {
            "grid" => {
                g.binding
                    == Some(naga::ResourceBinding {
                        group: 0,
                        binding: 0,
                    })
                    && g.space == A::Uniform
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
            "lights" | "counts" | "masks" => {
                let binding = if name == "lights" {
                    1
                } else if name == "counts" {
                    2
                } else {
                    3
                };
                let access = if binding == 1 {
                    naga::StorageAccess::LOAD
                } else {
                    naga::StorageAccess::LOAD | naga::StorageAccess::STORE
                };
                g.binding == Some(naga::ResourceBinding { group: 0, binding })
                    && g.space == A::Storage { access }
                    && matches!(m.types[g.ty].inner,T::Array{base,size:ArraySize::Dynamic,stride} if
                        if binding==1 {stride==64 && fields(m,base,64,&[("position_range",0,1),("direction_outer",16,1),("radiance_inner",32,1),("kind",48,2)])}
                        else {stride==4 && scalar(m,base,false)})
            }
            "hits" => g.binding.is_none() && g.space == A::WorkGroup && scalar(m, g.ty, true),
            "membership" => {
                g.binding.is_none()
                    && g.space == A::WorkGroup
                    && matches!(m.types[g.ty].inner,T::Array{base,size:ArraySize::Constant(n),stride:4} if n.get()==128 && scalar(m,base,true))
            }
            _ => false,
        };
        require(valid, "cluster resource memory/binding")?;
    }
    Ok(())
}
