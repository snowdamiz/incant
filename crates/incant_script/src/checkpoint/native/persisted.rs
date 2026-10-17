//! Bounded data files. Trusted code, action definitions and immutable catalogs
//! come from the receiving package; hashes never authenticate saved history.
use super::*;

impl NativeSessionCheckpoint {
    /// Encode only the public persisted physics domain. Captured native state
    /// outside that domain refuses even if process-local reconstruction works.
    /// Physics budgets exclude the separately bounded script/UI/audio sections.
    pub fn persisted_bytes(
        &self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<Vec<u8>> {
        let world = self
            .world
            .persisted_bytes(payload, planar_work, spatial_work)?;
        state::encode(
            &self.code,
            &self.state,
            self.origin,
            &world,
            self.ui.as_deref(),
            self.audio.as_deref(),
            true,
        )
    }
}

impl PendingSessionCheckpoint {
    /// Validate a data-only file against the actual trusted package. No VM,
    /// retained host or live session is modified or started during preparation.
    #[allow(clippy::too_many_arguments)] // Independent cumulative physics budgets.
    pub fn from_persisted_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<Self> {
        let code = program.into();
        let sections = state::sections(bytes, &code, true)?;
        let current = NativeWorld::new(initial.clone());
        Self::decode_sections(sections, code, actions, true, |world| {
            Ok(NativeWorld::from_persisted_bytes_for(
                &current,
                world,
                payload,
                planar_work,
                spatial_work,
                allocations,
            )?)
        })
    }
}

impl NativeSession {
    /// Prepare against this healthy receiver before any replacement. Only its
    /// current restart capability survives; a file cannot grant one. Existing
    /// hosted owners must be rebuilt against the returned fresh world.
    pub fn prepare_persisted_restore(
        &self,
        bytes: &[u8],
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<PendingSessionCheckpoint> {
        let (program, watermark, restart) = state::receiver(self)?;
        let sections = state::sections(bytes, &program, true)?;
        let mut pending = PendingSessionCheckpoint::decode_sections(
            sections,
            program,
            self.bindings.definitions().clone(),
            true,
            |world| {
                Ok(NativeWorld::from_persisted_bytes_for(
                    &self.world,
                    world,
                    payload,
                    planar_work,
                    spatial_work,
                    allocations,
                )?)
            },
        )?;
        pending.retain_authority(watermark, restart);
        Ok(pending)
    }

    /// Atomically replace a headless session after complete native, script and
    /// guest initialization checks. Refusal preserves the current live session.
    pub fn restore_persisted_bytes(
        &mut self,
        bytes: &[u8],
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<()> {
        let candidate = self
            .prepare_persisted_restore(bytes, payload, planar_work, spatial_work, allocations)?
            .start()?;
        *self = candidate;
        Ok(())
    }
}
