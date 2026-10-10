//! Fixed-tick native session. Physical input is transient and is not a save/VM
//! snapshot. A valid packet advances simulation before the synchronous script.
use crate::{CompiledScript, ScriptError, ScriptHost};
use incant_input::{
    InputActions, InputError, InputEvent, InputFrame, InputRuntime, ValidatedActions,
};
use incant_runtime::{CookedScene, NativeWorld, RuntimeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error("native session failed; recreate it before continuing or accessing saveable state")]
    Failed,
}

/// Owns a fresh scene, VM and physical-input timeline. No authoring state or
/// per-tick project/JSON projection. Invalid packets preserve all state and may
/// be retried. Once valid input advances, any runtime/script failure permanently
/// fails this session: the native step is not rolled back, and no world/save or
/// input access is returned. Script writes retain the host's atomic rollback.
pub struct NativeSession {
    world: NativeWorld,
    host: ScriptHost,
    input: InputRuntime,
    bindings: ValidatedActions,
    failed: bool,
}

impl NativeSession {
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
    ) -> Result<Self, SessionError> {
        let bindings = ValidatedActions::new(actions).map_err(InputError::from)?;
        let mut input = InputRuntime::default();
        input.map_validated(&bindings);
        Ok(Self {
            world: NativeWorld::new(scene),
            host: ScriptHost::new(script)?,
            input,
            bindings,
            failed: false,
        })
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
        self.ready()?;
        let dt = 1. / f64::from(self.world.tick_rate());
        self.input.advance_validated(events, dt, &self.bindings)?;
        // Any failure after packet acceptance is terminal. There is deliberately
        // no second whole-world/VM rollback or partially resumable session.
        self.failed = true;
        self.world.step()?;
        self.host
            .tick_with_input(&mut self.world, dt, &self.input)?;
        self.failed = false;
        Ok(())
    }
}

#[cfg(all(test, feature = "compiler"))]
#[path = "session_tests.rs"]
mod tests;
