//! Authored physics inputs. Distances are meters, mass density kg/m³, angular
//! velocity radians/second. Runtime solver handles never enter documents.
use crate::{Entity, Transform, Velocity};

use incant_types::{AngularVelocity, Collider, RigidBody};

pub(crate) fn validate(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    use incant_types::physics_validation as check;
    match kind {
        "RigidBody" => check::rigid_body(
            &serde_json::from_value::<RigidBody>(value.clone()).map_err(|e| e.to_string())?,
        ),
        "Collider" => check::collider(
            &serde_json::from_value::<Collider>(value.clone()).map_err(|e| e.to_string())?,
        ),
        "AngularVelocity" => check::angular(
            &serde_json::from_value::<AngularVelocity>(value.clone()).map_err(|e| e.to_string())?,
        ),
        _ => Err("unregistered physics component".into()),
    }
}

/// Cross-component checks still run on the complete authoring transaction.
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
    let body: Option<RigidBody> = c
        .get("RigidBody")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let velocity: Option<Velocity> = c
        .get("Velocity")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let angular: Option<AngularVelocity> = c
        .get("AngularVelocity")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    incant_types::physics_validation::motion(
        entity.parent.is_some(),
        &transform,
        body.as_ref(),
        velocity.as_ref(),
        angular.as_ref(),
    )
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
    collider["$defs"]["ColliderPart"]["properties"]["translation"]["x-incant-unit"] = json!("m");
    for shape in ["ColliderShape", "PrimitiveColliderShape"] {
        for variant in collider["$defs"][shape]["oneOf"]
            .as_array_mut()
            .expect("shape variants")
        {
            for (key, property) in variant["properties"]
                .as_object_mut()
                .expect("shape properties")
            {
                if ["half_extents", "radius", "half_height"].contains(&key.as_str()) {
                    property["x-incant-unit"] = json!("m");
                }
            }
        }
    }
    registry
        .get_mut("AngularVelocity")
        .expect("registered physics schema")["properties"]["angular"]["x-incant-unit"] =
        json!("rad/s");
}
