use super::*;
use crate::geometry::SolverContact;
use crate::pipeline::checked_restore::convert::{bits, pose_bits};
use incant_physics_snapshot as s;
pub(super) fn point(p: &parry::query::TrackedContact<ContactData>) -> s::TrackedContactPoint {
    let d = &p.data;
    #[cfg(feature = "dim2")]
    let tangent = |v: na::Vector1<f32>| [v[0].to_bits(), 0];
    #[cfg(feature = "dim3")]
    let tangent = |v: na::Vector2<f32>| [v[0].to_bits(), v[1].to_bits()];
    s::TrackedContactPoint {
        local_p1: bits(p.local_p1),
        local_p2: bits(p.local_p2),
        distance: p.dist.to_bits(),
        features: [p.fid1.0, p.fid2.0],
        data: s::ContactWarmstart {
            impulse: d.impulse.to_bits(),
            tangent_impulse: tangent(d.tangent_impulse),
            warmstart_impulse: d.warmstart_impulse.to_bits(),
            warmstart_tangent_impulse: tangent(d.warmstart_tangent_impulse),
            #[cfg(feature = "dim2")]
            warmstart_twist_impulse: 0,
            #[cfg(feature = "dim3")]
            warmstart_twist_impulse: d.warmstart_twist_impulse.to_bits(),
            #[cfg(feature = "dim2")]
            warmstart_tangent_world: [0; 3],
            #[cfg(feature = "dim3")]
            warmstart_tangent_world: bits(d.warmstart_tangent_world),
            solver_dp1: bits(d.solver_dp1),
            solver_dp2: bits(d.solver_dp2),
        },
    }
}
pub(super) fn solver(p: &SolverContact) -> s::SolverContactPoint {
    s::SolverContactPoint {
        anchor1: bits(p.anchor1),
        anchor2: bits(p.anchor2),
        distance: p.dist.to_bits(),
        tangent_velocity: bits(p.tangent_velocity),
        contact_id: p.contact_id[0],
        #[cfg(feature = "dim2")]
        padding: 0,
        #[cfg(feature = "dim3")]
        padding: p.padding[0].to_bits(),
    }
}
pub(super) fn push(t: &mut s::ManifoldTable, m: &ContactManifold) {
    let d = &m.data;
    let points = s::RecordSpan {
        start: t.points.len() as u32,
        count: m.points.len() as u32,
    };
    t.points.extend(m.points.iter().map(point));
    let solver = s::RecordSpan {
        start: t.solver.len() as u32,
        count: d.solver_contacts.len() as u32,
    };
    t.solver.extend(d.solver_contacts.iter().map(self::solver));
    let body = |h: Option<crate::dynamics::RigidBodyHandle>| {
        h.map(|h| crate::pipeline::checked_restore::capture::util::handle(h.into_raw_parts()))
    };
    t.manifolds.push(s::ContactManifoldRecord {
        points,
        solver,
        local_normals: [bits(m.local_n1), bits(m.local_n2)],
        subshapes: [m.subshape1, m.subshape2],
        subshape_poses: m.subshape_poses.as_ref().map(|_| {
            [
                m.subshape_pos1().copied().map(pose_bits),
                m.subshape_pos2().copied().map(pose_bits),
            ]
        }),
        bodies: [body(d.rigid_body1), body(d.rigid_body2)],
        solver_flags: d.solver_flags.bits(),
        solver_color: u32::from(d.solver_color),
        solver_body_ids: d.solver_body_ids,
        graph_position: if d.graph_pos.is_some() {
            (u32::from(d.graph_pos.bucket()) << 22) | d.graph_pos.local()
        } else {
            u32::MAX
        },
        normal: bits(d.normal),
        relative_dominance: i32::from(d.relative_dominance),
        user_data: d.user_data,
        friction: d.friction.to_bits(),
        restitution: d.restitution.to_bits(),
    });
}
