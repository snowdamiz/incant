//! Exact per-family interstage sampling; a centroid bundle cannot accept a
//! center-sampled source (and shadow/UI stay strictly center/flat at 1x).
use super::Kind;
use crate::{CookError, Result};
fn check(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&naga::Binding>,
    expected: naga::Sampling,
) -> Result<()> {
    match binding {
        Some(naga::Binding::Location {
            interpolation,
            sampling,
            ..
        }) => {
            let wanted = if *interpolation == Some(naga::Interpolation::Flat) {
                naga::Sampling::First
            } else {
                expected
            };
            if *sampling != Some(wanted) {
                return Err(CookError::Contract("geometry interstage sampling"));
            }
        }
        Some(naga::Binding::BuiltIn(_)) => {}
        None => {
            if let naga::TypeInner::Struct { members, .. } = &module.types[ty].inner {
                for member in members {
                    check(module, member.ty, member.binding.as_ref(), expected)?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn validate(kind: Kind, module: &naga::Module) -> Result<()> {
    let expected = match kind {
        Kind::Diagnostic | Kind::Material | Kind::Sprite | Kind::Text => naga::Sampling::Centroid,
        Kind::Shadow | Kind::Ui => naga::Sampling::Center,
    };
    for entry in &module.entry_points {
        match entry.stage {
            naga::ShaderStage::Vertex => {
                if let Some(result) = &entry.function.result {
                    check(module, result.ty, result.binding.as_ref(), expected)?;
                }
            }
            naga::ShaderStage::Fragment => {
                for argument in &entry.function.arguments {
                    check(module, argument.ty, argument.binding.as_ref(), expected)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
