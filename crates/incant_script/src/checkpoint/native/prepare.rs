//! Fresh exact-world candidates, before host factories and guest initialization.
use super::*;

impl NativeSessionCheckpoint {
    /// Prepare a new world with validated host sections, without evaluating any
    /// guest code. Rebuild required UI/audio owners against this exact world,
    /// then consume the pending checkpoint with the corresponding start method.
    /// Fresh preparation never imports a saved restart capability.
    pub fn prepare(
        &self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<PendingSessionCheckpoint> {
        let world = self
            .world
            .reconstruct(payload, planar_work, spatial_work, allocations)?;
        self.pending(world, false)
    }

    /// Stage compatible restoration for an existing healthy receiver. Its live
    /// state is untouched. Only that receiver's current restart capability is
    /// retained; queued restart requests refuse. The host must discard external
    /// input/command queues and publish the complete candidate atomically.
    pub fn prepare_for(
        &self,
        current: &NativeSession,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<PendingSessionCheckpoint> {
        let host = host(current)?;
        if host.session_request.is_some() {
            return Err(CheckpointError::Unsupported(
                "pending session restart request",
            ));
        }
        if identity(&host._compiled) != identity(&self.code) {
            return Err(CheckpointError::ProgramMismatch);
        }
        // A rewind cannot discard an existing receiver's retained host obligation,
        // even when immutable scene validation would independently refuse later.
        if current.world.requires_ui_checkpoint() && self.ui.is_none() {
            return Err(CheckpointError::Unsupported("hosted UI state is required"));
        }
        if current.world.requires_audio_checkpoint() && self.audio.is_none() {
            return Err(CheckpointError::Unsupported(
                "retained audio owner is required",
            ));
        }
        let world = self.world.reconstruct_for(
            &current.world,
            payload,
            planar_work,
            spatial_work,
            allocations,
        )?;
        self.pending(world, host.restart_requests)
    }

    fn pending(
        &self,
        world: NativeWorld,
        restart_requests: bool,
    ) -> Result<PendingSessionCheckpoint> {
        let mut pending = PendingSessionCheckpoint::from_native(
            world,
            self.code.clone(),
            self.state.copy()?,
            self.bindings.clone(),
        );
        pending.attach_hosted(self.ui.as_deref(), self.audio.as_deref())?;
        pending.retain_restart_requests(restart_requests);
        Ok(pending)
    }
}
