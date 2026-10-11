//! Authoring adapter for the shared planar values/constraints.
use crate::{Entity, Transform, Velocity};
use incant_types::{AngularVelocity2D, Collider2D, RigidBody, physics_2d_validation};

pub(crate) fn validate(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    match kind {
        "RigidBody2D" => incant_types::physics_validation::rigid_body(
            &serde_json::from_value::<RigidBody>(value.clone()).map_err(|e| e.to_string())?,
        ),
        "Collider2D" => physics_2d_validation::collider(
            &serde_json::from_value::<Collider2D>(value.clone()).map_err(|e| e.to_string())?,
        ),
        "AngularVelocity2D" => physics_2d_validation::angular(
            &serde_json::from_value::<AngularVelocity2D>(value.clone())
                .map_err(|e| e.to_string())?,
        ),
        _ => Err("unregistered 2D component".into()),
    }
}
pub(crate) fn validate_entity(entity: &Entity) -> Result<(), String> {
    let c = &entity.components;
    if c.contains_key("AngularVelocity2D") && !c.contains_key("RigidBody2D") {
        return Err("AngularVelocity2D requires RigidBody2D".into());
    }
    if c.contains_key("RigidBody2D") && !c.contains_key("Collider2D") {
        return Err("RigidBody2D requires Collider2D".into());
    }
    let solid = if let Some(map) = c.get("Tilemap") {
        let map: crate::Tilemap = serde_json::from_value(map.clone()).map_err(|e| e.to_string())?;
        map.layers
            .iter()
            .any(|layer| layer.solid && layer.cells.iter().any(Option::is_some))
    } else {
        false
    };
    let explicit = c.contains_key("Collider2D");
    if !explicit && !solid {
        return Ok(());
    }
    if (explicit && solid) || c.contains_key("Collider") {
        return Err("2D, 3D and solid-tilemap collision require separate entities".into());
    }
    let decode = |key: &str| c.get(key).cloned();
    let transform: Transform =
        serde_json::from_value(decode("Transform").ok_or("2D collision requires Transform")?)
            .map_err(|e| e.to_string())?;
    let body: Option<RigidBody> = decode("RigidBody2D")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let velocity: Option<Velocity> = decode("Velocity")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let spin: Option<AngularVelocity2D> = decode("AngularVelocity2D")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    physics_2d_validation::motion(
        entity.parent.is_some(),
        &transform,
        body.as_ref(),
        velocity.as_ref(),
        spin.as_ref(),
    )
}
