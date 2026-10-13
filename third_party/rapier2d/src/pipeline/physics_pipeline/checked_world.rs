//! No public installer: exact retained verdict plus freshly charged solver scratch.
use super::*;
use crate::pipeline::checked_restore::{NumericFrame, allocation::*};
impl PhysicsPipeline {
    pub(crate) fn checkpoint_construction_plan() -> ConstructionResult<AllocationPlan> {
        crate::dynamics::StagedIslandSolver::checkpoint_plan()
    }
    pub(crate) fn from_checkpoint(
        frame: &NumericFrame<'_>,
        p: &mut AllocationPermit,
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))] pool: Option<
            std::sync::Arc<rayon::ThreadPool>,
        >,
    ) -> ConstructionResult<Self> {
        let mut fresh = PhysicsPipeline {
            counters: Counters::new(true),
            #[cfg(feature = "parallel")]
            active_body_handles: vec![],
            sleep_observations: Vec::new(),
            staged_solver: crate::dynamics::StagedIslandSolver::checkpoint_new(p)?,
            #[cfg(feature = "parallel")]
            deferred_bvh: std::sync::Mutex::new(None),
            deferred_bvh_inline: None,
            #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
            thread_pool: None,
            joint_constraint_indices: vec![],
            joint_selection_primed: false,
            broad_phase_events: vec![],
            end_step_collider_aabbs: vec![],
            soft_synced_colliders: vec![],
            quarantine: Quarantine::default(),
        };
        fresh.staged_solver.post_solve_ccd_active =
            frame.boundary().pipeline().record().post_solve_ccd_active;
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
        {
            fresh.thread_pool = pool;
        }
        Ok(fresh)
    }
}
