//! Opaque in-memory continuation of native physics and explicitly opted-in data.
use super::*;
#[cfg(any(feature = "ui", feature = "audio"))]
mod hosted;
mod persisted;
mod prepare;
use incant_input::ValidatedActions;
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget, WorldCheckpoint,
};

/// A trusted, process-local checkpoint with separately validated data-file
/// export. File loading creates a PendingSessionCheckpoint, never this capture
/// type. There is no heap snapshot or Clone implementation. Wasm guests must
/// declare bounded host-owned persistent bytes; bytecode uses persistence v1.
/// Physics uses an opaque WorldCheckpoint.
///
/// Reconstruction runs module initialization again. Globals, closures and logs
/// are ephemeral. Validated action definitions are retained, but physical input
/// and its clock restart neutral. External event/command queues must be discarded
/// by the host. Hosted UI/audio owners are captured through typed overloads and
/// must be rebuilt against the prepared candidate before its VM is started.
pub struct NativeSessionCheckpoint {
    world: WorldCheckpoint,
    origin: [u8; 32],
    code: SessionProgram,
    state: ProgramState,
    bindings: ValidatedActions,
    ui: Option<Vec<u8>>,
    audio: Option<Vec<u8>>,
}

impl NativeSession {
    /// Capture a healthy committed script phase and its admitted native physics
    /// boundary without flushing pending ECS inputs. Every active behavior must
    /// opt into its explicit persistence contract. Pending restart requests are refused.
    /// Numeric/structural script transactions have already committed on return
    /// from tick; structural physics changes may still refuse native capture.
    ///
    /// Budgets cover physics only. Script data retains its existing bounded store,
    /// attachment/timer limits and VM limit; ECS keeps MAX_SAVE_BYTES. These are
    /// not a process-wide allocator budget. The public byte-save guards remain.
    pub fn checkpoint(
        &mut self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<NativeSessionCheckpoint> {
        self.capture_native(None, None, payload, planar_work, spatial_work)
    }

    fn capture_native(
        &mut self,
        ui: Option<&[u8]>,
        audio: Option<&[u8]>,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<NativeSessionCheckpoint> {
        let (code, state) = state::capture(self)?;
        if self.world.requires_ui_checkpoint() != ui.is_some() {
            return Err(CheckpointError::Unsupported("hosted UI state is required"));
        }
        if self.world.requires_audio_checkpoint() && audio.is_none() {
            return Err(CheckpointError::Unsupported(
                "retained audio owner is required",
            ));
        }
        let world = self.world.checkpoint(payload, planar_work, spatial_work)?;
        Ok(NativeSessionCheckpoint {
            world,
            origin: self.world.source_scene_fingerprint(),
            code,
            state,
            bindings: self.bindings.clone(),
            ui: ui.map(<[u8]>::to_vec),
            audio: audio.map(<[u8]>::to_vec),
        })
    }

    /// Stage a fresh world, VM and neutral input before replacing any live state.
    /// A failed budget, binding or VM initialization leaves this session intact.
    /// Success changes world identity and incarnations; all external queued work
    /// and retained hosts belong to the previous world and must be discarded.
    /// Captured action definitions replace current bindings. Only this receiving
    /// host's current restart opt-in survives, never a pending restart request.
    pub fn restore_checkpoint(
        &mut self,
        checkpoint: &NativeSessionCheckpoint,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<()> {
        let candidate = checkpoint
            .prepare_for(self, payload, planar_work, spatial_work, allocations)?
            .start()?;
        *self = candidate;
        Ok(())
    }
}

impl NativeSessionCheckpoint {
    /// Construct a new independent session from the actual captured startup code.
    /// The fresh host has restart requests disabled until its owner explicitly
    /// enables them. No update executes until the next normal synchronous tick.
    pub fn reconstruct(
        &self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<NativeSession> {
        self.prepare(payload, planar_work, spatial_work, allocations)?
            .start()
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests;
