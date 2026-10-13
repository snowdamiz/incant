//! Opaque in-memory continuation of native physics and explicitly opted-in data.
use super::*;
use incant_input::ValidatedActions;
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget, WorldCheckpoint,
};

/// A trusted, process-local checkpoint. There is deliberately no byte importer,
/// heap snapshot, Clone implementation, or Wasm state admission. Physics state
/// is owned by an opaque WorldCheckpoint; script state uses persistence version 1.
///
/// Reconstruction runs module initialization again. Globals, closures and logs
/// are ephemeral. Validated action definitions are retained, but physical input
/// and its clock restart neutral. External event/command queues must be discarded
/// by the host. Hosted UI/audio state requires a separate composed capability and
/// is refused here, including worlds with sticky prior-use obligations.
pub struct NativeSessionCheckpoint {
    world: WorldCheckpoint,
    code: CompiledScript,
    state: ScriptState,
    bindings: ValidatedActions,
}

fn headless(world: &NativeWorld) -> Result<()> {
    if world.requires_ui_checkpoint() {
        return Err(CheckpointError::Unsupported("hosted UI state is required"));
    }
    if world.requires_audio_checkpoint() {
        return Err(CheckpointError::Unsupported(
            "retained audio owner is required",
        ));
    }
    Ok(())
}

impl NativeSession {
    /// Capture a healthy committed script phase and its admitted native physics
    /// boundary without flushing pending ECS inputs. Every active behavior must
    /// opt into persistence version 1. Pending restart requests are refused.
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
        let code = host(self)?._compiled.clone();
        headless(&self.world)?;
        let state = script_state(self)?;
        let world = self.world.checkpoint(payload, planar_work, spatial_work)?;
        Ok(NativeSessionCheckpoint {
            world,
            code,
            state,
            bindings: self.bindings.clone(),
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
        let current = host(self)?;
        if current.session_request.is_some() {
            return Err(CheckpointError::Unsupported(
                "pending session restart request",
            ));
        }
        if identity(&current._compiled) != identity(&checkpoint.code) {
            return Err(CheckpointError::ProgramMismatch);
        }
        let restart_requests = current.restart_requests;
        headless(&self.world)?;
        let world = checkpoint.world.reconstruct_for(
            &self.world,
            payload,
            planar_work,
            spatial_work,
            allocations,
        )?;
        let mut candidate = checkpoint.start(world)?;
        if restart_requests {
            // The same host already opted into whole-session replacement.
            match &mut candidate.host {
                SessionVm::Bytecode(host) => host.enable_restart_requests()?,
                #[cfg(feature = "wasmtime-host")]
                SessionVm::DevelopmentWasm(_) => unreachable!("bytecode checkpoint"),
                #[cfg(feature = "static-module")]
                SessionVm::Static(_) => unreachable!("bytecode checkpoint"),
            }
        }
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
        let world = self
            .world
            .reconstruct(payload, planar_work, spatial_work, allocations)?;
        self.start(world)
    }

    fn start(&self, world: NativeWorld) -> Result<NativeSession> {
        headless(&world)?;
        PendingSessionCheckpoint::from_native(
            world,
            self.code.clone(),
            self.state.copy()?,
            self.bindings.clone(),
        )
        .start()
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests;
