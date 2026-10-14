//! Direct fixed-field capture. Serialization is only an independent test oracle.
use super::super::convert::{bits, mass_bits, pose_bits};
use crate::dynamics::{
    CoefficientCombineRule, RigidBody, RigidBodyAdditionalMassProps, RigidBodyType,
    RigidBodyVelocity,
};
use crate::geometry::{self as native, Collider};
use crate::math::AngVector;
use incant_physics_snapshot as s;

fn angular_bits(v: AngVector) -> [u32; 3] {
    #[cfg(feature = "dim2")]
    {
        [v.to_bits(), 0, 0]
    }
    #[cfg(feature = "dim3")]
    {
        bits(v)
    }
}
fn velocity(v: &RigidBodyVelocity<f32>) -> s::BodyVelocity {
    s::BodyVelocity {
        linear: bits(v.linvel),
        angular: angular_bits(v.angvel),
    }
}
fn user_words(v: u128) -> [u32; 4] {
    core::array::from_fn(|i| (v >> (32 * i)) as u32)
}
pub(super) fn body(b: &RigidBody) -> Result<s::RigidBodyState, s::Error> {
    let m = &b.mprops;
    #[cfg(feature = "dim2")]
    let inertia = [m.effective_world_inv_inertia.to_bits(), 0, 0, 0, 0, 0];
    #[cfg(feature = "dim3")]
    let inertia = {
        let i = m.effective_world_inv_inertia;
        [i.m11, i.m12, i.m13, i.m22, i.m23, i.m33].map(f32::to_bits)
    };
    let a = &b.activation;
    let f = &b.forces;
    let c = &b.ccd;
    let (soft_slot, soft_generation) = b.soft_body.into_raw_parts();
    Ok(s::RigidBodyState {
        position: pose_bits(b.pos.position),
        next_position: pose_bits(b.pos.next_position),
        velocity: velocity(&b.vels),
        ccd_velocity: velocity(&b.ccd_vels),
        forces: s::BodyForces {
            force: bits(f.force),
            torque: angular_bits(f.torque),
            gravity_scale: f.gravity_scale.to_bits(),
            user_force: bits(f.user_force),
            user_torque: angular_bits(f.user_torque),
            #[cfg(feature = "dim2")]
            gyroscopic: false,
            #[cfg(feature = "dim3")]
            gyroscopic: f.gyroscopic_forces_enabled,
        },
        damping: [
            b.damping.linear_damping.to_bits(),
            b.damping.angular_damping.to_bits(),
        ],
        mass: s::BodyMass {
            world_center: bits(m.world_com),
            effective_inverse_mass: bits(m.effective_inv_mass),
            effective_inverse_inertia: inertia,
            local: mass_bits(m.local_mprops),
            locked_axes: u32::from(m.flags.bits()),
            additional: m.additional_local_mprops.as_deref().map(|m| match m {
                RigidBodyAdditionalMassProps::Mass(v) => s::AdditionalMass::Mass(v.to_bits()),
                RigidBodyAdditionalMassProps::MassProps(v) => {
                    s::AdditionalMass::Properties(mass_bits(*v))
                }
            }),
            max_extent: m.max_extent.to_bits(),
        },
        activation: s::BodyActivation {
            linear_threshold: a.normalized_linear_threshold.to_bits(),
            angular_threshold: a.angular_threshold.to_bits(),
            time_until_sleep: a.time_until_sleep.to_bits(),
            time_since_can_sleep: a.time_since_can_sleep.to_bits(),
            sleeping: a.sleeping,
            previous_pose: pose_bits(a.sleep_prev_pose),
        },
        ccd: s::BodyCcd {
            thickness: c.ccd_thickness.to_bits(),
            active: c.ccd_active,
            enabled: c.ccd_enabled,
            soft_prediction: c.soft_ccd_prediction.to_bits(),
            allow_fast_rotation: c.allow_fast_rotation,
        },
        ids: [
            b.ids.active_island_id,
            b.ids.active_set_id,
            b.ids.island_id,
            b.ids.island_index,
        ],
        changes: b.changes.bits(),
        kind: match b.body_type {
            RigidBodyType::Dynamic => s::BodyKind::Dynamic,
            RigidBodyType::Fixed => s::BodyKind::Fixed,
            RigidBodyType::KinematicPositionBased => s::BodyKind::KinematicPosition,
            RigidBodyType::KinematicVelocityBased => s::BodyKind::KinematicVelocity,
            RigidBodyType::SoftFrame => return Err(s::Error::UnsupportedBody),
        },
        dominance: i32::from(b.dominance.0),
        enabled: b.enabled,
        additional_iterations: [
            b.additional_solver_iterations as u64,
            b.additional_pgs_iterations as u64,
        ],
        soft_body: s::Handle {
            slot: soft_slot,
            generation: soft_generation,
        },
        soft_cluster: b.soft_cluster,
        soft_motion_margin: b.soft_motion_margin.to_bits(),
        user_data: user_words(b.user_data),
    })
}
fn rule(v: CoefficientCombineRule) -> Result<s::CombineRule, s::Error> {
    Ok(match v {
        CoefficientCombineRule::Average => s::CombineRule::Average,
        CoefficientCombineRule::Min => s::CombineRule::Min,
        CoefficientCombineRule::Multiply => s::CombineRule::Multiply,
        CoefficientCombineRule::Max => s::CombineRule::Max,
        CoefficientCombineRule::ClampedSum | CoefficientCombineRule::GeometricMean => {
            return Err(s::Error::UnsupportedCollider);
        }
    })
}
fn groups(v: native::InteractionGroups) -> s::ColliderGroups {
    s::ColliderGroups {
        memberships: v.memberships.bits(),
        filter: v.filter.bits(),
        test: match v.test_mode {
            native::InteractionTestMode::And => s::GroupTest::And,
            native::InteractionTestMode::Or => s::GroupTest::Or,
        },
    }
}
pub(super) fn collider(c: &Collider) -> Result<s::ColliderState, s::Error> {
    let f = &c.flags;
    Ok(s::ColliderState {
        kind: match c.coll_type {
            native::ColliderType::Solid => s::ColliderKind::Solid,
            native::ColliderType::Sensor => s::ColliderKind::Sensor,
        },
        position: pose_bits(c.pos.0),
        mass: match &c.mprops {
            native::ColliderMassProps::Density(v) => s::ColliderMass::Density(v.to_bits()),
            native::ColliderMassProps::Mass(v) => s::ColliderMass::Mass(v.to_bits()),
            native::ColliderMassProps::MassProperties(v) => {
                s::ColliderMass::Properties(mass_bits(**v))
            }
        },
        changes: c.changes.bits(),
        material: s::ColliderMaterial {
            friction: c.material.friction.to_bits(),
            restitution: c.material.restitution.to_bits(),
            friction_rule: rule(c.material.friction_combine_rule)?,
            restitution_rule: rule(c.material.restitution_combine_rule)?,
        },
        active_collision_types: u32::from(f.active_collision_types.bits()),
        collision_groups: groups(f.collision_groups),
        solver_groups: groups(f.solver_groups),
        active_hooks: f.active_hooks.bits(),
        active_events: f.active_events.bits(),
        enabled: match f.enabled {
            native::ColliderEnabled::Enabled => s::ColliderEnabled::Enabled,
            native::ColliderEnabled::Disabled => s::ColliderEnabled::Disabled,
            native::ColliderEnabled::DisabledByParent => s::ColliderEnabled::DisabledByParent,
        },
        contact_skin: c.contact_skin().to_bits(),
        force_event_threshold: c.contact_force_event_threshold().to_bits(),
        deformable: c.deformable_mesh_ref.is_some(),
        user_data: user_words(c.user_data),
    })
}
pub(super) fn geometry(
    c: &Collider,
    id: s::Identity,
    budget: &mut s::Budget,
) -> Result<s::ValidatedGeometry, s::Error> {
    if id.dimension as usize != crate::math::DIM {
        return Err(s::Error::Geometry);
    }
    if let Some(compound) = c.shape().as_compound() {
        let (geometry, acceleration) = compound.checkpoint_capture(id, budget)?;
        return s::attach_compound(s::validate_geometry(geometry)?, acceleration, budget);
    }
    let shape = match c.shape().as_typed_shape() {
        parry::shape::TypedShape::Ball(b)
            if c.shape().as_shape::<parry::shape::Ball>().is_some() =>
        {
            s::Primitive::Ball {
                radius: b.radius.to_bits(),
            }
        }
        parry::shape::TypedShape::Cuboid(b)
            if c.shape().as_shape::<parry::shape::Cuboid>().is_some() =>
        {
            s::Primitive::Box {
                half_extents: bits(b.half_extents),
            }
        }
        parry::shape::TypedShape::Capsule(b)
            if c.shape().as_shape::<parry::shape::Capsule>().is_some() =>
        {
            let h = b.segment.b.y;
            let canonical = parry::shape::Capsule::new_y(h, b.radius);
            // Equality of floating values would silently fold signed zeros.
            // The compact shape must reconstruct the exact native endpoints.
            if bits(b.segment.a) != bits(canonical.segment.a)
                || bits(b.segment.b) != bits(canonical.segment.b)
            {
                return Err(s::Error::Geometry);
            }
            s::Primitive::CapsuleY {
                half_height: h.to_bits(),
                radius: b.radius.to_bits(),
            }
        }
        _ => return Err(s::Error::Geometry),
    };
    let mut parts = budget.vector::<s::Part>(1)?;
    parts.push(s::Part {
        translation: [0; 3],
        rotation: [0, 0, 0, 1f32.to_bits()],
        shape,
    });
    s::validate_geometry(s::GeometryRecord {
        identity: id,
        kind: s::GeometryKind::Primitive,
        parts,
    })
}
#[cfg(all(test, feature = "serde-serialize"))]
#[path = "fields_tests.rs"]
mod tests;
