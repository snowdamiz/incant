use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ColliderShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BodyMotion {
    Fixed,
    Dynamic,
    Kinematic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RigidBody {
    pub motion: BodyMotion,
    #[schemars(range(min = -100, max = 100))]
    pub gravity_scale: f64,
    /// Linear velocity damping rate, in inverse seconds.
    #[schemars(range(min = 0, max = 100))]
    pub linear_damping: f64,
    /// Angular velocity damping rate, in inverse seconds.
    #[schemars(range(min = 0, max = 100))]
    pub angular_damping: f64,
    pub can_sleep: bool,
    pub ccd: bool,
}
impl Default for RigidBody {
    fn default() -> Self {
        Self {
            motion: BodyMotion::Dynamic,
            gravity_scale: 1.,
            linear_damping: 0.,
            angular_damping: 0.,
            can_sleep: true,
            ccd: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Collider {
    pub shape: ColliderShape,
    #[schemars(range(min = 0.001, max = 100000))]
    pub density: f64,
    #[schemars(range(min = 0, max = 10))]
    pub friction: f64,
    #[schemars(range(min = 0, max = 1))]
    pub restitution: f64,
    pub sensor: bool,
    /// Collision requires both membership/filter intersections to be nonzero.
    pub memberships: u32,
    pub filter: u32,
}
impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: [0.5; 3],
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
pub struct AngularVelocity {
    pub angular: [f64; 3],
}
