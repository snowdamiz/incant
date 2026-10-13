//! Exact ordered persistent buckets from a validated joined rigid owner.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::ValidatedRigidOwner;
impl SolverContactGraph {
    pub(crate) fn checkpoint_plan(
        w: &ValidatedRigidOwner<'_>,
    ) -> ConstructionResult<AllocationPlan> {
        let mut p = AllocationPlan::array::<Vec<ContactRef>>(w.record().buckets.len())?;
        for span in &w.record().buckets {
            p.append(AllocationPlan::array::<ContactRef>(span.count as usize)?)?;
        }
        Ok(p)
    }
    pub(crate) fn from_checkpoint(
        w: &ValidatedRigidOwner<'_>,
        p: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        let r = w.record();
        let mut buckets = p.vector(r.buckets.len())?;
        for span in &r.buckets {
            let mut row = p.vector(span.count as usize)?;
            row.extend(
                r.references[span.start as usize..(span.start + span.count) as usize]
                    .iter()
                    .map(|r| ContactRef {
                        edge: r.edge,
                        manifold: r.manifold,
                    }),
            );
            buckets.push(row);
        }
        Ok(Self { buckets })
    }
}
