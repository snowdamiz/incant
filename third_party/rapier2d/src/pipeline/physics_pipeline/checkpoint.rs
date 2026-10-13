//! Read-only diagnostics for a separately authenticated checkpoint boundary.
use super::PhysicsPipeline;

/// Copied workspace observations; not proof of a completed or restorable owner.
///
/// In particular, the pipeline does not retain the last timestep or an
/// authenticated step phase. Empty reports do not prove scratch completeness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointPipelineObservation {
    /// Retained staged-solver verdict, including its unavailable state.
    pub post_solve_ccd_active: Option<bool>,
    /// A parallel optimization receiver is owned, even if its task has finished.
    pub deferred_bvh_receiver: bool,
    /// An inline optimization task is owned and has not been joined.
    pub deferred_bvh_inline: bool,
    /// Number of rigid-body quarantine reports from the last step.
    pub quarantine_bodies: usize,
    /// Number of collider quarantine reports from the last step.
    pub quarantine_colliders: usize,
    /// Number of soft-body quarantine reports from the last step.
    pub quarantine_soft_bodies: usize,
    /// Unconsumed end-of-step rigid-body quarantine work.
    pub quarantine_body_workspace: usize,
    /// Unconsumed end-of-step collider quarantine work.
    pub quarantine_collider_workspace: usize,
}

/// Observation cannot inspect poisoned deferred ownership without changing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointPipelineError {
    /// The deferred-work mutex is poisoned; its contents are not accessed.
    PoisonedDeferredBvh,
}

impl PhysicsPipeline {
    /// Observes fixed-size workspace metadata without allocation or solver work.
    ///
    /// Exclusive access only permits inspecting the deferred mutex via
    /// `get_mut`: this never locks, receives, takes, clears poison, runs or joins
    /// a task. The returned value authenticates neither phase nor world identity
    /// and must not be used alone to admit restoration.
    pub fn checkpoint_observation(
        &mut self,
    ) -> Result<CheckpointPipelineObservation, CheckpointPipelineError> {
        #[cfg(feature = "parallel")]
        let deferred_bvh_receiver = self
            .deferred_bvh
            .get_mut()
            .map_err(|_| CheckpointPipelineError::PoisonedDeferredBvh)?
            .is_some();
        #[cfg(not(feature = "parallel"))]
        let deferred_bvh_receiver = false;
        Ok(CheckpointPipelineObservation {
            post_solve_ccd_active: self.staged_solver.post_solve_ccd_active,
            deferred_bvh_receiver,
            deferred_bvh_inline: self.deferred_bvh_inline.is_some(),
            quarantine_bodies: self.quarantine.bodies().len(),
            quarantine_colliders: self.quarantine.colliders().len(),
            quarantine_soft_bodies: self.quarantine.soft_bodies().len(),
            quarantine_body_workspace: self.quarantine.body_workspace.len(),
            quarantine_collider_workspace: self.quarantine.collider_workspace.len(),
        })
    }
}
