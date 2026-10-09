//! Authored physics inputs. Distances are meters, mass density kg/m³, angular
//! velocity radians/second. Runtime solver handles never enter documents.
use crate::{Entity, Transform, Velocity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

pub(crate) fn validate(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    match kind {
        "RigidBody" => {
            let body: RigidBody =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !(-100. ..=100.).contains(&body.gravity_scale)
                || ![body.linear_damping, body.angular_damping]
                    .iter()
                    .all(|v| (0. ..=100.).contains(v))
            {
                return Err("gravity scale must be -100..100; damping must be 0..100".into());
            }
        }
        "Collider" => {
            let collider: Collider =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            let size_ok = |v: f64| (0.001..=10000.).contains(&v);
            let valid = match collider.shape {
                ColliderShape::Box { half_extents } => half_extents.into_iter().all(size_ok),
                ColliderShape::Sphere { radius } => size_ok(radius),
                ColliderShape::Capsule {
                    half_height,
                    radius,
                } => size_ok(radius) && (0. ..=10000.).contains(&half_height),
            };
            if !valid {
                return Err("collider dimensions must be 0.001..10000 meters (capsule half-height may be zero)".into());
            }
            if !(0.001..=100000.).contains(&collider.density)
                || !(0. ..=10.).contains(&collider.friction)
                || !(0. ..=1.).contains(&collider.restitution)
            {
                return Err(
                    "collider density must be 0.001..100000, friction 0..10, restitution 0..1"
                        .into(),
                );
            }
        }
        "AngularVelocity" => {
            let velocity: AngularVelocity =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !velocity
                .angular
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 10000.)
            {
                return Err("angular velocity must be finite and within ±10000 rad/s".into());
            }
        }
        _ => return Err("unregistered physics component".into()),
    }
    Ok(())
}

/// Cross-component constraints are checked on the complete transaction, so adding
/// a body, collider and transform together is atomic for every authoring client.
pub(crate) fn validate_entity(entity: &Entity) -> Result<(), String> {
    let c = &entity.components;
    if c.contains_key("AngularVelocity") && !c.contains_key("RigidBody") {
        return Err("AngularVelocity requires RigidBody".into());
    }
    if c.contains_key("RigidBody") && !c.contains_key("Collider") {
        return Err("RigidBody requires Collider".into());
    }
    if !c.contains_key("Collider") {
        return Ok(());
    }
    let transform: Transform = serde_json::from_value(
        c.get("Transform")
            .ok_or("Collider requires Transform")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    // Explicit initial runtime boundary; never silently flatten a rig or ignore scale.
    if entity.parent.is_some() || transform.scale != [1.; 3] {
        return Err("physics entities currently require a scene-root Transform with unit scale; size the Collider shape explicitly".into());
    }
    if !transform
        .translation
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    {
        return Err("physics position must be finite and within ±1000000 meters".into());
    }
    let body: Option<RigidBody> = c
        .get("RigidBody")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let fixed = body.as_ref().is_none_or(|b| b.motion == BodyMotion::Fixed);
    if fixed && let Some(value) = c.get("AngularVelocity") {
        let velocity: AngularVelocity =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if velocity.angular != [0.; 3] {
            return Err("fixed bodies cannot have angular velocity".into());
        }
    }
    if let Some(value) = c.get("Velocity") {
        let velocity: Velocity =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if !velocity
            .linear
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 10000.)
        {
            return Err("physics velocity must be finite and within ±10000 m/s".into());
        }
        if fixed && velocity.linear != [0.; 3] {
            return Err("moving colliders require a dynamic or kinematic RigidBody".into());
        }
    }
    Ok(())
}

pub(crate) fn annotate_schemas(
    registry: &mut std::collections::BTreeMap<String, serde_json::Value>,
) {
    use serde_json::json;
    let body = registry
        .get_mut("RigidBody")
        .expect("registered physics schema");
    body["order"] = json!([
        "motion",
        "gravity_scale",
        "linear_damping",
        "angular_damping",
        "can_sleep",
        "ccd"
    ]);
    for key in ["linear_damping", "angular_damping"] {
        body["properties"][key]["x-incant-unit"] = json!("1/s");
    }
    let collider = registry
        .get_mut("Collider")
        .expect("registered physics schema");
    collider["order"] = json!([
        "shape",
        "density",
        "friction",
        "restitution",
        "sensor",
        "memberships",
        "filter"
    ]);
    collider["properties"]["density"]["x-incant-unit"] = json!("kg/m³");
    for key in ["memberships", "filter"] {
        collider["properties"][key]["x-incant-widget"] = json!("collision-mask");
        collider["properties"][key]["maximum"] = json!(u32::MAX);
    }
    for variant in collider["$defs"]["ColliderShape"]["oneOf"]
        .as_array_mut()
        .expect("shape variants")
    {
        for (key, property) in variant["properties"]
            .as_object_mut()
            .expect("shape properties")
        {
            if key != "type" {
                property["x-incant-unit"] = json!("m");
            }
        }
    }
    registry
        .get_mut("AngularVelocity")
        .expect("registered physics schema")["properties"]["angular"]["x-incant-unit"] =
        json!("rad/s");
}
