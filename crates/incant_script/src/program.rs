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
/// `incant_log_v1` publishes copied primitive diagnostics only after acceptance.
// This trusted startup data may be retained by a cold-restart seed, but is never
// copied on the tick path. Cloning it never copies a live VM or its capabilities.
#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum SessionProgram {
    Bytecode(CompiledScript),
    #[cfg(feature = "wasmtime-host")]
    DevelopmentWasm(Vec<u8>),
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
    /// Cold data provenance for a prepared behavior. It never authorizes loading
    /// executable bytes. Static modules lack a serialized artifact and refuse.
    pub fn artifact_sha256(&self) -> Option<[u8; 32]> {
        use sha2::{Digest, Sha256};
        match self {
            Self::Bytecode(script) => {
                let metadata = script.metadata();
                let mut hash = Sha256::new();
                hash.update(b"Incant bytecode program provenance v1\0");
                hash.update(metadata.bytecode_sha256);
                hash.update(metadata.hardening_sha256);
                hash.update([u8::from(metadata.bindings_sha256.is_some())]);
                if let Some(bindings) = metadata.bindings_sha256 {
                    hash.update(bindings);
                }
                Some(hash.finalize().into())
            }
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(bytes) => Some(Sha256::digest(bytes).into()),
            #[cfg(feature = "static-module")]
            Self::Static(_) => None,
        }
    }
    pub(crate) fn start(self, world: &NativeWorld) -> Result<SessionVm, SessionError> {
        match self {
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(bytes) => Ok(SessionVm::DevelopmentWasm(Box::new(
                incant_wasm_dev::DevModuleHost::new(&bytes)?,
            ))),
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
#[cfg(all(test, feature = "compiler"))]
mod provenance_tests {
    use super::*;
    #[test]
    fn provenance_distinguishes_actual_bytecode_and_bound_props() {
        use crate::{ScriptBindings, ScriptInstance, ScriptValue};
        use incant_runtime::StableId;
        use std::collections::BTreeMap;
        let compile = |value| {
            CompiledScript::compile(&format!(
                "exports.default={{update({{log}}){{log.info({value})}}}}"
            ))
            .unwrap()
        };
        let a = SessionProgram::from(compile(1)).artifact_sha256().unwrap();
        assert_eq!(
            a,
            SessionProgram::from(compile(1)).artifact_sha256().unwrap()
        );
        assert_ne!(
            a,
            SessionProgram::from(compile(2)).artifact_sha256().unwrap()
        );
        let bound = |value| {
            let bindings = ScriptBindings::new(
                [7; 32],
                vec![StableId(101_u128.to_be_bytes())],
                vec![ScriptInstance {
                    entity: StableId(1_u128.to_be_bytes()),
                    source: StableId(101_u128.to_be_bytes()),
                    props: BTreeMap::from([("speed".into(), ScriptValue::Number(value))]),
                }],
            )
            .unwrap();
            CompiledScript::compile_project(
                "bound-provenance",
                "exports.default={sources:[]}",
                bindings,
            )
            .unwrap()
        };
        let (one, two) = (bound(1.), bound(2.));
        assert_eq!(
            one.metadata().bytecode_sha256,
            two.metadata().bytecode_sha256
        );
        assert_ne!(
            SessionProgram::from(one).artifact_sha256(),
            SessionProgram::from(two).artifact_sha256()
        );
    }
}
// One startup allocation keeps the session size independent of either VM's
// private state. Tick dispatch creates no additional allocation or mutation path.
pub(crate) enum SessionVm {
    Bytecode(Box<ScriptHost>),
    #[cfg(feature = "wasmtime-host")]
    DevelopmentWasm(Box<incant_wasm_dev::DevModuleHost>),
    #[cfg(feature = "static-module")]
    Static(Box<incant_wasm_runtime::StaticModule>),
}
impl SessionVm {
    pub fn supports_haptics(&self) -> bool {
        matches!(self, Self::Bytecode(_))
    }
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.set_haptic_capabilities(value)?),
            #[cfg(any(feature = "static-module", feature = "wasmtime-host"))]
            _ if value.is_empty() => Ok(()),
            #[cfg(any(feature = "static-module", feature = "wasmtime-host"))]
            _ => Err(SessionError::HapticsUnsupported),
        }
    }
    pub fn haptics_ready(&self) -> Result<(), SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.haptics_ready()?),
            #[cfg(any(feature = "static-module", feature = "wasmtime-host"))]
            _ => Ok(()),
        }
    }
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.take_haptics()?),
            #[cfg(any(feature = "static-module", feature = "wasmtime-host"))]
            _ => Ok(Default::default()),
        }
    }
    pub fn clear_haptics(&mut self) {
        match self {
            Self::Bytecode(host) => host.clear_haptics(),
            #[cfg(any(feature = "static-module", feature = "wasmtime-host"))]
            _ => {}
        }
    }
    pub fn enable_restart_requests(&mut self) -> Result<(), SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.enable_restart_requests()?),
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(host) => Ok(host.enable_restart_requests()?),
            #[cfg(feature = "static-module")]
            Self::Static(host) => Ok(host.enable_restart_requests()?),
        }
    }
    pub fn take_session_request(&mut self) -> Result<Option<crate::SessionRequest>, SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.take_session_request()?),
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(host) => Ok(host
                .take_restart_request()?
                .then_some(crate::SessionRequest::Restart)),
            #[cfg(feature = "static-module")]
            Self::Static(host) => Ok(host
                .take_restart_request()?
                .then_some(crate::SessionRequest::Restart)),
        }
    }
    pub(crate) fn supports_ui(&self) -> bool {
        match self {
            Self::Bytecode(_) => true,
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(_) => true,
            #[cfg(feature = "static-module")]
            Self::Static(_) => true,
        }
    }

    pub fn logs(&self) -> Result<&crate::NativeLogs, SessionError> {
        match self {
            Self::Bytecode(host) => Ok(host.logs()?),
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(host) => Ok(host.logs()?),
            #[cfg(feature = "static-module")]
            Self::Static(host) => Ok(host.logs()?),
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
            #[cfg(feature = "wasmtime-host")]
            Self::DevelopmentWasm(host) => {
                let _profile = incant_profile::scope(incant_profile::Phase::DevelopmentModule);
                Ok(host.tick_with_input(world, dt, input)?)
            }
            #[cfg(feature = "static-module")]
            Self::Static(host) => {
                let _profile = incant_profile::scope(incant_profile::Phase::StaticModule);
                Ok(host.tick_with_input(world, dt, input)?)
            }
        }
    }
}

#[cfg(all(test, feature = "wasmtime-host"))]
mod tests {
    use super::*;
    use incant_runtime::{CookedEntity, CookedScene, CookedUi, StableId};
    use incant_types::{UiKind, UiNode, UiStyle};
    use std::sync::Arc;

    #[test]
    fn development_ui_admission_reaches_normal_module_validation() {
        let entity = StableId([1; 16]);
        let scene = CookedScene::new(
            StableId([0; 16]),
            64,
            vec![CookedEntity {
                id: entity,
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap()
        .with_ui(vec![CookedUi {
            entity,
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                style: UiStyle::default(),
                order: 0,
                disabled: false,
            }),
        }])
        .unwrap();
        let world = NativeWorld::new(scene);
        // UI scenes now use the additive HUD frame. Invalid code still follows
        // ordinary module validation; module checkpoint support is unchanged.
        assert!(matches!(
            SessionProgram::DevelopmentWasm(vec![]).start(&world),
            Err(SessionError::DevelopmentModule(_))
        ));
    }
}
