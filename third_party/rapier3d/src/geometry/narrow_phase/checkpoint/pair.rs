use super::*;
use crate::geometry::contact_pair::ContactRecycleState;
use crate::math::Rotation;
use crate::pipeline::checked_restore::convert::pose;
fn rotation(b: [u32; 4]) -> Rotation {
    #[cfg(feature = "dim2")]
    {
        Rotation::from_cos_sin_unchecked(f32::from_bits(b[0]), f32::from_bits(b[1]))
    }
    #[cfg(feature = "dim3")]
    {
        Rotation::from_array(b.map(f32::from_bits))
    }
}
pub(super) fn plan(f: WorldBoundary<'_>) -> ConstructionResult<AllocationPlan> {
    let mut p = AllocationPlan::default();
    for pair in &f.manifolds().record().pairs {
        for span in [pair.plain, pair.clusters] {
            p.append(AllocationPlan::array::<ContactManifold>(
                span.count as usize,
            )?)?;
            for row in span.start..span.start + span.count {
                p.append(manifold::plan(f.manifolds(), row)?)?;
            }
        }
    }
    Ok(p)
}
pub(super) fn construct(
    f: WorldBoundary<'_>,
    row: usize,
    p: &mut AllocationPermit,
) -> ConstructionResult<ContactPair> {
    let r = f.manifolds().record().pairs[row];
    let s = f
        .broad()
        .context()
        .narrow
        .context()
        .rigid
        .pairs()
        .record()
        .pairs[row];
    let mut build =
        |span: incant_physics_snapshot::RecordSpan| -> ConstructionResult<Vec<ContactManifold>> {
            let mut out = p.vector(span.count as usize)?;
            for row in span.start..span.start + span.count {
                out.push(manifold::construct(f.manifolds(), row, p)?);
            }
            Ok(out)
        };
    let manifolds = build(r.plain)?;
    let solver_clusters = build(r.clusters)?;
    Ok(ContactPair {
        collider1: handle(r.colliders[0]),
        collider2: handle(r.colliders[1]),
        event_status: PairEventStatus::from_bits(s.event_status as u8)
            .ok_or(ConstructionError::Payload)?,
        contacts: PairContacts::Rigid(RigidPairContacts {
            manifolds,
            solver_clusters,
            solver_clusters_prev: Vec::new(),
            solver_color: s.solver_color as u8,
            solver_color_bodies: s.solver_color_bodies,
            workspace: None,
            recycle_state: s.recycle.map(|s| ContactRecycleState {
                pos12: pose(s.relative_pose),
                rot1: rotation(s.rotations[0]),
                rot2: rotation(s.rotations[1]),
                max_extent: f32::from_bits(s.max_extent),
                max_drift: f32::from_bits(s.max_drift),
            }),
            soft: None,
        }),
    })
}
