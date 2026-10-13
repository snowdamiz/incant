use super::*;
use crate::dynamics::RigidBodyHandle;
use crate::dynamics::solver::solver_contact_graph::GraphPos;
use crate::geometry::{ContactData, ContactManifoldData, SolverContact};
use crate::pipeline::checked_restore::convert::{pose, v};
use incant_physics_snapshot::{SolverContactPoint, TrackedContactPoint, ValidatedManifolds};
use parry::query::TrackedContact;
use parry::shape::PackedFeatureId;
fn point(r: &TrackedContactPoint) -> TrackedContact<ContactData> {
    let d = r.data;
    #[cfg(feature = "dim2")]
    let tangent = |v: [u32; 2]| na::Vector1::new(f32::from_bits(v[0]));
    #[cfg(feature = "dim3")]
    let tangent = |v: [u32; 2]| na::Vector2::new(f32::from_bits(v[0]), f32::from_bits(v[1]));
    TrackedContact {
        local_p1: v(r.local_p1),
        local_p2: v(r.local_p2),
        dist: f32::from_bits(r.distance),
        fid1: PackedFeatureId(r.features[0]),
        fid2: PackedFeatureId(r.features[1]),
        data: ContactData {
            impulse: f32::from_bits(d.impulse),
            tangent_impulse: tangent(d.tangent_impulse),
            warmstart_impulse: f32::from_bits(d.warmstart_impulse),
            warmstart_tangent_impulse: tangent(d.warmstart_tangent_impulse),
            #[cfg(feature = "dim3")]
            warmstart_twist_impulse: f32::from_bits(d.warmstart_twist_impulse),
            #[cfg(feature = "dim3")]
            warmstart_tangent_world: v(d.warmstart_tangent_world),
            solver_dp1: v(d.solver_dp1),
            solver_dp2: v(d.solver_dp2),
        },
    }
}
fn solver(r: &SolverContactPoint) -> SolverContact {
    SolverContact {
        anchor1: v(r.anchor1),
        anchor2: v(r.anchor2),
        dist: f32::from_bits(r.distance),
        tangent_velocity: v(r.tangent_velocity),
        contact_id: [r.contact_id],
        #[cfg(feature = "dim3")]
        padding: [f32::from_bits(r.padding)],
    }
}
// The public field's payload type is not reexported by Parry. Infer its exact
// layout without adding a vendor API or approximating the Box's request.
fn subshape_plan() -> ConstructionResult<AllocationPlan> {
    fn layout<T>(_: &Option<Box<T>>) -> ConstructionResult<AllocationPlan> {
        AllocationPlan::array::<T>(1)
    }
    layout(&ContactManifold::new().subshape_poses)
}
pub(super) fn plan(w: &ValidatedManifolds<'_>, row: u32) -> ConstructionResult<AllocationPlan> {
    let r = &w.record().manifolds[row as usize];
    let mut p = AllocationPlan::default();
    #[cfg(feature = "dim3")]
    {
        p.append(AllocationPlan::array::<TrackedContact<ContactData>>(
            r.points.count as usize,
        )?)?;
        p.append(AllocationPlan::array::<SolverContact>(
            r.solver.count as usize,
        )?)?;
    }
    if r.subshape_poses.is_some() {
        p.append(subshape_plan()?)?;
    }
    Ok(p)
}
pub(super) fn construct(
    w: &ValidatedManifolds<'_>,
    row: u32,
    p: &mut AllocationPermit,
) -> ConstructionResult<ContactManifold> {
    let r = &w.record().manifolds[row as usize];
    #[cfg(feature = "dim2")]
    let (mut points, mut contacts) = (
        arrayvec::ArrayVec::<_, 2>::new(),
        arrayvec::ArrayVec::<_, 2>::new(),
    );
    #[cfg(feature = "dim3")]
    let (mut points, mut contacts) = (
        p.vector(r.points.count as usize)?,
        p.vector(r.solver.count as usize)?,
    );
    points.extend(
        w.manifold_points(row)
            .ok_or(ConstructionError::Payload)?
            .iter()
            .map(point),
    );
    contacts.extend(
        w.manifold_solver(row)
            .ok_or(ConstructionError::Payload)?
            .iter()
            .map(solver),
    );
    let subshape_poses = match r.subshape_poses {
        None => None,
        Some(s) => {
            p.charge(subshape_plan()?)?;
            let mut slot = ContactManifold::new();
            // Allocate exactly once using the native setter, including the
            // distinct Some([None,None]) state. These temporary values are
            // overwritten before publication; no saved bits are normalized.
            slot.set_subshape_pos1(Some(crate::math::Pose::IDENTITY));
            slot.set_subshape_pos1(s[0].map(pose));
            slot.set_subshape_pos2(s[1].map(pose));
            slot.subshape_poses
        }
    };
    let body = |h: Option<incant_physics_snapshot::Handle>| {
        h.map(|h| RigidBodyHandle::from_raw_parts(h.slot, h.generation))
    };
    let graph_pos = if r.graph_position == u32::MAX {
        GraphPos::NONE
    } else {
        GraphPos::new(
            (r.graph_position >> 22) as u16,
            r.graph_position & ((1 << 22) - 1),
        )
    };
    Ok(ContactManifold {
        points,
        local_n1: v(r.local_normals[0]),
        local_n2: v(r.local_normals[1]),
        subshape1: r.subshapes[0],
        subshape2: r.subshapes[1],
        subshape_poses,
        data: ContactManifoldData {
            rigid_body1: body(r.bodies[0]),
            rigid_body2: body(r.bodies[1]),
            solver_flags: SolverFlags::from_bits(r.solver_flags)
                .ok_or(ConstructionError::Payload)?,
            solver_color: r.solver_color as u8,
            solver_body_ids: r.solver_body_ids,
            graph_pos,
            normal: v(r.normal),
            solver_contacts: contacts,
            relative_dominance: r.relative_dominance as i16,
            user_data: r.user_data,
            friction: f32::from_bits(r.friction),
            restitution: f32::from_bits(r.restitution),
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::checked_restore::allocation_test_meter::measure;
    #[test]
    fn checked_narrow_native_subshape_box_charge_and_absent_pose_identity() {
        let plan = subshape_plan().unwrap();
        let (m, bytes, calls) = measure(|| {
            let mut m = ContactManifold::new();
            m.set_subshape_pos1(Some(crate::math::Pose::IDENTITY));
            m.set_subshape_pos1(None);
            m.set_subshape_pos2(None);
            m
        });
        assert!(m.subshape_poses.is_some());
        assert!(m.subshape_pos1().is_none() && m.subshape_pos2().is_none());
        assert_eq!(bytes, plan.bytes());
        assert_eq!(calls, plan.allocations());
        assert_eq!(calls, 1);
        #[cfg(feature = "serde-serialize")]
        assert_ne!(
            bincode::serialize(&m).unwrap(),
            bincode::serialize(&ContactManifold::new()).unwrap()
        );
        std::eprintln!(
            "CHECKED_NARROW_SUBSHAPE dimension={} actual_bytes={bytes} charged_bytes={} allocations={calls} some_none_distinct=true",
            crate::math::DIM,
            plan.bytes()
        );
    }
}
