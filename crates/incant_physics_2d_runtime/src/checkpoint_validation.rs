//! Incant identity, declaration and observation joins around a native 2D frame.
use super::*;
use incant_types::ColliderShape2D;
#[cfg(test)]
mod tests;

pub(super) fn validate(runtime: &PhysicsWorld2D) -> Result<(), &'static str> {
    if runtime.failed {
        return Err("failed runtime");
    }
    if runtime.query_dirty {
        return Err("unsettled query frame");
    }
    if runtime.entries.len() > MAX_BODIES
        || runtime.overlaps.len() > MAX_TRIGGER_PAIRS
        || runtime.events.len() > MAX_TRIGGER_EVENTS
    {
        return Err("checkpoint collection limit");
    }
    if runtime.entries.len() != runtime.ids.len()
        || runtime.entries.len() != runtime.world.read().colliders.len()
        || runtime
            .entries
            .values()
            .filter(|entry| entry.body.is_some())
            .count()
            != runtime.world.read().bodies.len()
    {
        return Err("native ownership cardinality");
    }
    let mut shape_count = 0usize;
    for (id, entry) in &runtime.entries {
        shape_count = shape_count
            .checked_add(entry.config.shapes())
            .filter(|n| *n <= MAX_SHAPES)
            .ok_or("aggregate shape limit")?;
        if runtime.ids.get(&entry.collider) != Some(id) {
            return Err("collider reverse identity");
        }
        validate_collider(&entry.config.collider).map_err(|_| "invalid retained collider")?;
        validate_state(entry.config.body.as_ref(), &entry.config.state)
            .map_err(|_| "invalid retained state")?;
        if let Some(body) = &entry.config.body {
            incant_types::physics_validation::rigid_body(body)
                .map_err(|_| "invalid retained body")?;
        }
        match &entry.config.geometry {
            config::Geometry::Primitive(shape) if shape != &entry.config.collider.shape => {
                return Err("retained geometry declaration");
            }
            config::Geometry::Tiles(_)
                if entry.config.body.is_some()
                    || entry.config.collider != incant_types::Collider2D::default() =>
            {
                return Err("tile declaration");
            }
            _ => {}
        }
        let collider = runtime
            .world
            .read()
            .colliders
            .get(entry.collider)
            .ok_or("collider generation")?;
        if collider.parent() != entry.body {
            return Err("collider parent");
        }
        let config = &entry.config.collider;
        if !same(collider.density(), config.density as f32)
            || !same(collider.friction(), config.friction as f32)
            || !same(collider.restitution(), config.restitution as f32)
            || collider.is_sensor() != config.sensor
            || collider.collision_groups() != groups(config.memberships, config.filter)
            || collider.active_collision_types() != ActiveCollisionTypes::all()
        {
            return Err("collider declaration");
        }
        if !geometry_matches(collider.shape(), &entry.config.geometry)? {
            return Err("collider geometry");
        }
        match (entry.body, &entry.config.body) {
            (Some(handle), Some(config)) => {
                let body = runtime
                    .world
                    .read()
                    .bodies
                    .get(handle)
                    .ok_or("body generation")?;
                if body.colliders() != [entry.collider]
                    || collider.position_wrt_parent() != Some(&Pose::IDENTITY)
                {
                    return Err("body collider attachment");
                }
                let motion = match config.motion {
                    BodyMotion::Fixed => RigidBodyType::Fixed,
                    BodyMotion::Dynamic => RigidBodyType::Dynamic,
                    BodyMotion::Kinematic => RigidBodyType::KinematicVelocityBased,
                };
                let activation = if config.can_sleep {
                    RigidBodyActivation::active()
                } else {
                    RigidBodyActivation::cannot_sleep()
                };
                if body.body_type() != motion
                    || !same(body.gravity_scale(), config.gravity_scale as f32)
                    || !same(body.linear_damping(), config.linear_damping as f32)
                    || !same(body.angular_damping(), config.angular_damping as f32)
                    || body.is_ccd_enabled() != config.ccd
                    || !same(
                        body.activation().normalized_linear_threshold,
                        activation.normalized_linear_threshold,
                    )
                    || !same(
                        body.activation().angular_threshold,
                        activation.angular_threshold,
                    )
                    || !same(
                        body.activation().time_until_sleep,
                        activation.time_until_sleep,
                    )
                {
                    return Err("body declaration");
                }
                let state = &entry.config.state;
                if !array_same(
                    body.translation().to_array(),
                    state.translation.map(|n| n as f32),
                ) || !same(body.rotation().angle(), state.rotation as f32)
                    || !array_same(body.linvel().to_array(), state.velocity.map(|n| n as f32))
                    || !same(body.angvel(), state.angular_velocity as f32)
                    || body.is_sleeping() != state.sleeping
                {
                    return Err("committed body observation");
                }
            }
            (None, None) => {
                if collider.position() != &pose(&entry.config.state) || entry.config.state.sleeping
                {
                    return Err("committed fixed collider observation");
                }
            }
            _ => return Err("body declaration presence"),
        }
    }
    let mut native_pairs = 0usize;
    for (a, _, b, _, intersects) in runtime.world.read().intersection_pairs() {
        if !intersects {
            continue;
        }
        let a = runtime.ids.get(&a).ok_or("unowned trigger collider")?;
        let b = runtime.ids.get(&b).ok_or("unowned trigger collider")?;
        if a == b {
            return Err("self trigger pair");
        }
        let pair = if a < b { (*a, *b) } else { (*b, *a) };
        if !runtime.overlaps.contains(&pair) {
            return Err("missing committed overlap");
        }
        native_pairs += 1;
    }
    if native_pairs != runtime.overlaps.len() {
        return Err("extra committed overlap");
    }
    if !runtime.events.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err("event order or duplicate");
    }
    for event in &runtime.events {
        if event.first >= event.second {
            return Err("event entity order");
        }
        if runtime.overlaps.contains(&(event.first, event.second)) != event.entered {
            return Err("event transition disagrees with committed overlaps");
        }
        // A committed exit may name an entity removed before this completed step.
    }
    Ok(())
}
fn same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}
fn array_same<const N: usize>(a: [f32; N], b: [f32; N]) -> bool {
    a.map(f32::to_bits) == b.map(f32::to_bits)
}
fn shape_matches(shape: &dyn Shape, expected: &ColliderShape2D) -> Result<bool, &'static str> {
    Ok(match expected {
        ColliderShape2D::Box { half_extents } => shape
            .as_cuboid()
            .is_some_and(|s| array_same(s.half_extents.to_array(), half_extents.map(|n| n as f32))),
        ColliderShape2D::Circle { radius } => shape
            .as_ball()
            .is_some_and(|s| same(s.radius, *radius as f32)),
        ColliderShape2D::Capsule {
            half_height,
            radius,
        } => shape.as_capsule().is_some_and(|s| {
            same(s.radius, *radius as f32)
                && s.segment.a == Vector::new(0., -(*half_height as f32))
                && s.segment.b == Vector::new(0., *half_height as f32)
        }),
    })
}

fn geometry_matches(shape: &dyn Shape, expected: &config::Geometry) -> Result<bool, &'static str> {
    let config::Geometry::Tiles(parts) = expected else {
        let config::Geometry::Primitive(primitive) = expected else {
            unreachable!()
        };
        return shape_matches(shape, primitive);
    };
    if parts.is_empty() || parts.len() > MAX_SHAPES {
        return Err("tile part limit");
    }
    let Some(compound) = shape.as_compound() else {
        return Ok(false);
    };
    if !compound.flags().is_empty() || compound.shapes().len() != parts.len() {
        return Ok(false);
    }
    for ((pose, child), (center, half)) in compound.shapes().iter().zip(parts) {
        if !center
            .iter()
            .all(|n| n.is_finite() && (*n as f32).is_finite())
            || !half
                .iter()
                .all(|n| n.is_finite() && *n > 0. && (*n as f32).is_finite() && (*n as f32) > 0.)
            || !array_same(pose.translation.to_array(), center.map(|n| n as f32))
            || !same(pose.rotation.re, 1.)
            || !same(pose.rotation.im, 0.)
            || !child
                .as_cuboid()
                .is_some_and(|s| array_same(s.half_extents.to_array(), half.map(|n| n as f32)))
        {
            return Ok(false);
        }
    }
    Ok(true)
}
