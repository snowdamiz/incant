//! Programs accepted by the existing native session lifecycle.
use crate::{CompiledScript, ScriptHost, SessionError};
use incant_input::InputRuntime;
use incant_runtime::NativeWorld;

/// A precompiled program selected by trusted startup code. Bytecode retains its
/// build-produced trust boundary; static modules contain reviewed native function
/// descriptors, never pointers or executable bytes deserialized from a project.
///
/// Static `incant_v1` numeric/structural and `incant_input_v1` read-only snapshot
/// capabilities use the same accepted native input timeline. Additive
/// `incant_flipbook_v1` exposes bounded committed playback and queued controls.
// This startup value is consumed immediately, never retained in a collection
// or on the tick path. Boxing CompiledScript would add a second transient heap
// allocation before the persistent SessionVm allocation.
#[allow(clippy::large_enum_variant)]
pub enum SessionProgram {
    Bytecode(CompiledScript),
    #[cfg(feature = "static-module")]
    Static(incant_wasm_runtime::ModuleExports),
}
impl From<CompiledScript> for SessionProgram {
    fn from(value: CompiledScript) -> Self {
        Self::Bytecode(value)
    }
}
#[cfg(feature = "static-module")]
impl From<incant_wasm_runtime::ModuleExports> for SessionProgram {
    fn from(value: incant_wasm_runtime::ModuleExports) -> Self {
        Self::Static(value)
    }
}
impl SessionProgram {
    pub(crate) fn start(self, world: &NativeWorld) -> Result<SessionVm, SessionError> {
        match self {
            Self::Bytecode(script) => Ok(SessionVm::Bytecode(Box::new(ScriptHost::for_world(
                script, world,
            )?))),
            #[cfg(feature = "static-module")]
            Self::Static(module) => Ok(SessionVm::Static(Box::new(
                incant_wasm_runtime::StaticModule::new(module)?,
            ))),
        }
    }
}
// One startup allocation keeps the session size independent of either VM's
// private state. Tick dispatch creates no additional allocation or mutation path.
pub(crate) enum SessionVm {
    Bytecode(Box<ScriptHost>),
    #[cfg(feature = "static-module")]
    Static(Box<incant_wasm_runtime::StaticModule>),
}
impl SessionVm {
    pub fn logs(&self) -> Result<&crate::NativeLogs, SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.logs()?),
            #[cfg(feature = "static-module")]
            Self::Static(_) => Ok(&crate::logging::NO_MODULE_LOGS),
        }
    }

    pub fn tick(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &InputRuntime,
    ) -> Result<(), SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.tick_with_input(world, dt, input)?),
            #[cfg(feature = "static-module")]
            Self::Static(host) => Ok(host.tick_with_input(world, dt, input)?),
        }
    }
}
