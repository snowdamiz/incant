//! Exact private collider fields, never a public record-to-live constructor.
use super::*;
use crate::geometry::{Group, InteractionTestMode};
use crate::pipeline::checked_restore::{allocation::*, convert::*};
use incant_physics_snapshot as inert;
impl Collider {
    pub(crate) fn checkpoint_payload_plan(
        c: &inert::ColliderState,
    ) -> ConstructionResult<AllocationPlan> {
        match c.mass {
            inert::ColliderMass::Properties(_) => AllocationPlan::array::<MassProperties>(1),
            _ => Ok(AllocationPlan::default()),
        }
    }
    pub(crate) fn from_checkpoint_fields(
        c: &inert::ColliderState,
        shape: SharedShape,
        parent: Option<inert::ParentAttachment>,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        if c.deformable || c.active_hooks != 0 {
            return Err(ConstructionError::Payload);
        }
        Ok(Self {
            coll_type: match c.kind {
                inert::ColliderKind::Solid => ColliderType::Solid,
                inert::ColliderKind::Sensor => ColliderType::Sensor,
            },
            shape,
            mprops: match c.mass {
                inert::ColliderMass::Density(x) => ColliderMassProps::Density(f32::from_bits(x)),
                inert::ColliderMass::Mass(x) => ColliderMassProps::Mass(f32::from_bits(x)),
                inert::ColliderMass::Properties(x) => {
                    ColliderMassProps::MassProperties(permit.boxed(mass(x))?)
                }
            },
            changes: ColliderChanges::from_bits(c.changes).ok_or(ConstructionError::Payload)?,
            parent: parent.map(|p| ColliderParent {
                handle: RigidBodyHandle::from_raw_parts(p.body.slot, p.body.generation),
                pos_wrt_parent: pose(p.local_pose),
            }),
            pos: ColliderPosition(pose(c.position)),
            material: ColliderMaterial {
                friction: f32::from_bits(c.material.friction),
                restitution: f32::from_bits(c.material.restitution),
                friction_combine_rule: rule(c.material.friction_rule),
                restitution_combine_rule: rule(c.material.restitution_rule),
            },
            flags: ColliderFlags {
                active_collision_types: ActiveCollisionTypes::from_bits(
                    c.active_collision_types
                        .try_into()
                        .map_err(|_| ConstructionError::Payload)?,
                )
                .ok_or(ConstructionError::Payload)?,
                collision_groups: groups(c.collision_groups)?,
                solver_groups: groups(c.solver_groups)?,
                active_hooks: ActiveHooks::from_bits(c.active_hooks)
                    .ok_or(ConstructionError::Payload)?,
                active_events: ActiveEvents::from_bits(c.active_events)
                    .ok_or(ConstructionError::Payload)?,
                enabled: match c.enabled {
                    inert::ColliderEnabled::Enabled => ColliderEnabled::Enabled,
                    inert::ColliderEnabled::DisabledByParent => ColliderEnabled::DisabledByParent,
                    inert::ColliderEnabled::Disabled => ColliderEnabled::Disabled,
                },
            },
            contact_skin: f32::from_bits(c.contact_skin),
            contact_force_event_threshold: f32::from_bits(c.force_event_threshold),
            deformable_mesh_ref: None,
            user_data: c
                .user_data
                .iter()
                .enumerate()
                .fold(0u128, |a, (i, v)| a | ((*v as u128) << (i * 32))),
        })
    }
}
fn rule(r: inert::CombineRule) -> CoefficientCombineRule {
    match r {
        inert::CombineRule::Average => CoefficientCombineRule::Average,
        inert::CombineRule::Min => CoefficientCombineRule::Min,
        inert::CombineRule::Multiply => CoefficientCombineRule::Multiply,
        inert::CombineRule::Max => CoefficientCombineRule::Max,
    }
}
fn groups(g: inert::ColliderGroups) -> ConstructionResult<InteractionGroups> {
    Ok(InteractionGroups {
        memberships: Group::from_bits(g.memberships).ok_or(ConstructionError::Payload)?,
        filter: Group::from_bits(g.filter).ok_or(ConstructionError::Payload)?,
        test_mode: match g.test {
            inert::GroupTest::And => InteractionTestMode::And,
            inert::GroupTest::Or => InteractionTestMode::Or,
        },
    })
}
