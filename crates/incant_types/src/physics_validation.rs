//! Bounded collision geometry. Compound parts are stable, local authoring data;
//! they never contain solver handles or recursively nested compounds.
use crate::{
    AngularVelocity, BodyMotion, Collider, ColliderShape, PrimitiveColliderShape, RigidBody,
    Transform, Velocity,
};
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
pub fn collider_shape(shape: &ColliderShape) -> Result<(), String> {
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
                if !valid_id(&part.id) || !ids.insert(&part.id) {
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

fn valid_id(id: &str) -> bool {
    id.len() == 26
        && id
            .bytes()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c.to_ascii_uppercase()))
}

pub fn rigid_body(body: &RigidBody) -> Result<(), String> {
    if !(-100. ..=100.).contains(&body.gravity_scale)
        || ![body.linear_damping, body.angular_damping]
            .iter()
            .all(|v| (0. ..=100.).contains(v))
    {
        return Err("gravity scale must be -100..100; damping must be 0..100".into());
    }
    Ok(())
}

pub fn collider(collider: &Collider) -> Result<(), String> {
    collider_shape(&collider.shape)?;
    if !(0.001..=100000.).contains(&collider.density)
        || !(0. ..=10.).contains(&collider.friction)
        || !(0. ..=1.).contains(&collider.restitution)
    {
        return Err(
            "collider density must be 0.001..100000, friction 0..10, restitution 0..1".into(),
        );
    }
    Ok(())
}

pub fn angular(velocity: &AngularVelocity) -> Result<(), String> {
    if !velocity
        .angular
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 10000.)
    {
        return Err("angular velocity must be finite and within ±10000 rad/s".into());
    }
    Ok(())
}

/// Typed runtime constraints for an entity with a collider. General transform
/// quaternion/finite checks remain required at scene/component boundaries.
pub fn motion(
    parent: bool,
    transform: &Transform,
    body: Option<&RigidBody>,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity>,
) -> Result<(), String> {
    if spin.is_some() && body.is_none() {
        return Err("AngularVelocity requires RigidBody".into());
    }
    if parent || transform.scale != [1.; 3] {
        return Err("physics entities currently require a scene-root Transform with unit scale; size the Collider shape explicitly".into());
    }
    if !transform
        .translation
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    {
        return Err("physics position must be finite and within ±1000000 meters".into());
    }
    let fixed = body.is_none_or(|b| b.motion == BodyMotion::Fixed);
    if let Some(spin) = spin {
        angular(spin)?;
        if fixed && spin.angular != [0.; 3] {
            return Err("fixed bodies cannot have angular velocity".into());
        }
    }
    if let Some(velocity) = velocity {
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
