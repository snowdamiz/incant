//! Trusted live-to-live workspace experiment, never an input decoder.
use super::{CheckpointPipelineError, PhysicsPipeline};

/// A fresh scratch workspace cannot replace outstanding or quarantined work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointWorkspaceError {
    /// The read-only observer refused ownership inspection.
    Observation(CheckpointPipelineError),
    /// An inline task or parallel receiver is still owned.
    DeferredBvh,
    /// A quarantine report or unconsumed quarantine workspace is present.
    Quarantine,
}

impl PhysicsPipeline {
    /// Creates fresh scratch for a trusted, independently completed live owner.
    ///
    /// This diagnostic accepts no serialized state and authenticates no phase.
    /// Its caller must establish a completed positive step and preserve the other
    /// native owners. It retains the exact staged CCD verdict, including `None`;
    /// the configured execution pool is shared, never serialized or recreated.
    /// Counters and all remaining scratch start at `new()` defaults. This is an
    /// experiment for proving scratch overwrite, not a hostile restore API.
    ///
    /// Refusal does not join/take deferred work, clear poison or quarantine, or
    /// mutate the source pipeline. A fresh unstepped pipeline may also satisfy
    /// these checks; absence of outstanding work is not phase authentication.
    pub fn checkpoint_fresh_workspace(&mut self) -> Result<Self, CheckpointWorkspaceError> {
        let observation = self
            .checkpoint_observation()
            .map_err(CheckpointWorkspaceError::Observation)?;
        if observation.deferred_bvh_receiver || observation.deferred_bvh_inline {
            return Err(CheckpointWorkspaceError::DeferredBvh);
        }
        if observation.quarantine_bodies != 0
            || observation.quarantine_colliders != 0
            || observation.quarantine_soft_bodies != 0
            || observation.quarantine_body_workspace != 0
            || observation.quarantine_collider_workspace != 0
        {
            return Err(CheckpointWorkspaceError::Quarantine);
        }
        let mut fresh = Self::new();
        fresh.staged_solver.post_solve_ccd_active = observation.post_solve_ccd_active;
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
        {
            fresh.thread_pool = self.thread_pool.clone();
        }
        Ok(fresh)
    }
}
