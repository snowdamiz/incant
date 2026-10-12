use super::*;
use incant_input::{InputRuntime, ValidatedActions};

/// Decoded ordinary-load candidate, before any guest initialization. Native hosts
/// can validate/retain bindings against this exact world before starting its VM.
/// Fields are private: callers cannot pair arbitrary worlds, state and metadata.
/// Construction checks immutable source identity and all data-only membership;
/// `start` additionally checks the behavior's actual evaluated persistence opt-in.
pub struct PendingSessionCheckpoint {
    world: NativeWorld,
    code: CompiledScript,
    active: Vec<StableId>,
    store: Store,
    schedule: Schedule,
    bindings: ValidatedActions,
    #[cfg(feature = "ui")]
    ui: Option<incant_ui::PendingUiCheckpoint>,
}
impl PendingSessionCheckpoint {
    pub fn new(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
    ) -> Result<Self> {
        Self::decode(bytes, bytecode(program.into())?, actions, |world| {
            Ok(NativeWorld::from_save_bytes_for_scene(world, initial)?)
        })
    }
    pub(super) fn decode(
        bytes: &[u8],
        code: CompiledScript,
        actions: InputActions,
        load_world: impl FnOnce(&[u8]) -> Result<NativeWorld>,
    ) -> Result<Self> {
        let sections = codec::sections(bytes, identity(&code))?;
        let active = codec::active(sections.active)?;
        let store = codec::store(sections.store)?;
        let schedule = codec::schedule(sections.schedule)?;
        let bindings = ValidatedActions::new(actions).map_err(InputError::from)?;
        let version = sections
            .world
            .get(8..12)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()));
        if !matches!(version, Some(3..=7)) {
            return Err(CheckpointError::Invalid(
                "session requires world save v3-v7",
            ));
        }
        let world = load_world(sections.world)?;
        if world.source_scene_fingerprint() != sections.origin {
            return Err(CheckpointError::WorldMismatch);
        }
        if schedule.tick != world.tick() {
            return Err(CheckpointError::Invalid("world/script tick pairing"));
        }
        if world.requires_ui_checkpoint() != sections.ui.is_some() {
            return Err(CheckpointError::Invalid("world/UI section pairing"));
        }
        #[cfg(feature = "ui")]
        let ui = sections
            .ui
            .map(|bytes| incant_ui::PendingUiCheckpoint::new(&world, bytes))
            .transpose()?;
        #[cfg(not(feature = "ui"))]
        if sections.ui.is_some() {
            return Err(CheckpointError::Unsupported("hosted UI checkpoint feature"));
        }
        let scope_known = |scope| match (&code.bindings, scope) {
            (None, Scope::Global) => true,
            (Some(b), Scope::Source(id)) => b.sources.binary_search(&id).is_ok(),
            _ => false,
        };
        match &code.bindings {
            Some(b) => {
                if b.scene_sha256 != world.source_scene_fingerprint()
                    || active.iter().any(|id| {
                        b.instances.binary_search_by_key(id, |i| i.entity).is_err()
                            || world.entity_identity(*id).is_none()
                    })
                {
                    return Err(CheckpointError::Invalid("active attachment catalog"));
                }
            }
            None if !active.is_empty() => {
                return Err(CheckpointError::Invalid("active attachments"));
            }
            None => {}
        }
        for key in store.entries.keys() {
            if !scope_known(key.scope) {
                return Err(CheckpointError::Invalid("persistent source scope"));
            }
            if let Some(id) = key.entity {
                let b = code
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
        if schedule.timers.keys().any(|k| !scope_known(k.scope)) {
            return Err(CheckpointError::Invalid("timer source scope"));
        }
        Ok(Self {
            world,
            code,
            active,
            store,
            schedule,
            bindings,
            #[cfg(feature = "ui")]
            ui,
        })
    }
    pub fn world(&self) -> &NativeWorld {
        &self.world
    }
    /// Run trusted module initialization; globals/closures are explicitly
    /// ephemeral. No tick occurs and the prepared world identity is preserved.
    pub fn start(self) -> Result<NativeSession> {
        if self.world.requires_ui_checkpoint() {
            return Err(CheckpointError::Unsupported(
                "restored hosted UI owner is required",
            ));
        }
        self.start_validated()
    }
    #[cfg(feature = "ui")]
    pub fn ui(&self) -> Option<&incant_ui::PendingUiCheckpoint> {
        self.ui.as_ref()
    }
    /// The owner must be reconstructed for this exact fresh world before any
    /// guest initialization. A same-ID owner from another world cannot pass.
    #[cfg(feature = "ui")]
    pub fn start_with_ui(self, ui: &incant_ui::UiRuntime) -> Result<NativeSession> {
        if let Some(state) = &self.ui {
            state.validate_restored(&self.world, ui)?;
        } else {
            ui.checkpoint_for_world(&self.world)?;
        }
        self.start_validated()
    }
    fn start_validated(self) -> Result<NativeSession> {
        let mut host = ScriptHost::for_checkpoint(self.code, &self.world, &self.active)?;
        validate(
            &host,
            &self.active,
            &self.store,
            &self.schedule,
            &self.world,
        )?;
        *host.persistent.borrow_mut() = self.store;
        host.schedule = self.schedule;
        let mut input = InputRuntime::default();
        input.map_validated(&self.bindings);
        Ok(NativeSession {
            world: self.world,
            host: SessionVm::Bytecode(Box::new(host)),
            input,
            bindings: self.bindings,
            failed: false,
        })
    }
}
