//! Private typed installation of all three retained cache states.
use super::*;
use crate::pipeline::checked_restore::{NumericFrame, allocation::*, convert::v};
use incant_physics_snapshot::CcdCache;
impl CCDSolver {
    pub(crate) fn checkpoint_plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
        match &frame.boundary().pipeline().context().ccd.record().cache {
            CcdCache::Targets { targets, .. } => {
                AllocationPlan::array::<(ColliderHandle, Aabb)>(targets.len())
            }
            _ => Ok(AllocationPlan::default()),
        }
    }
    pub(crate) fn from_checkpoint(
        frame: &NumericFrame<'_>,
        p: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        let fixed_targets_cache = match &frame.boundary().pipeline().context().ccd.record().cache {
            CcdCache::Uninitialized => None,
            CcdCache::FullBvh { prediction } => Some((f32::from_bits(*prediction), None)),
            CcdCache::Targets {
                prediction,
                targets,
            } => {
                let mut rows = p.vector(targets.len())?;
                for t in targets {
                    rows.push((
                        ColliderHandle::from_raw_parts(t.collider.slot, t.collider.generation),
                        Aabb::new(v(t.mins), v(t.maxs)),
                    ));
                }
                Some((f32::from_bits(*prediction), Some(rows)))
            }
        };
        Ok(Self {
            fixed_targets_cache,
        })
    }
}
