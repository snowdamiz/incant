//! Bounded collision geometry. Compound parts are stable, local authoring data;
//! they never contain solver handles or recursively nested compounds.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrimitiveColliderShape {
    Box {
        half_extents: [f64; 3],
    },
    Sphere {
        radius: f64,
    },
    /// Capsule along local Y; half_height excludes the hemispherical ends.
    Capsule {
        half_height: f64,
        radius: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColliderPart {
    /// Stable ULID, unique within this collider, retained when editing a part.
    pub id: String,
    /// Offset in the collider entity's local frame, in meters.
    pub translation: [f64; 3],
    /// Unit quaternion (x, y, z, w), relative to the collider entity.
    pub rotation: [f64; 4],
    pub shape: PrimitiveColliderShape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ColliderShape {
    Box {
        half_extents: [f64; 3],
    },
    Sphere {
        radius: f64,
    },
    /// Capsule along local Y; half_height excludes the hemispherical ends.
    Capsule {
        half_height: f64,
        radius: f64,
    },
    /// One body and material shared by a bounded union of local primitive parts.
    Compound {
        #[schemars(length(min = 1, max = 64))]
        parts: Vec<ColliderPart>,
    },
}

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
impl PrimitiveColliderShape {
    fn validate(&self) -> Result<(), String> {
        match *self {
            Self::Box { half_extents } => dimensions(half_extents),
            Self::Sphere { radius } => dimensions([radius]),
            Self::Capsule {
                half_height,
                radius,
            } => capsule(half_height, radius),
        }
    }
}
impl ColliderShape {
    pub(crate) fn validate(&self) -> Result<(), String> {
        match self {
            Self::Box { half_extents } => dimensions(*half_extents),
            Self::Sphere { radius } => dimensions([*radius]),
            Self::Capsule {
                half_height,
                radius,
            } => capsule(*half_height, *radius),
            Self::Compound { parts } => {
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
                    part.shape
                        .validate()
                        .map_err(|e| format!("compound part {index}: {e}"))?;
                }
                Ok(())
            }
        }
    }
}
