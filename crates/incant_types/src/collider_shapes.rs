use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
