//! Registered installation: ordinary import preparation inside a service lease.
use super::*;
use incant_cmd::BinaryVersion;
use incant_import::{CompiledWasmRequest, ImportDetails, PreparedCompiledWasm};
use incant_module_build::InstallRequest;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Install {
    module_id: String,
    snapshot_id: String,
    candidate_id: String,
    expected_revision: u64,
    expected_source: SourceVersion,
    expected_destination: BinaryVersion,
    description: String,
}
impl Install {
    fn validate(&self) -> Result<(), AgentError> {
        if self.description.trim().is_empty() || self.description.len() > 1024 {
            return Err(AgentError::Tool(
                "bounded installation description required".into(),
            ));
        }
        Ok(())
    }
}
pub(super) fn tool() -> Value {
    crate::tool(
        "module_install_registered",
        "Install a completed registered build candidate by module/snapshot/candidate IDs and observed source/destination versions plus revision. Requires destructive/always approval. Revalidates the whole package and service generation, uses ordinary Wasm import, then one durable Undo/Redo transaction. Never accepts paths, compilers, source bytes or scene activation.",
        json!(schemars::schema_for!(Install)),
    )
}
pub(super) fn before_approval(
    access: &mut ProjectAccess<'_>,
    args: &Value,
    cancel: &AtomicBool,
) -> Result<Observed, AgentError> {
    let a: Install = decode(args.clone())?;
    a.validate()?;
    access.with_bus(cancel, |bus| {
        let observed =
            Observed::capture(bus, &a.module_id, a.expected_revision, &a.expected_source)?;
        if bus
            .module_binary_version(&a.module_id)
            .map_err(|e| AgentError::Tool(e.to_string()))?
            != a.expected_destination
        {
            return Err(AgentError::Tool(
                "compiled destination changed; read again".into(),
            ));
        }
        Ok(observed)
    })
}
fn service_error(error: AgentError) -> incant_module_build::Error {
    match error {
        AgentError::Interrupted => incant_module_build::Error::Cancelled,
        other => incant_module_build::Error::Worker(other.to_string()),
    }
}
pub(super) fn dispatch(
    access: &mut ProjectAccess<'_>,
    host: &dyn EngineHost,
    args: Value,
    actor: Actor,
    cancel: &AtomicBool,
    approved: Option<Observed>,
) -> Result<Value, AgentError> {
    let a: Install = decode(args)?;
    a.validate()?;
    let jobs = host
        .module_jobs()
        .ok_or_else(|| AgentError::Tool("no module build service".into()))?;
    let observed = access.with_bus(cancel, |bus| {
        let observed = match approved {
            Some(observed) => observed,
            None => Observed::capture(bus, &a.module_id, a.expected_revision, &a.expected_source)?,
        };
        observed.validate(bus)?;
        Ok(observed)
    })?;
    let installation = jobs
        .prepare_install(
            &observed.project,
            InstallRequest {
                snapshot_id: a.snapshot_id,
                candidate_id: a.candidate_id,
                expected_revision: a.expected_revision,
            },
            observed.registered.clone(),
            cancel,
            |root, bytes, receipt| {
                access
                    .with_bus(cancel, |bus| {
                        observed.validate(bus)?;
                        let prepared = PreparedCompiledWasm::prepare(
                            bus,
                            root,
                            CompiledWasmRequest {
                                module_id: a.module_id,
                                expected_revision: a.expected_revision,
                                expected_source: a.expected_source,
                                expected_destination: a.expected_destination,
                            },
                            bytes,
                        )
                        .map_err(|e| AgentError::Tool(e.to_string()))?;
                        let ImportDetails::Wasm {
                            bytes,
                            input_sha256,
                            prepared_sha256,
                            ..
                        } = &prepared.outcome().details
                        else {
                            return Err(AgentError::Tool(
                                "ordinary Wasm admission required".into(),
                            ));
                        };
                        if *bytes != receipt.raw_bytes
                            || *input_sha256 != receipt.raw_sha256
                            || *prepared_sha256 != receipt.prepared_sha256
                        {
                            return Err(AgentError::Tool(
                                "ordinary validator differs from completed worker".into(),
                            ));
                        }
                        Ok(prepared)
                    })
                    .map_err(service_error)
            },
        )
        .map_err(worker_error)?;
    access.with_bus(cancel, |bus| {
        // Bus first, service second. Neither callback acquires a bus guard or
        // invokes a backend while the service is locked.
        observed.validate(bus)?;
        installation.publish(cancel, |prepared, receipt| {
            let committed = prepared.commit(bus, actor, a.description).map_err(|e| AgentError::Tool(e.to_string()))?;
            let id = bus.history().last().ok_or_else(|| AgentError::Tool("missing committed installation".into()))?.id.clone();
            Ok(json!({"transaction_id":id,"revision":committed.revision,"imports":committed.imports,"installation":receipt}))
        }).map_err(worker_error)?
    })
}
