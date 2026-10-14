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

impl SolverContactGraph {
    pub(crate) fn checkpoint_capture(
        &self,
        budget: &mut incant_physics_snapshot::Budget,
    ) -> incant_physics_snapshot::Result<(
        Vec<incant_physics_snapshot::RecordSpan>,
        Vec<incant_physics_snapshot::SolverReference>,
    )> {
        use crate::pipeline::checked_restore::capture::util::{count, total};
        use incant_physics_snapshot::{RecordSpan, SolverReference};
        count(self.buckets.len())?;
        let n = total(self.buckets.iter().map(|b| b.len()))?;
        let mut buckets = budget.vector(self.buckets.len())?;
        let mut references = budget.vector(n)?;
        for bucket in &self.buckets {
            buckets.push(RecordSpan {
                start: references.len() as u32,
                count: bucket.len() as u32,
            });
            references.extend(bucket.iter().map(|v| SolverReference {
                edge: v.edge,
                manifold: v.manifold,
            }));
        }
        Ok((buckets, references))
    }
}
