//! Private exact fields. Only the enclosing numeric frame selects payload rows.
//! This does not publish a body set into any PhysicsWorld or restore save support.
use super::{NumericFrame, allocation::*, convert::*};
use crate::data::Arena;
use crate::dynamics::*;
use crate::geometry::ColliderHandle;
use incant_physics_snapshot::{AdditionalMass, BodyVelocity, Handle, RigidBodyState};

pub(super) fn plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
    let state = frame
        .frame
        .pipeline()
        .context()
        .ccd
        .context()
        .colliders
        .context()
        .bodies;
    let a = frame.frame.attachments().record();
    let mut p = Arena::<RigidBody>::checkpoint_plan(frame.frame.body_arena())?;
    for (row, body) in state.record().bodies.iter().enumerate() {
        p.append(row_plan(body, a.bodies[row].count as usize)?)?;
    }
    Ok(p)
}
pub(super) fn construct(
    frame: &NumericFrame<'_>,
    permit: &mut AllocationPermit,
) -> ConstructionResult<RigidBodySet> {
    let state = frame
        .frame
        .pipeline()
        .context()
        .ccd
        .context()
        .colliders
        .context()
        .bodies;
    let a = frame.frame.attachments().record();
    let bodies = Arena::from_checkpoint(frame.frame.body_arena(), permit, |row, p| {
        let body = state
            .record()
            .bodies
            .get(row as usize)
            .ok_or(ConstructionError::Payload)?;
        let span = a
            .bodies
            .get(row as usize)
            .ok_or(ConstructionError::Payload)?;
        let list = a
            .attached
            .get(span.start as usize..(span.start + span.count) as usize)
            .ok_or(ConstructionError::Payload)?;
        from_row(body, list, p)
    })?;
    Ok(RigidBodySet {
        bodies,
        // Completed owner journals are explicitly empty in the joined frame.
        modified_bodies: Default::default(),
        // Immutable sentinel is not serialized by native Rapier and its sole
        // constructor is exactly this fixed builder (no arena/body references).
        default_fixed: RigidBodyBuilder::fixed().build(),
    })
}
fn row_plan(b: &RigidBodyState, count: usize) -> ConstructionResult<AllocationPlan> {
    let mut p = AllocationPlan::array::<ColliderHandle>(count)?;
    if b.mass.additional.is_some() {
        p.append(AllocationPlan::array::<RigidBodyAdditionalMassProps>(1)?)?;
    }
    Ok(p)
}
fn velocity(b: BodyVelocity) -> RigidBodyVelocity<f32> {
    RigidBodyVelocity {
        linvel: v(b.linear),
        angvel: angular(b.angular),
    }
}
fn from_row(
    b: &RigidBodyState,
    handles: &[Handle],
    permit: &mut AllocationPermit,
) -> ConstructionResult<RigidBody> {
    let mut colliders = permit.vector(handles.len())?;
    colliders.extend(
        handles
            .iter()
            .map(|h| ColliderHandle::from_raw_parts(h.slot, h.generation)),
    );
    let additional = match b.mass.additional {
        None => None,
        Some(m) => Some(permit.boxed(match m {
            AdditionalMass::Mass(bits) => RigidBodyAdditionalMassProps::Mass(f32::from_bits(bits)),
            AdditionalMass::Properties(m) => RigidBodyAdditionalMassProps::MassProps(mass(m)),
        })?),
    };
    #[cfg(feature = "dim2")]
    let inertia = f32::from_bits(b.mass.effective_inverse_inertia[0]);
    #[cfg(feature = "dim3")]
    let inertia = {
        let x = b.mass.effective_inverse_inertia.map(f32::from_bits);
        parry::utils::SdpMatrix3::new(x[0], x[1], x[2], x[3], x[4], x[5])
    };
    Ok(RigidBody {
        ids: RigidBodyIds {
            active_island_id: b.ids[0],
            active_set_id: b.ids[1],
            island_id: b.ids[2],
            island_index: b.ids[3],
        },
        pos: RigidBodyPosition {
            position: pose(b.position),
            next_position: pose(b.next_position),
        },
        damping: RigidBodyDamping {
            linear_damping: f32::from_bits(b.damping[0]),
            angular_damping: f32::from_bits(b.damping[1]),
        },
        vels: velocity(b.velocity),
        forces: RigidBodyForces {
            force: v(b.forces.force),
            torque: angular(b.forces.torque),
            gravity_scale: f32::from_bits(b.forces.gravity_scale),
            user_force: v(b.forces.user_force),
            user_torque: angular(b.forces.user_torque),
            #[cfg(feature = "dim3")]
            gyroscopic_forces_enabled: b.forces.gyroscopic,
        },
        mprops: RigidBodyMassProps {
            world_com: v(b.mass.world_center),
            effective_inv_mass: v(b.mass.effective_inverse_mass),
            effective_world_inv_inertia: inertia,
            local_mprops: mass(b.mass.local),
            flags: LockedAxes::from_bits(
                b.mass
                    .locked_axes
                    .try_into()
                    .map_err(|_| ConstructionError::Payload)?,
            )
            .ok_or(ConstructionError::Payload)?,
            additional_local_mprops: additional,
            max_extent: f32::from_bits(b.mass.max_extent),
        },
        ccd_vels: velocity(b.ccd_velocity),
        ccd: RigidBodyCcd {
            ccd_thickness: f32::from_bits(b.ccd.thickness),
            ccd_active: b.ccd.active,
            ccd_enabled: b.ccd.enabled,
            soft_ccd_prediction: f32::from_bits(b.ccd.soft_prediction),
            allow_fast_rotation: b.ccd.allow_fast_rotation,
        },
        colliders: RigidBodyColliders(colliders),
        activation: RigidBodyActivation {
            normalized_linear_threshold: f32::from_bits(b.activation.linear_threshold),
            angular_threshold: f32::from_bits(b.activation.angular_threshold),
            time_until_sleep: f32::from_bits(b.activation.time_until_sleep),
            time_since_can_sleep: f32::from_bits(b.activation.time_since_can_sleep),
            sleeping: b.activation.sleeping,
            sleep_prev_pose: pose(b.activation.previous_pose),
        },
        changes: RigidBodyChanges::from_bits(b.changes).ok_or(ConstructionError::Payload)?,
        body_type: kind(b.kind),
        dominance: RigidBodyDominance(
            b.dominance
                .try_into()
                .map_err(|_| ConstructionError::Payload)?,
        ),
        enabled: b.enabled,
        additional_solver_iterations: b.additional_iterations[0]
            .try_into()
            .map_err(|_| ConstructionError::Payload)?,
        additional_pgs_iterations: b.additional_iterations[1]
            .try_into()
            .map_err(|_| ConstructionError::Payload)?,
        soft_body: SoftBodyHandle::from_raw_parts(b.soft_body.slot, b.soft_body.generation),
        soft_cluster: b.soft_cluster,
        soft_motion_margin: f32::from_bits(b.soft_motion_margin),
        user_data: b
            .user_data
            .iter()
            .enumerate()
            .fold(0u128, |acc, (i, v)| acc | ((*v as u128) << (i * 32))),
    })
}
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
#[path = "body_tests.rs"]
mod tests;
