//! Fixed-tick native session. Physical input is transient and is not a save/VM
//! snapshot. A valid packet advances simulation before the synchronous script.
use crate::{ScriptError, SessionProgram, program::SessionVm};
use incant_input::{
    InputActions, InputError, InputEvent, InputFrame, InputRuntime, ValidatedActions,
};
use incant_runtime::{CookedScene, NativeWorld, RuntimeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("this program has no native UI snapshot capability")]
    UiUnsupported,
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[cfg(feature = "static-module")]
    #[error(transparent)]
    Module(#[from] incant_wasm_runtime::StaticError),
    #[cfg(feature = "wasmtime-host")]
    #[error(transparent)]
    DevelopmentModule(#[from] incant_wasm_dev::ModuleError),
    #[error("native session failed; recreate it before continuing or accessing saveable state")]
    Failed,
}

/// Owns a fresh scene, VM and physical-input timeline. No authoring state or
/// per-tick project/JSON projection. Invalid packets preserve all state and may
/// be retried. Once valid input advances, any runtime/script failure permanently
/// fails this session: the native step is not rolled back, and no world/save or
/// input access is returned. Each guest retains its own bounded write transaction;
/// there is no rollback of the already accepted input or native simulation step.
pub struct NativeSession {
    pub(crate) world: NativeWorld,
    pub(crate) host: SessionVm,
    pub(crate) input: InputRuntime,
    pub(crate) bindings: ValidatedActions,
    pub(crate) failed: bool,
}

/// A fresh native world and validated input bindings, before guest execution.
/// Native hosts may prepare retained bindings against this exact world. Starting
/// preserves its identity across the move; no mutable world or tick is exposed.
pub struct PendingNativeSession {
    world: NativeWorld,
    input: InputRuntime,
    bindings: ValidatedActions,
}
impl PendingNativeSession {
    pub fn new(scene: CookedScene, actions: InputActions) -> Result<Self, SessionError> {
        let bindings = ValidatedActions::new(actions).map_err(InputError::from)?;
        let mut input = InputRuntime::default();
        input.map_validated(&bindings);
        Ok(Self {
            world: NativeWorld::new(scene),
            input,
            bindings,
        })
    }
    pub fn world(&self) -> &NativeWorld {
        &self.world
    }
    pub fn start(self, program: impl Into<SessionProgram>) -> Result<NativeSession, SessionError> {
        let host = program.into().start(&self.world)?;
        Ok(NativeSession {
            world: self.world,
            host,
            input: self.input,
            bindings: self.bindings,
            failed: false,
        })
    }
}

impl NativeSession {
    pub fn new(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
    ) -> Result<Self, SessionError> {
        PendingNativeSession::new(scene, actions)?.start(program)
    }

    fn ready(&self) -> Result<(), SessionError> {
        if self.failed {
            Err(SessionError::Failed)
        } else {
            Ok(())
        }
    }

    pub fn is_failed(&self) -> bool {
        self.failed
    }

    pub fn world(&self) -> Result<&NativeWorld, SessionError> {
        self.ready()?;
        Ok(&self.world)
    }

    pub fn input(&self) -> Result<&InputFrame, SessionError> {
        self.ready()?;
        Ok(self.input.frame())
    }

    /// Committed bounded diagnostics; terminal failures hide all session outputs.
    pub fn logs(&self) -> Result<&crate::NativeLogs, SessionError> {
        self.ready()?;
        self.host.logs()
    }

    /// Validate once and reseed from held controls without inventing edges.
    /// An invalid rebind leaves the existing bindings and input frame unchanged.
    pub fn rebind(&mut self, actions: InputActions) -> Result<(), SessionError> {
        self.ready()?;
        let bindings = ValidatedActions::new(actions).map_err(InputError::from)?;
        self.input.map_validated(&bindings);
        self.bindings = bindings;
        Ok(())
    }

    pub fn tick(&mut self, events: &[InputEvent]) -> Result<(), SessionError> {
        self.tick_with_ui(events, incant_input::UiSnapshot::default())
    }

    /// Trusted composed UI host input. The host stages widget state before this
    /// call and publishes it only with the successfully accepted native tick.
    pub fn tick_with_ui(
        &mut self,
        events: &[InputEvent],
        ui: incant_input::UiSnapshot,
    ) -> Result<(), SessionError> {
        self.ready()?;
        if !self.host.supports_ui() && !ui.is_empty() {
            return Err(SessionError::UiUnsupported);
        }
        let dt = 1. / f64::from(self.world.tick_rate());
        self.input
            .advance_validated_with_ui(events, dt, &self.bindings, ui)?;
        // Any failure after packet acceptance is terminal. There is deliberately
        // no second whole-world/VM rollback or partially resumable session.
        self.failed = true;
        self.world.step()?;
        self.host.tick(&mut self.world, dt, &self.input)?;
        self.failed = false;
        Ok(())
    }
}

#[cfg(all(test, feature = "compiler"))]
#[path = "session_tests.rs"]
mod tests;
