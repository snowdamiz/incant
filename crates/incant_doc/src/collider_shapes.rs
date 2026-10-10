//! Bounded collision geometry. Compound parts are stable, local authoring data;
//! they never contain solver handles or recursively nested compounds.
use incant_types::{ColliderShape, PrimitiveColliderShape};
use std::collections::BTreeSet;

fn dimensions(values: impl IntoIterator<Item = f64>) -> Result<(), String> {
    if values.into_iter().all(|v| (0.001..=10000.).contains(&v)) {
        Ok(())
    } else {
        Err("collider dimensions must be 0.001..10000 meters".into())
    }
}
fn capsule(half_height: f64, radius: f64) -> Result<(), String> {
    dimensions([radius])?;
    if !(0. ..=10000.).contains(&half_height) {
        return Err("capsule half-height must be 0..10000 meters".into());
    }
    Ok(())
}
fn validate_primitive(shape: &PrimitiveColliderShape) -> Result<(), String> {
    match *shape {
        PrimitiveColliderShape::Box { half_extents } => dimensions(half_extents),
        PrimitiveColliderShape::Sphere { radius } => dimensions([radius]),
        PrimitiveColliderShape::Capsule {
            half_height,
            radius,
        } => capsule(half_height, radius),
    }
}
pub(crate) fn validate(shape: &ColliderShape) -> Result<(), String> {
    match shape {
        ColliderShape::Box { half_extents } => dimensions(*half_extents),
        ColliderShape::Sphere { radius } => dimensions([*radius]),
        ColliderShape::Capsule {
            half_height,
            radius,
        } => capsule(*half_height, *radius),
        ColliderShape::Compound { parts } => {
            if !(1..=64).contains(&parts.len()) {
                return Err("compound collider requires 1..64 primitive parts".into());
            }
            let mut ids = BTreeSet::new();
            for (index, part) in parts.iter().enumerate() {
                if ulid::Ulid::from_string(&part.id).is_err() || !ids.insert(&part.id) {
                    return Err(format!(
                        "compound part {index} requires a unique valid ULID"
                    ));
                }
                if !part
                    .translation
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 10000.)
                {
                    return Err(format!(
                        "compound part {index} offset must be finite and within ±10000 meters"
                    ));
                }
                let norm: f64 = part.rotation.iter().map(|v| v * v).sum();
                if !norm.is_finite() || (norm - 1.).abs() > 1.0e-4 {
                    return Err(format!(
                        "compound part {index} rotation must be a unit quaternion"
                    ));
                }
                validate_primitive(&part.shape)
                    .map_err(|e| format!("compound part {index}: {e}"))?;
            }
            Ok(())
        }
    }
}
