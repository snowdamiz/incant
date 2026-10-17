//! Ordinary game-load checkpoints for explicitly opted-in persistent program data.
//! Code is supplied by trusted startup; save bytes contain data only.
mod codec;
#[cfg(feature = "native-checkpoints")]
mod native;
mod pending;
mod state;
use crate::{
    CompiledScript, NativeSession, ScriptError, ScriptHost, SessionProgram,
    persistence::Store,
    program::SessionVm,
    schedule::{Schedule, Scope},
};
pub use codec::MAX_CHECKPOINT_BYTES;
#[cfg(feature = "native-checkpoints")]
pub use codec::MAX_PERSISTED_SESSION_BYTES;
use incant_input::{InputActions, InputError};
use incant_runtime::{CookedScene, NativeWorld, SaveError, StableId};
#[cfg(feature = "native-checkpoints")]
pub use native::NativeSessionCheckpoint;
pub use pending::PendingSessionCheckpoint;
use sha2::{Digest, Sha256};
use state::ProgramState;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("invalid native session checkpoint: {0}")]
    Invalid(&'static str),
    #[error("native session checkpoint integrity check failed")]
    Integrity,
    #[error("native session checkpoint program/runtime is incompatible")]
    ProgramMismatch,
    #[error("native session checkpoint belongs to a different original scene")]
    WorldMismatch,
    #[error("native session checkpoint is unsupported: {0}")]
    Unsupported(&'static str),
    #[error("failed native sessions cannot create or restore checkpoints")]
    FailedSession,
    #[error(transparent)]
    Session(#[from] crate::SessionError),
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error(transparent)]
    World(#[from] SaveError),
    #[cfg(feature = "native-checkpoints")]
    #[error(transparent)]
    NativeWorld(#[from] incant_runtime::WorldCheckpointError),
    #[error(transparent)]
    Input(#[from] InputError),
    #[cfg(feature = "ui")]
    #[error(transparent)]
    Ui(#[from] incant_ui::UiCheckpointError),
    #[cfg(feature = "audio")]
    #[error(transparent)]
    Audio(#[from] incant_audio_runtime::AudioError),
}
type Result<T> = std::result::Result<T, CheckpointError>;

fn identity(code: &CompiledScript) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"incant-native-session-ordinary-load-v1");
    h.update(code.metadata.format.to_le_bytes());
    h.update([u8::from(code.allow_source)]);
    for bytes in [
        code.bytes.as_ref(),
        code.hardening.as_ref(),
        code.metadata.engine.as_bytes(),
        code.metadata.build_id.as_bytes(),
        code.metadata.target.as_bytes(),
        env!("INCANT_CHECKPOINT_RUNTIME_ID").as_bytes(),
    ] {
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    match &code.bindings {
        None => h.update([0]),
        Some(b) => {
            h.update([1]);
            h.update(b.fingerprint());
        }
    }
    h.finalize().into()
}
fn host(session: &NativeSession) -> Result<&ScriptHost> {
    if session.failed {
        return Err(CheckpointError::FailedSession);
    }
    match &session.host {
        SessionVm::Bytecode(host) if !host.failed => Ok(host),
        SessionVm::Bytecode(_) => Err(CheckpointError::FailedSession),
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(_) => {
            Err(CheckpointError::Unsupported("development module state ABI"))
        }
        #[cfg(feature = "static-module")]
        SessionVm::Static(_) => Err(CheckpointError::Unsupported("static module state ABI")),
    }
}
fn opted(host: &ScriptHost, scope: Scope) -> bool {
    match (&host._compiled.bindings, scope) {
        (None, Scope::Global) => host.persistent_opt_in[0],
        (Some(b), Scope::Source(id)) => b
            .sources
            .binary_search(&id)
            .ok()
            .is_some_and(|i| host.persistent_opt_in[i]),
        _ => false,
    }
}
fn validate(
    host: &ScriptHost,
    active: &[StableId],
    store: &Store,
    schedule: &Schedule,
    world: &NativeWorld,
) -> Result<()> {
    if schedule.tick != world.tick() {
        return Err(CheckpointError::Invalid("world/script tick pairing"));
    }
    if let Some(bindings) = &host._compiled.bindings {
        for id in active {
            let i = bindings
                .instances
                .binary_search_by_key(id, |i| i.entity)
                .map_err(|_| CheckpointError::Invalid("attachment"))?;
            if !opted(host, Scope::Source(bindings.instances[i].source)) {
                return Err(CheckpointError::Unsupported(
                    "active behavior has not opted into persistence version 1",
                ));
            }
        }
    } else if !active.is_empty() || !opted(host, Scope::Global) {
        return Err(CheckpointError::Unsupported(
            "active behavior has not opted into persistence version 1",
        ));
    }
    for key in store.entries.keys() {
        if !opted(host, key.scope) {
            return Err(CheckpointError::Invalid("persistent source scope"));
        }
        if let Some(id) = key.entity {
            let b = host
                ._compiled
                .bindings
                .as_ref()
                .ok_or(CheckpointError::Invalid("persistent instance"))?;
            let i = b
                .instances
                .binary_search_by_key(&id, |i| i.entity)
                .map_err(|_| CheckpointError::Invalid("persistent instance"))?;
            if key.scope != Scope::Source(b.instances[i].source)
                || active.binary_search(&id).is_err()
            {
                return Err(CheckpointError::Invalid("persistent instance lifetime"));
            }
        }
    }
    for key in schedule.timers.keys() {
        let valid = match (&host._compiled.bindings, key.scope) {
            (None, Scope::Global) => true,
            (Some(b), Scope::Source(id)) => b.sources.binary_search(&id).is_ok(),
            _ => false,
        };
        if !valid {
            return Err(CheckpointError::Invalid("timer source scope"));
        }
    }
    validate_tasks(host._compiled.bindings.as_deref(), active, schedule)?;
    if let Some(tasks) = &schedule.coroutines {
        for (key, task) in &tasks.entries {
            if !opted(host, key.scope)
                || !host.coroutine_catalog.known(
                    host._compiled.bindings.as_deref(),
                    key.scope,
                    &task.routine,
                )
            {
                return Err(CheckpointError::Invalid(
                    "coroutine routine/persistence catalog",
                ));
            }
        }
    }
    Ok(())
}
fn task_live(
    bindings: Option<&crate::ScriptBindings>,
    active: &[StableId],
    scope: Scope,
    entity: Option<StableId>,
) -> bool {
    match (bindings, scope) {
        (None, Scope::Global) => entity.is_none(),
        (Some(b), Scope::Source(source)) => {
            let belongs = |id: &StableId| {
                b.instances
                    .binary_search_by_key(id, |i| i.entity)
                    .ok()
                    .is_some_and(|i| b.instances[i].source == source)
            };
            if let Some(id) = entity {
                active.binary_search(&id).is_ok() && belongs(&id)
            } else {
                active.iter().any(belongs)
            }
        }
        _ => false,
    }
}
fn validate_tasks(
    bindings: Option<&crate::ScriptBindings>,
    active: &[StableId],
    schedule: &Schedule,
) -> Result<()> {
    if let Some(tasks) = &schedule.coroutines {
        for (key, task) in &tasks.entries {
            if !task_live(bindings, active, key.scope, task.entity)
                || task.due <= schedule.tick
                || task.due > crate::schedule::MAX_TICK
            {
                return Err(CheckpointError::Invalid("coroutine source/owner/deadline"));
            }
        }
    }
    Ok(())
}

/// Bounded explicit data only. Entries are immutable; copying their Arcs never
/// retains a guest object, callback or VM. Entity retirement is applied at capture.
struct ScriptState {
    active: Vec<StableId>,
    store: Store,
    schedule: Schedule,
}
fn copy_store(stored: &Store, active: &[StableId]) -> Result<Store> {
    let mut store = Store::default();
    for (key, entry) in &stored.entries {
        if key
            .entity
            .is_none_or(|id| active.binary_search(&id).is_ok())
        {
            store.cost.add(entry.cost.bytes, entry.cost.nodes)?;
            store.entries.insert(*key, std::sync::Arc::clone(entry));
        }
    }
    Ok(store)
}
fn script_state(session: &NativeSession) -> Result<ScriptState> {
    let host = host(session)?;
    if !host.haptic_requests.is_empty() {
        return Err(CheckpointError::Unsupported("pending haptic output"));
    }
    if host.session_request.is_some() {
        return Err(CheckpointError::Unsupported(
            "pending session restart request",
        ));
    }
    let active = host
        .project
        .as_ref()
        .map(|p| p.active_ids(&session.world))
        .transpose()?
        .unwrap_or_default();
    let store = copy_store(&host.persistent.borrow(), &active)?;
    let mut schedule = host.schedule.clone();
    if let Some(tasks) = &mut schedule.coroutines {
        tasks.retain(|key, task| {
            task_live(
                host._compiled.bindings.as_deref(),
                &active,
                key.scope,
                task.entity,
            )
        });
        if tasks.entries.is_empty() {
            schedule.coroutines = None;
        }
    }
    validate(host, &active, &store, &schedule, &session.world)?;
    Ok(ScriptState {
        active,
        store,
        schedule,
    })
}
#[cfg(feature = "native-checkpoints")]
impl ScriptState {
    fn copy(&self) -> Result<Self> {
        Ok(Self {
            active: self.active.clone(),
            store: copy_store(&self.store, &self.active)?,
            schedule: self.schedule.clone(),
        })
    }
}

impl NativeSession {
    /// Save a committed native tick with typed persistent values, live authored
    /// attachments and native timer data. Every active behavior must explicitly
    /// declare `persistence: {version: 1}`. No JS heap, closures, diagnostics,
    /// device audio, solver continuation or physical input is serialized.
    /// Existing world capability refusals remain in force.
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>> {
        if self.world.requires_ui_checkpoint() {
            return Err(CheckpointError::Unsupported("hosted UI state is required"));
        }
        self.encode_checkpoint(None, None)
    }
    /// Capture the exact committed hosted owner together with world and script
    /// state. UI declarations alone never substitute for mutable widget state.
    #[cfg(feature = "ui")]
    pub fn checkpoint_bytes_with_ui(&self, ui: &incant_ui::UiRuntime) -> Result<Vec<u8>> {
        let state = ui.checkpoint_for_world(&self.world)?;
        state.validate_world(&self.world)?;
        if !self.world.requires_ui_checkpoint() {
            // Never-used, empty UI owners have no mutable widget state.
            return self.checkpoint_bytes();
        }
        self.encode_checkpoint(Some(state.bytes()), None)
    }
    #[cfg(feature = "audio")]
    pub fn checkpoint_bytes_with_audio<B: incant_audio_runtime::AudioBackend>(
        &self,
        audio: &incant_audio_runtime::SceneAudio<B>,
        #[cfg(feature = "ui")] ui: Option<&incant_ui::UiRuntime>,
    ) -> Result<Vec<u8>>
    where
        B::Error: std::fmt::Debug,
    {
        let audio_state = audio.checkpoint_for_world(&self.world)?;
        audio_state.validate_session(&self.world)?;
        #[cfg(feature = "ui")]
        let ui_state = ui
            .map(|owner| owner.checkpoint_for_world(&self.world))
            .transpose()?;
        #[cfg(feature = "ui")]
        let ui = ui_state
            .as_ref()
            .filter(|_| self.world.requires_ui_checkpoint())
            .map(|state| state.bytes());
        #[cfg(not(feature = "ui"))]
        let ui = None;
        self.encode_checkpoint(
            ui,
            self.world
                .requires_audio_checkpoint()
                .then_some(audio_state.bytes()),
        )
    }
    fn encode_checkpoint(&self, ui: Option<&[u8]>, audio: Option<&[u8]>) -> Result<Vec<u8>> {
        if self.world.requires_ui_checkpoint() != ui.is_some() {
            return Err(CheckpointError::Unsupported("hosted UI state is required"));
        }
        if self.world.requires_audio_checkpoint() != audio.is_some() {
            return Err(CheckpointError::Unsupported(
                "retained audio owner is required",
            ));
        }
        let (program, state) = state::capture(self)?;
        let world = self.world.save_bytes()?;
        state::encode(
            &program,
            &state,
            self.world.source_scene_fingerprint(),
            &world,
            ui,
            audio,
            false,
        )
    }

    /// Load data using the actual trusted initial scene and startup program.
    /// Immutable scene/rate/catalog bindings are checked before VM execution.
    /// Checksums and origin hashes are compatibility checks, not authentication.
    /// This runs module
    /// initialization again; the opt-in contract makes all module globals and
    /// closures ephemeral. No update callback runs until the next normal tick.
    /// Physical controls and the input clock start neutral, without synthetic
    /// pressed/released edges. Hosts must discard queued OS events and reconnect
    /// devices explicitly. This ordinary game load is not exact input replay.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
    ) -> Result<Self> {
        PendingSessionCheckpoint::new(bytes, initial, program, actions)?.start()
    }

    /// Replace this session only after a complete candidate validates. Failure
    /// preserves its world, VM, input and committed persistent data. Success
    /// creates a new world identity; retained external hosts must be rebuilt.
    /// GameSession/device/export loading is a separate, not-yet-supported route.
    pub fn restore_checkpoint_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let (program, watermark, restart) = state::receiver(self)?;
        let sections = state::sections(bytes, &program, false)?;
        if sections.origin != self.world.source_scene_fingerprint() {
            return Err(CheckpointError::WorldMismatch);
        }
        let mut pending = PendingSessionCheckpoint::decode_sections(
            sections,
            program,
            self.bindings.definitions().clone(),
            false,
            |world| {
                let candidate = NativeWorld::from_save_bytes(world)?;
                self.world.validate_save_compatibility(&candidate)?;
                Ok(candidate)
            },
        )?;
        pending.retain_authority(watermark, restart);
        let candidate = pending.start()?;
        *self = candidate;
        Ok(())
    }
}
#[cfg(all(test, feature = "compiler"))]
mod tests;
