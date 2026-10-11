//! Shared typed planar constraints. No backend, schema or JSON in native ticks.
use crate::{
    AngularVelocity2D, BodyMotion, Collider2D, ColliderShape2D, RigidBody, Transform, Velocity,
};

pub fn collider(collider: &Collider2D) -> Result<(), String> {
    let dimension = |v: f64| (0.001..=10000.).contains(&v);
    let valid = match collider.shape {
        ColliderShape2D::Box { half_extents } => half_extents.into_iter().all(dimension),
        ColliderShape2D::Circle { radius } => dimension(radius),
        ColliderShape2D::Capsule {
            half_height,
            radius,
        } => dimension(radius) && (0. ..=10000.).contains(&half_height),
    };
    if !valid
        || !(0.001..=100000.).contains(&collider.density)
        || !(0. ..=10.).contains(&collider.friction)
        || !(0. ..=1.).contains(&collider.restitution)
    {
        return Err("2D collider dimensions/material out of range".into());
    }
    Ok(())
}

pub fn angular(velocity: &AngularVelocity2D) -> Result<(), String> {
    if !velocity.angular.is_finite() || velocity.angular.abs() > 10000. {
        return Err("2D angular velocity must be finite and within ±10000 rad/s".into());
    }
    Ok(())
}

pub fn motion(
    parent: bool,
    transform: &Transform,
    body: Option<&RigidBody>,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity2D>,
) -> Result<(), String> {
    if parent
        || transform.scale != [1.; 3]
        || transform.rotation[0] != 0.
        || transform.rotation[1] != 0.
    {
        return Err("2D physics requires a root, unit-scale, planar Transform".into());
    }
    let norm: f64 = transform.rotation.iter().map(|v| v * v).sum();
    if !norm.is_finite()
        || (norm - 1.).abs() > 1e-4
        || !transform
            .translation
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    {
        return Err("2D position/quaternion out of range".into());
    }
    let fixed = body.is_none_or(|b| b.motion == BodyMotion::Fixed);
    if let Some(spin) = spin {
        angular(spin)?;
        if body.is_none() || (fixed && spin.angular != 0.) {
            return Err("2D angular motion requires a moving rigid body".into());
        }
    }
    if let Some(velocity) = velocity
        && (!velocity
            .linear
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 10000.)
            || velocity.linear[2] != 0.
            || (fixed && velocity.linear != [0.; 3]))
    {
        return Err("2D velocity requires finite planar motion and a moving rigid body".into());
    }
    Ok(())
}
