//! Outer owner joins used before capturing or publishing a physics checkpoint.
//! Native frame admission supplies solver coherence; this checks the identities,
//! typed declarations and committed observations owned by Incant around it.
use super::*;
use incant_types::{ColliderShape, PrimitiveColliderShape, StableId};

#[cfg(test)]
mod tests;

pub(super) fn validate(runtime: &PhysicsRuntime<StableId>) -> Result<(), &'static str> {
    if runtime.failed {
        return Err("failed runtime");
    }
    let mut entries = 0usize;
    for scene in runtime.scenes.values() {
        entries = entries
            .checked_add(scene.entries.len())
            .filter(|n| *n <= 1_000_000)
            .ok_or("entry limit")?;
        validate_scene(scene)?;
    }
    if runtime.overlaps.len() > MAX_TRIGGER_PAIRS || runtime.events.len() > MAX_TRIGGER_EVENTS {
        return Err("trigger limit");
    }
    let mut native_pairs = 0usize;
    for (scene_id, scene) in &runtime.scenes {
        for (a, _, b, _, intersects) in scene.world.read().intersection_pairs() {
            if !intersects {
                continue;
            }
            let a = scene.ids.get(&a).ok_or("unowned trigger collider")?;
            let b = scene.ids.get(&b).ok_or("unowned trigger collider")?;
            if a == b {
                return Err("self trigger pair");
            }
            let (first, second) = if a < b { (*a, *b) } else { (*b, *a) };
            if !runtime.overlaps.contains(&(*scene_id, first, second)) {
                return Err("missing committed overlap");
            }
            native_pairs += 1;
        }
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
        let present = runtime
            .overlaps
            .contains(&(event.scene_id, event.first, event.second));
        if present != event.entered {
            return Err("event transition disagrees with committed overlaps");
        }
        // An exit from the last completed tick may name a removed entity or a
        // removed scene. Requiring live handles here would discard valid exits.
    }
    Ok(())
}

fn validate_scene(scene: &SceneWorld<StableId>) -> Result<(), &'static str> {
    if scene.query_dirty || scene.character.is_some() {
        return Err("unsettled query or character frame");
    }
    if scene.entries.len() != scene.ids.len()
        || scene.entries.len() != scene.world.read().colliders.len()
        || scene
            .entries
            .values()
            .filter(|entry| entry.body.is_some())
            .count()
            != scene.world.read().bodies.len()
    {
        return Err("native ownership cardinality");
    }
    for (id, entry) in &scene.entries {
        if scene.ids.get(&entry.collider) != Some(id) {
            return Err("collider reverse identity");
        }
        entry
            .config
            .validate()
            .map_err(|_| "invalid retained declaration")?;
        let collider = scene
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
        if !shape_matches(collider.shape(), &config.shape)? {
            return Err("collider geometry");
        }
        match (entry.body, &entry.config.body) {
            (Some(handle), Some(config)) => {
                let body = scene
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
                ) || !array_same(body.rotation().to_array(), state.rotation.map(|n| n as f32))
                    || !array_same(body.linvel().to_array(), state.velocity.map(|n| n as f32))
                    || !array_same(
                        body.angvel().to_array(),
                        state.angular_velocity.map(|n| n as f32),
                    )
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
    Ok(())
}

fn same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}
fn array_same<const N: usize>(a: [f32; N], b: [f32; N]) -> bool {
    a.map(f32::to_bits) == b.map(f32::to_bits)
}
fn shape_matches(shape: &dyn Shape, expected: &ColliderShape) -> Result<bool, &'static str> {
    Ok(match expected {
        ColliderShape::Box { half_extents } => shape
            .as_cuboid()
            .is_some_and(|s| array_same(s.half_extents.to_array(), half_extents.map(|n| n as f32))),
        ColliderShape::Sphere { radius } => shape
            .as_ball()
            .is_some_and(|s| same(s.radius, *radius as f32)),
        ColliderShape::Capsule {
            half_height,
            radius,
        } => shape.as_capsule().is_some_and(|s| {
            same(s.radius, *radius as f32)
                && s.segment.a == Vector::new(0., -(*half_height as f32), 0.)
                && s.segment.b == Vector::new(0., *half_height as f32, 0.)
        }),
        ColliderShape::Compound { parts } => {
            if !(1..=64).contains(&parts.len()) || !parts.windows(2).all(|w| w[0].id < w[1].id) {
                return Err("compound part order or limit");
            }
            let Some(compound) = shape.as_compound() else {
                return Ok(false);
            };
            if !compound.flags().is_empty() || compound.shapes().len() != parts.len() {
                return Ok(false);
            }
            for ((pose, child), part) in compound.shapes().iter().zip(parts) {
                // Match the original constructor's f64 -> f32 normalized rotation,
                // not a reconstructed Compound or an approximate pose comparison.
                let rotation = Rotation::from_array(part.rotation.map(|v| v as f32)).normalize();
                if !array_same(
                    pose.translation.to_array(),
                    part.translation.map(|v| v as f32),
                ) || !array_same(pose.rotation.to_array(), rotation.to_array())
                    || !primitive_matches(child.as_ref(), &part.shape)?
                {
                    return Ok(false);
                }
            }
            true
        }
    })
}

fn primitive_matches(
    shape: &dyn Shape,
    expected: &PrimitiveColliderShape,
) -> Result<bool, &'static str> {
    let expected = match *expected {
        PrimitiveColliderShape::Box { half_extents } => ColliderShape::Box { half_extents },
        PrimitiveColliderShape::Sphere { radius } => ColliderShape::Sphere { radius },
        PrimitiveColliderShape::Capsule {
            half_height,
            radius,
        } => ColliderShape::Capsule {
            half_height,
            radius,
        },
    };
    shape_matches(shape, &expected)
}
