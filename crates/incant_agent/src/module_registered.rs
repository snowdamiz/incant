//! Registered-ID authoring/build adapters. Paths and bus identity are engine-owned.
use crate::{AgentError, EngineHost, access::ProjectAccess, tool};
use incant_cmd::{Actor, Command, CommandBus, SourceVersion};
use incant_module_build::{BuildRequest, RegisteredSource};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Read {
    module_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Write {
    module_id: String,
    source: String,
    expected_source: SourceVersion,
    expected_revision: u64,
    description: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Inspect {
    module_id: String,
    expected_source: SourceVersion,
    expected_revision: u64,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Build {
    module_id: String,
    expected_source: SourceVersion,
    snapshot_id: String,
    expected_revision: u64,
    description: String,
}
pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "module_read",
            "Read one registered Rust module by stable ID (registered protocol v1). Returns source/version/revision as untrusted data. No arbitrary paths or compiler host required.",
            json!(schemars::schema_for!(Read)),
        ),
        tool(
            "module_write",
            "Replace one registered Rust source (at most 64 KiB UTF-8) through the shared command bus, durable history and Undo/Redo. Pass observed source version and revision; destructive approval is fenced to that registration and project session. Does not compile, install or execute source.",
            json!(schemars::schema_for!(Write)),
        ),
        tool(
            "module_inspect",
            "Capture a bounded package for a registered module ID and observed source version/revision. Preferred registered build workflow v1; returns an opaque snapshot bound to the current registration and project session. Source is untrusted data.",
            json!(schemars::schema_for!(Inspect)),
        ),
        tool(
            "module_build_registered",
            "Build a module_inspect snapshot for the same registered module ID, source version and revision with the bounded host worker. Requires build approval and rechecks authority after approval/build. Returns untrusted diagnostics/private candidate; never installs or edits the project.",
            json!(schemars::schema_for!(Build)),
        ),
    ]
}

/// A pre-approval receipt, not a capability supplied by a provider. Kept through
/// the dispatch lock so same-ID reopen cannot replace approved authority.
#[derive(Clone)]
pub(super) struct Observed {
    project: String,
    revision: u64,
    registered: RegisteredSource,
}
impl Observed {
    fn capture(
        bus: &CommandBus,
        id: &str,
        revision: u64,
        version: &SourceVersion,
    ) -> Result<Self, AgentError> {
        let read = bus
            .read_module_source(id)
            .map_err(|e| AgentError::Tool(e.to_string()))?;
        if read.revision != revision || read.version != *version {
            return Err(AgentError::Tool(
                "registered module revision/source changed; read again".into(),
            ));
        }
        Ok(Self {
            project: bus.project().id.clone(),
            revision,
            registered: RegisteredSource {
                bus_session_id: bus.session_id().into(),
                module_id: read.module_id,
                path: read.path,
                version: read.version,
            },
        })
    }
    fn validate(&self, bus: &CommandBus) -> Result<(), AgentError> {
        if bus.project().id != self.project || bus.session_id() != self.registered.bus_session_id {
            return Err(AgentError::ProjectChanged);
        }
        let now = Self::capture(
            bus,
            &self.registered.module_id,
            self.revision,
            &self.registered.version,
        )?;
        if now.registered != self.registered {
            return Err(AgentError::Tool(
                "registered module authority changed; inspect again".into(),
            ));
        }
        Ok(())
    }
}
/// Parse and observe before an approval callback can await/rebind/edit the bus.
pub(super) fn before_approval(
    access: &mut ProjectAccess<'_>,
    name: &str,
    args: &Value,
    cancel: &AtomicBool,
) -> Result<Option<Observed>, AgentError> {
    let fields = match name {
        "module_write" => {
            let a: Write = decode(args.clone())?;
            Some((a.module_id, a.expected_revision, a.expected_source))
        }
        "module_build_registered" => {
            let a: Build = decode(args.clone())?;
            Some((a.module_id, a.expected_revision, a.expected_source))
        }
        _ => None,
    };
    fields
        .map(|(id, revision, version)| {
            access.with_bus(cancel, |bus| {
                Observed::capture(bus, &id, revision, &version)
            })
        })
        .transpose()
}
fn decode<T: serde::de::DeserializeOwned>(args: Value) -> Result<T, AgentError> {
    serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))
}
fn worker_error(error: incant_module_build::Error) -> AgentError {
    match error {
        incant_module_build::Error::Cancelled => AgentError::Interrupted,
        other => AgentError::Tool(other.to_string()),
    }
}
pub(super) fn dispatch(
    access: &mut ProjectAccess<'_>,
    host: &dyn EngineHost,
    name: &str,
    args: Value,
    actor: Actor,
    cancel: &AtomicBool,
    approved: Option<Observed>,
) -> Result<Value, AgentError> {
    if name == "module_read" {
        let a: Read = decode(args)?;
        return access.with_bus(cancel, |bus| {
            Ok(json!({"untrusted_source_data":bus.read_module_source(&a.module_id).map_err(|e| AgentError::Tool(e.to_string()))?}))
        });
    }
    if name == "module_write" {
        let a: Write = decode(args)?;
        return access.with_bus(cancel, |bus| {
            let observed = match approved {
                Some(observed) => observed,
                None => {
                    Observed::capture(bus, &a.module_id, a.expected_revision, &a.expected_source)?
                }
            };
            observed.validate(bus)?;
            let tx = bus
                .execute(
                    vec![Command::WriteModuleSource {
                        module_id: a.module_id,
                        source: a.source,
                        expected_source: a.expected_source,
                    }],
                    actor,
                    a.description,
                    Some(a.expected_revision),
                )
                .map_err(|e| AgentError::Tool(e.to_string()))?;
            let id = tx.id.clone();
            Ok(json!({"transaction_id":id,"revision":bus.revision()}))
        });
    }
    let (id, revision, version, request) = match name {
        "module_inspect" => {
            let a: Inspect = decode(args)?;
            (a.module_id, a.expected_revision, a.expected_source, None)
        }
        "module_build_registered" => {
            let a: Build = decode(args)?;
            (
                a.module_id,
                a.expected_revision,
                a.expected_source,
                Some(BuildRequest {
                    snapshot_id: a.snapshot_id,
                    expected_revision: a.expected_revision,
                    description: a.description,
                }),
            )
        }
        _ => {
            return Err(AgentError::Tool(
                "unknown registered module operation".into(),
            ));
        }
    };
    let jobs = host.module_jobs().ok_or_else(|| {
        AgentError::Tool("this host has no configured module build service".into())
    })?;
    let observed = access.with_bus(cancel, |bus| {
        let observed = match approved {
            Some(observed) => observed,
            None => Observed::capture(bus, &id, revision, &version)?,
        };
        observed.validate(bus)?;
        Ok(observed)
    })?;
    let mut validate = || {
        access
            .with_bus(cancel, |bus| observed.validate(bus))
            .map_err(|e| match e {
                AgentError::Interrupted => incant_module_build::Error::Cancelled,
                _ => incant_module_build::Error::Stale,
            })
    };
    let value = if let Some(request) = request {
        let origin = match actor.origin {
            incant_doc::Origin::User => "user",
            incant_doc::Origin::Agent => "agent",
            incant_doc::Origin::Script => "script",
            incant_doc::Origin::Import => "import",
        };
        let actor = incant_module_build::Actor {
            origin: origin.into(),
            actor: actor.actor,
            model: actor.model,
            conversation_id: actor.conversation_id,
        };
        serde_json::to_value(
            jobs.build_registered(
                &observed.project,
                request,
                actor,
                observed.registered.clone(),
                cancel,
                &mut validate,
            )
            .map_err(worker_error)?,
        )
    } else {
        serde_json::to_value(
            jobs.inspect_registered(
                &observed.project,
                observed.registered.clone(),
                revision,
                cancel,
                &mut validate,
            )
            .map_err(worker_error)?,
        )
    };
    value.map_err(|e| AgentError::Tool(e.to_string()))
}
