//! Tagged explicit state. Saved data can never select or construct native code.
use super::*;
#[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
use incant_wasm_runtime::persistent::{Declaration, PersistentSnapshot};

pub(super) enum ProgramState {
    Bytecode(ScriptState),
    #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
    Wasm(PersistentSnapshot),
}
impl ProgramState {
    #[cfg(feature = "native-checkpoints")]
    pub(super) fn copy(&self) -> Result<Self> {
        Ok(match self {
            Self::Bytecode(state) => Self::Bytecode(state.copy()?),
            #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
            Self::Wasm(state) => Self::Wasm(state.clone()),
        })
    }
}

pub(super) fn program_identity(program: &SessionProgram) -> Result<[u8; 32]> {
    match program {
        SessionProgram::Bytecode(code) => Ok(identity(code)),
        #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
        _ => {
            let declaration = wasm_declaration(program)?;
            let mut hash = Sha256::new();
            hash.update(b"Incant explicit Wasm checkpoint v1\0");
            hash.update(declaration.encode());
            for value in [
                env!("INCANT_WASM_CHECKPOINT_RUNTIME_ID"),
                env!("INCANT_CHECKPOINT_RUNTIME_ID"),
                env!("INCANT_SCRIPT_TARGET"),
            ] {
                hash.update((value.len() as u64).to_le_bytes());
                hash.update(value.as_bytes());
            }
            match program {
                #[cfg(feature = "wasmtime-host")]
                SessionProgram::DevelopmentWasm(bytes) => {
                    hash.update(b"wasmtime\0");
                    hash.update(bytes);
                }
                #[cfg(feature = "static-module")]
                SessionProgram::Static(module) => {
                    hash.update(b"wasm2c\0");
                    hash.update(
                        module
                            .persistent_program()
                            .expect("validated declaration")
                            .fingerprint,
                    );
                }
                SessionProgram::Bytecode(_) => unreachable!(),
            }
            Ok(hash.finalize().into())
        }
    }
}
#[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
pub(super) fn wasm_declaration(program: &SessionProgram) -> Result<Declaration> {
    match program {
        #[cfg(feature = "wasmtime-host")]
        SessionProgram::DevelopmentWasm(bytes) => {
            incant_wasm_runtime::persistent::declaration_in_module(bytes)
                .map_err(CheckpointError::Invalid)?
                .ok_or(CheckpointError::Unsupported("development module state ABI"))
        }
        #[cfg(feature = "static-module")]
        SessionProgram::Static(module) => module
            .persistent_program()
            .map(|p| p.declaration)
            .ok_or(CheckpointError::Unsupported("static module state ABI")),
        SessionProgram::Bytecode(_) => Err(CheckpointError::ProgramMismatch),
    }
}
/// Check the receiving owner without consuming outputs or granting authority.
pub(super) fn receiver(session: &NativeSession) -> Result<(SessionProgram, i64, bool)> {
    if session.failed {
        return Err(CheckpointError::FailedSession);
    }
    match &session.host {
        SessionVm::Bytecode(_) => {
            let host = host(session)?;
            if !host.haptic_requests.is_empty() {
                return Err(CheckpointError::Unsupported("pending haptic output"));
            }
            if host.session_request.is_some() {
                return Err(CheckpointError::Unsupported(
                    "pending session restart request",
                ));
            }
            Ok((host._compiled.clone().into(), 0, host.restart_requests))
        }
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(host) => {
            let source = host
                .persistent_source()
                .ok_or(CheckpointError::Unsupported("development module state ABI"))?;
            let (watermark, restart) = host
                .checkpoint_authority(&session.world)
                .map_err(crate::SessionError::from)?;
            Ok((
                SessionProgram::DevelopmentWasm(source.to_vec()),
                watermark,
                restart,
            ))
        }
        #[cfg(feature = "static-module")]
        SessionVm::Static(host) => {
            let module = host.exports();
            if module.persistent_program().is_none() {
                return Err(CheckpointError::Unsupported("static module state ABI"));
            }
            let (watermark, restart) = host
                .checkpoint_authority(&session.world)
                .map_err(crate::SessionError::from)?;
            Ok((module.into(), watermark, restart))
        }
    }
}
pub(super) fn capture(session: &NativeSession) -> Result<(SessionProgram, ProgramState)> {
    let (program, _, _) = receiver(session)?;
    let state = match &session.host {
        SessionVm::Bytecode(_) => ProgramState::Bytecode(script_state(session)?),
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(host) => ProgramState::Wasm(
            host.persistent_checkpoint(&session.world)
                .map_err(crate::SessionError::from)?,
        ),
        #[cfg(feature = "static-module")]
        SessionVm::Static(host) => ProgramState::Wasm(
            host.persistent_checkpoint(&session.world)
                .map_err(crate::SessionError::from)?,
        ),
    };
    Ok((program, state))
}
pub(super) fn sections<'a>(
    bytes: &'a [u8],
    program: &SessionProgram,
    persisted: bool,
) -> Result<codec::Sections<'a>> {
    let identity = program_identity(program)?;
    match program {
        SessionProgram::Bytecode(_) => {
            #[cfg(feature = "native-checkpoints")]
            if persisted {
                return codec::persisted_sections(bytes, identity);
            }
            let _ = persisted;
            codec::sections(bytes, identity)
        }
        #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
        _ => codec::wasm_sections(bytes, identity, persisted),
    }
}
#[allow(clippy::too_many_arguments)] // Same explicit section inputs as the existing codec.
pub(super) fn encode(
    program: &SessionProgram,
    state: &ProgramState,
    origin: [u8; 32],
    world: &[u8],
    ui: Option<&[u8]>,
    audio: Option<&[u8]>,
    persisted: bool,
) -> Result<Vec<u8>> {
    let identity = program_identity(program)?;
    match (program, state) {
        (SessionProgram::Bytecode(_), ProgramState::Bytecode(state)) => {
            #[cfg(feature = "native-checkpoints")]
            if persisted {
                return codec::encode_persisted(
                    identity,
                    origin,
                    world,
                    &state.active,
                    &state.store,
                    &state.schedule,
                    ui,
                    audio,
                );
            }
            let _ = persisted;
            codec::encode(
                identity,
                origin,
                world,
                &state.active,
                &state.store,
                &state.schedule,
                ui,
                audio,
            )
        }
        #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
        (_, ProgramState::Wasm(state)) if state.declaration() == wasm_declaration(program)? => {
            codec::wasm_encode(
                identity,
                origin,
                world,
                &state.encode(),
                ui,
                audio,
                persisted,
            )
        }
        #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
        _ => Err(CheckpointError::ProgramMismatch),
    }
}
