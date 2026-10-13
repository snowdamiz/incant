use super::*;
use crate::query::contact_manifolds::checkpoint::map;
use incant_physics_snapshot as s;
use s::construction::{AllocationPermit, AllocationPlan, ConstructionResult};
impl CompositeShapeCompositeShapeContactManifoldsWorkspace {
    pub(crate) fn checkpoint_capture(
        &self,
        budget: &mut s::Budget,
    ) -> s::Result<s::WorkspaceRecord> {
        map::capture(
            &self.sub_detectors,
            self.timestamp,
            s::WorkspaceKind::CompositeComposite,
            |d| (d.manifold_id, d.timestamp),
            budget,
        )
    }
    pub(crate) fn checkpoint_plan(n: usize) -> ConstructionResult<AllocationPlan> {
        let mut p = map::plan::<(u32, u32), SubDetector>(n)?;
        p.append(AllocationPlan::array::<Self>(1)?)?;
        Ok(p)
    }
    pub(crate) fn checkpoint_construct(
        timestamp: bool,
        rows: &[s::SubDetector],
        p: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        Ok(Self {
            timestamp,
            sub_detectors: map::construct(
                rows,
                |d| SubDetector {
                    manifold_id: d.manifold as usize,
                    timestamp: d.timestamp,
                },
                p,
            )?,
        })
    }
}
