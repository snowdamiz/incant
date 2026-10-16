use super::*;
mod bytecode;
#[cfg(feature = "native-checkpoints")]
mod native;
use incant_input::{InputRuntime, ValidatedActions};

/// Decoded ordinary-load candidate, before any guest initialization. Native hosts
/// can validate/retain bindings against this exact world before starting its VM.
/// Fields are private: callers cannot pair arbitrary worlds, state and metadata.
/// Construction checks immutable source identity and all data-only membership;
/// `start` additionally checks the behavior's actual evaluated persistence opt-in.
pub struct PendingSessionCheckpoint {
    world: NativeWorld,
    code: SessionProgram,
    state: ProgramState,
    #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
    watermark: i64,
    bindings: ValidatedActions,
    restart_requests: bool,
    #[cfg(feature = "ui")]
    ui: Option<incant_ui::PendingUiCheckpoint>,
    #[cfg(feature = "audio")]
    audio: Option<incant_audio_runtime::PendingAudioCheckpoint>,
}
impl PendingSessionCheckpoint {
    /// Only the opaque native checkpoint can supply this already validated,
    /// separately reconstructed world. Public byte decoding still uses decode.
    #[cfg(feature = "native-checkpoints")]
    pub(super) fn from_native(
        world: NativeWorld,
        code: SessionProgram,
        state: ProgramState,
        bindings: ValidatedActions,
    ) -> Self {
        Self {
            world,
            code,
            state,
            #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
            watermark: 0,
            bindings,
            restart_requests: false,
            #[cfg(feature = "ui")]
            ui: None,
            #[cfg(feature = "audio")]
            audio: None,
        }
    }
    pub fn new(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
    ) -> Result<Self> {
        Self::decode(bytes, program.into(), actions, |world| {
            Ok(NativeWorld::from_save_bytes_for_scene(world, initial)?)
        })
    }
    pub(super) fn decode(
        bytes: &[u8],
        code: SessionProgram,
        actions: InputActions,
        load_world: impl FnOnce(&[u8]) -> Result<NativeWorld>,
    ) -> Result<Self> {
        let sections = state::sections(bytes, &code, false)?;
        Self::decode_sections(sections, code, actions, false, load_world)
    }
    pub(in crate::checkpoint) fn decode_sections(
        sections: codec::Sections<'_>,
        code: SessionProgram,
        actions: InputActions,
        persisted: bool,
        load_world: impl FnOnce(&[u8]) -> Result<NativeWorld>,
    ) -> Result<Self> {
        let bindings = ValidatedActions::new(actions).map_err(InputError::from)?;
        let version = sections
            .world
            .get(8..12)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()));
        if if persisted {
            version != Some(12)
        } else {
            !matches!(version, Some(3..=11))
        } {
            return Err(CheckpointError::Invalid(if persisted {
                "persisted session requires world save v12"
            } else {
                "session requires world save v3-v11"
            }));
        }
        let world = load_world(sections.world)?;
        if world.source_scene_fingerprint() != sections.origin {
            return Err(CheckpointError::WorldMismatch);
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
        if (world.requires_audio_checkpoint() && sections.audio.is_none())
            || (!persisted && !world.requires_audio_checkpoint() && sections.audio.is_some())
        {
            return Err(CheckpointError::Invalid("world/audio section pairing"));
        }
        #[cfg(feature = "audio")]
        let audio = sections
            .audio
            .map(|bytes| incant_audio_runtime::PendingAudioCheckpoint::new(&world, bytes))
            .transpose()?;
        #[cfg(feature = "audio")]
        if let Some(audio) = &audio {
            audio.validate_session(&world)?;
        }
        #[cfg(not(feature = "audio"))]
        if sections.audio.is_some() {
            return Err(CheckpointError::Unsupported(
                "hosted audio checkpoint feature",
            ));
        }
        let state = match &code {
            SessionProgram::Bytecode(code) => {
                ProgramState::Bytecode(bytecode::decode(sections, code, &world)?)
            }
            #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
            _ => {
                let bytes = sections.wasm.ok_or(CheckpointError::ProgramMismatch)?;
                let snapshot = incant_wasm_runtime::persistent::PersistentSnapshot::decode(
                    bytes,
                    state::wasm_declaration(&code)?,
                )
                .map_err(|_| CheckpointError::Invalid("persistent Wasm state"))?;
                if snapshot.tick() != world.tick() {
                    return Err(CheckpointError::Invalid("world/module tick pairing"));
                }
                ProgramState::Wasm(snapshot)
            }
        };
        Ok(Self {
            world,
            code,
            state,
            #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
            watermark: 0,
            bindings,
            restart_requests: false,
            #[cfg(feature = "ui")]
            ui,
            #[cfg(feature = "audio")]
            audio,
        })
    }
    pub fn world(&self) -> &NativeWorld {
        &self.world
    }
    fn requires_audio_owner(&self) -> bool {
        #[cfg(feature = "audio")]
        if self.audio.is_some() {
            return true;
        }
        self.world.requires_audio_checkpoint()
    }
    /// Run trusted module initialization; globals/closures are explicitly
    /// ephemeral. No tick occurs and the prepared world identity is preserved.
    pub fn start(self) -> Result<NativeSession> {
        if self.requires_audio_owner() {
            return Err(CheckpointError::Unsupported(
                "restored audio owner is required",
            ));
        }
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
        if self.requires_audio_owner() {
            return Err(CheckpointError::Unsupported(
                "restored audio owner is required",
            ));
        }
        if let Some(state) = &self.ui {
            state.validate_restored(&self.world, ui)?;
        } else {
            ui.checkpoint_for_world(&self.world)?;
        }
        self.start_validated()
    }
    #[cfg(feature = "audio")]
    pub fn audio(&self) -> Option<&incant_audio_runtime::PendingAudioCheckpoint> {
        self.audio.as_ref()
    }
    #[cfg(feature = "audio")]
    pub fn start_with_audio(
        self,
        audio: &incant_audio_runtime::SceneAudio<incant_audio_runtime::OfflineBackend>,
        #[cfg(feature = "ui")] ui: Option<&incant_ui::UiRuntime>,
    ) -> Result<NativeSession> {
        if let Some(state) = &self.audio {
            state.validate_restored(&self.world, audio)?;
        }
        #[cfg(feature = "ui")]
        if let Some(state) = &self.ui {
            state.validate_restored(
                &self.world,
                ui.ok_or(CheckpointError::Unsupported(
                    "restored hosted UI owner is required",
                ))?,
            )?;
        }
        self.start_validated()
    }
    pub(in crate::checkpoint) fn retain_authority(&mut self, watermark: i64, restart: bool) {
        #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
        {
            self.watermark = watermark;
        }
        #[cfg(not(any(feature = "wasmtime-host", feature = "static-module")))]
        let _ = watermark;
        self.restart_requests = restart;
    }
    fn start_validated(self) -> Result<NativeSession> {
        let host = match self.state {
            ProgramState::Bytecode(state) => {
                let code = match self.code {
                    SessionProgram::Bytecode(code) => code,
                    #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
                    _ => return Err(CheckpointError::ProgramMismatch),
                };
                let mut host = ScriptHost::for_checkpoint(code, &self.world, &state.active)?;
                validate(
                    &host,
                    &state.active,
                    &state.store,
                    &state.schedule,
                    &self.world,
                )?;
                if self.restart_requests {
                    host.enable_restart_requests()?;
                }
                *host.persistent.borrow_mut() = state.store;
                host.schedule = state.schedule;
                SessionVm::Bytecode(Box::new(host))
            }
            #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
            ProgramState::Wasm(snapshot) => {
                if snapshot.declaration() != state::wasm_declaration(&self.code)? {
                    return Err(CheckpointError::ProgramMismatch);
                }
                let mut host = self.code.start(&self.world)?;
                match &mut host {
                    #[cfg(feature = "wasmtime-host")]
                    SessionVm::DevelopmentWasm(vm) => vm
                        .restore_persistent(
                            &self.world,
                            &snapshot,
                            self.watermark,
                            self.restart_requests,
                        )
                        .map_err(crate::SessionError::from)?,
                    #[cfg(feature = "static-module")]
                    SessionVm::Static(vm) => vm
                        .restore_persistent(
                            &self.world,
                            &snapshot,
                            self.watermark,
                            self.restart_requests,
                        )
                        .map_err(crate::SessionError::from)?,
                    SessionVm::Bytecode(_) => return Err(CheckpointError::ProgramMismatch),
                }
                host
            }
        };
        let mut input = InputRuntime::default();
        input.map_validated(&self.bindings);
        Ok(NativeSession {
            world: self.world,
            host,
            input,
            bindings: self.bindings,
            failed: false,
        })
    }
}
