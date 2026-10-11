//! Planar values shared by authoring/cooking and the document-free solver.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ColliderShape2D {
    Box {
        half_extents: [f64; 2],
    },
    Circle {
        radius: f64,
    },
    /// Capsule aligned with local Y; zero half-height is a circle.
    Capsule {
        half_height: f64,
        radius: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Collider2D {
    pub shape: ColliderShape2D,
    #[schemars(range(min = 0.001, max = 100000))]
    pub density: f64,
    #[schemars(range(min = 0, max = 10))]
    pub friction: f64,
    #[schemars(range(min = 0, max = 1))]
    pub restitution: f64,
    pub sensor: bool,
    pub memberships: u32,
    pub filter: u32,
}
impl Default for Collider2D {
    fn default() -> Self {
        Self {
            shape: ColliderShape2D::Box {
                half_extents: [0.5; 2],
            },
            density: 1000.,
            friction: 0.5,
            restitution: 0.,
            sensor: false,
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AngularVelocity2D {
    /// Counter-clockwise rotation about +Z, in radians per second.
    #[schemars(range(min = -10000, max = 10000))]
    pub angular: f64,
}
