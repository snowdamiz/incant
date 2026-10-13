//! Conservative bounds for saved values consumed by first native arithmetic.
//! Bounds use f64 absolute sums so cancellation cannot hide an overflowing f32
//! intermediate. This does not promise finite results for arbitrary future game
//! edits or replace native failure isolation after an actual simulation step.
use super::{E, Result};
use crate::pipeline::checked_restore::NumericFrame;
use crate::pipeline::checked_restore::convert;
use incant_physics_snapshot::BodyKind;
use incant_physics_snapshot::RigidBodyState;
fn value(x: u32) -> f64 {
    f64::from(f32::from_bits(x)).abs()
}
fn sum<const N: usize>(v: [u32; N]) -> f64 {
    v.into_iter().map(value).sum()
}
fn bounded(v: f64) -> Result<()> {
    // Retain 64x headroom for the fixed-width (<= 4) vector/quaternion sums and
    // roundoff. This is a documented stricter scalar domain, never clamping.
    if !v.is_finite() || v > f64::from(f32::MAX) / 64. {
        Err(E::Arithmetic)
    } else {
        Ok(())
    }
}
fn squared(v: f64) -> Result<()> {
    bounded(v * v)
}
fn velocity(b: &RigidBodyState) -> f64 {
    sum(b.velocity.linear) + sum(b.velocity.angular)
}
// Same first force refresh/integration as derived admission, retained here as
// a velocity bound as well as a finite-result check. No native owner advances.
fn force_velocity(b: &RigidBodyState, gravity: [u32; 3], dt: f32) -> f64 {
    if b.kind != BodyKind::Dynamic || !b.enabled {
        return velocity(b);
    }
    use crate::dynamics::{LockedAxes, RigidBodyForces, RigidBodyMassProps, RigidBodyVelocity};
    let mut mass = RigidBodyMassProps::from(convert::mass(b.mass.local));
    mass.flags = LockedAxes::from_bits_retain(b.mass.locked_axes as u8);
    mass.update_world_mass_properties(convert::kind(b.kind), &convert::pose(b.position));
    let mut forces = RigidBodyForces {
        force: convert::v(b.forces.force),
        torque: convert::angular(b.forces.torque),
        gravity_scale: f32::from_bits(b.forces.gravity_scale),
        user_force: convert::v(b.forces.user_force),
        user_torque: convert::angular(b.forces.user_torque),
        #[cfg(feature = "dim3")]
        gyroscopic_forces_enabled: b.forces.gyroscopic,
    };
    forces.compute_effective_force_and_torque(convert::v(gravity), mass.effective_mass());
    let next = forces.integrate(
        dt,
        &RigidBodyVelocity {
            linvel: convert::v(b.velocity.linear),
            angvel: convert::angular(b.velocity.angular),
        },
        &mass,
    );
    #[cfg(feature = "dim2")]
    let angular = f64::from(next.angvel).abs();
    #[cfg(feature = "dim3")]
    let angular = sum(convert::bits(next.angvel));
    sum(convert::bits(next.linvel)) + angular
}
#[cfg(feature = "dim3")]
fn gyro_bound(b: &RigidBodyState, velocity: f64, dt: f32) -> Result<()> {
    if !b.forces.gyroscopic || b.kind != BodyKind::Dynamic || !b.enabled {
        return Ok(());
    }
    let mass = convert::mass(b.mass.local);
    let inertia = sum(convert::bits(mass.principal_inertia()));
    let inverse = sum(b.mass.local.inverse_principal_inertia);
    let w = 4. * velocity;
    let momentum = inertia * w;
    // Includes force/warm increments and any unit-axis orientation, not only
    // the saved angular velocity checked by the exact scalar formula below.
    squared(momentum + 2. * w * momentum * f64::from(dt))?;
    squared(4. * inverse * momentum)?;
    Ok(())
}
#[cfg(feature = "dim3")]
fn gyro(b: &RigidBodyState, dt: f32) -> Result<()> {
    if !b.forces.gyroscopic || b.kind != BodyKind::Dynamic || !b.enabled {
        return Ok(());
    }
    let m = convert::mass(b.mass.local);
    let axes = convert::pose(b.position).rotation * m.principal_inertia_local_frame;
    let w = axes.inverse() * convert::v(b.velocity.angular);
    let momentum = m.principal_inertia() * w;
    let explicit = -w.cross(momentum) * dt;
    let total = momentum + explicit;
    let a = momentum.length_squared();
    let d = total.length_squared();
    if !w.is_finite()
        || !momentum.is_finite()
        || !explicit.is_finite()
        || !total.is_finite()
        || !a.is_finite()
        || !d.is_finite()
    {
        return Err(E::Gyroscopic);
    }
    if d != 0. {
        let ratio = a / d;
        let out = axes * (m.inv_principal_inertia * (total * ratio.sqrt()));
        if !ratio.is_finite() || !out.is_finite() {
            return Err(E::Gyroscopic);
        }
    }
    Ok(())
}
pub(super) fn admit(n: &NumericFrame<'_>) -> Result<()> {
    let f = n.boundary();
    let cs = f.pipeline().context().ccd.context().colliders;
    let bs = &cs.context().bodies.record().bodies;
    let config = f.pipeline().context().integration.record();
    let dt = f32::from_bits(config.dt);
    let inv_dt = 1. / dt;
    let sub_dt = dt / n.report().max_substeps as f32;
    let inv_sub_dt = 1. / sub_dt;
    let mut max_mass = 0.0f64;
    let mut max_inertia = 0.0f64;
    let mut max_velocity = 0.0f64;
    let mut max_position = 0.0f64;
    for b in bs {
        let position = sum(b.position.translation)
            .max(sum(b.next_position.translation))
            .max(sum(b.activation.previous_pose.translation))
            .max(sum(b.mass.world_center));
        squared(4. * position)?;
        max_position = max_position.max(position);
        let mut velocity = velocity(b)
            .max(sum(b.ccd_velocity.linear) + sum(b.ccd_velocity.angular))
            .max(force_velocity(b, f.pipeline().record().gravity, dt));
        if b.kind == BodyKind::KinematicPosition && b.enabled {
            velocity = velocity.max((2. * position + 8.) * f64::from(inv_sub_dt));
        }
        squared(velocity * (1. + value(b.mass.max_extent)))?;
        max_velocity = max_velocity.max(velocity);
        max_mass = max_mass.max(sum(b.mass.effective_inverse_mass));
        max_inertia = max_inertia.max(sum(b.mass.effective_inverse_inertia));
        for threshold in [
            b.activation.linear_threshold,
            b.activation.angular_threshold,
        ] {
            // Negative native thresholds mean never sleep; no squaring occurs.
            if f32::from_bits(threshold) >= 0. {
                squared(value(threshold) * value(config.length_unit))?;
            }
        }
        bounded(value(b.activation.time_since_can_sleep) + f64::from(dt))?;
        #[cfg(feature = "dim3")]
        {
            gyro(b, dt)?;
            gyro(b, sub_dt)?;
        }
    }
    bounded(max_mass)?;
    bounded(max_inertia)?;
    let table = f.manifolds().record();
    let mut warm_delta = 0.0f64;
    let mut max_arm = 0.0f64;
    let mut max_impulse = 0.0f64;
    for p in &table.points {
        let arm = sum(p.data.solver_dp1).max(sum(p.data.solver_dp2));
        let local = sum(p.local_p1).max(sum(p.local_p2));
        squared(4. * (local + arm + max_position))?;
        let impulse = value(p.data.warmstart_impulse)
            + sum(p.data.warmstart_tangent_impulse)
            + sum(p.data.warmstart_tangent_world)
            + value(p.data.warmstart_twist_impulse);
        // The saved normal impulse is unilateral in the pinned rigid solver.
        if f32::from_bits(p.data.impulse) < 0. || f32::from_bits(p.data.warmstart_impulse) < 0. {
            return Err(E::Arithmetic);
        }
        bounded(value(p.data.impulse))?;
        bounded(sum(p.data.tangent_impulse))?;
        bounded(impulse)?;
        max_impulse = max_impulse.max(impulse);
        max_arm = max_arm.max(arm);
        bounded(16. * arm * arm * max_inertia + max_mass)?;
        // Sum all saved kicks as if applied to one body. This over-approximates
        // every world component without allocating per-body scratch.
        warm_delta += 8. * impulse * (max_mass + (arm + 1.) * max_inertia);
        bounded(warm_delta)?;
    }
    let velocity = max_velocity + warm_delta;
    #[cfg(feature = "dim3")]
    for b in bs {
        gyro_bound(b, velocity, dt)?;
    }
    squared(4. * velocity * (max_arm + 1.))?;
    squared(4. * (max_position + f64::from(dt) * velocity))?;
    for s in &table.solver {
        let anchor = sum(s.anchor1) + sum(s.anchor2);
        let relative = 4. * (anchor + max_position);
        bounded(relative * f64::from(inv_sub_dt))?;
        bounded(value(s.distance) * f64::from(inv_dt))?;
        squared(sum(s.tangent_velocity))?;
    }
    for m in &table.manifolds {
        let friction = value(m.friction);
        let restitution = value(m.restitution);
        bounded(friction * max_impulse)?;
        bounded(restitution * velocity * (max_arm + 1.))?;
        // The coupled spatial tangent form includes r * rhs² before division.
        // Every r term is an inverse-mass/inertia quadratic in the frozen arm.
        bounded(
            64. * (max_mass + max_inertia * max_arm * max_arm)
                * velocity
                * velocity
                * (max_arm + 1.)
                * (max_arm + 1.),
        )?;
    }
    // Retained recycle poses enter a translation-length and quaternion-chord
    // comparison before regeneration; huge finite cache poses must not overflow.
    for pair in &f
        .broad()
        .context()
        .narrow
        .context()
        .rigid
        .pairs()
        .record()
        .pairs
    {
        if let Some(c) = pair.recycle {
            squared(4. * (sum(c.relative_pose.translation) + max_position))?;
            bounded(8. * value(c.max_extent) + value(c.max_drift))?;
        }
    }
    for wide in &f.broad().context().tree.record().nodes {
        for node in [wide.left, wide.right] {
            if node.leaf_count() != 0 {
                squared(4. * (sum(node.mins) + sum(node.maxs)))?;
            }
        }
    }
    Ok(())
}
