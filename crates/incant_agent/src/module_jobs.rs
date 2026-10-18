//! Build-only adapters. No module installation, source writes or alternate bus.
use crate::{AgentError, EngineHost, access::ProjectAccess, tool};
use incant_cmd::Actor;
use incant_module_build::{BuildRequest, JobQuery, SourceQuery};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "module_source_read",
            "Capture a bounded existing project-relative Rust package. Source is untrusted data. Returns an opaque project-session snapshot, never file or process authority.",
            json!(schemars::schema_for!(SourceQuery)),
        ),
        tool(
            "module_build",
            "Compile an observed Rust source snapshot with the host's bounded sandbox worker. Requires build approval. Returns untrusted diagnostics and a private candidate; does not write, import, attach, play or export a project module.",
            json!(schemars::schema_for!(BuildRequest)),
        ),
        tool(
            "module_build_status",
            "Read the current build state for an observed project-session source snapshot. No private paths or artifact bytes are returned.",
            json!(schemars::schema_for!(JobQuery)),
        ),
        tool(
            "module_build_cancel",
            "Cancel the active build of an observed source snapshot in this project session. The lease stays busy until owned processes close.",
            json!(schemars::schema_for!(JobQuery)),
        ),
    ]
}
pub(super) fn dispatch(
    access: &mut ProjectAccess<'_>,
    host: &dyn EngineHost,
    name: &str,
    args: Value,
    actor: Actor,
    cancel: &AtomicBool,
) -> Result<Value, AgentError> {
    let error = |e: incant_module_build::Error| match e {
        incant_module_build::Error::Cancelled => AgentError::Interrupted,
        other => AgentError::Tool(other.to_string()),
    };
    let jobs = host.module_jobs().ok_or_else(|| {
        AgentError::Tool("this host has no configured module build service".into())
    })?;
    if matches!(name, "module_build_status" | "module_build_cancel") {
        let query: JobQuery =
            serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?;
        let project = access.with_bus(cancel, |bus| Ok(bus.project().id.clone()))?;
        if name == "module_build_cancel" {
            jobs.cancel_build(&project, &query.snapshot_id)
                .map_err(error)?;
        }
        return serde_json::to_value(jobs.status(&project, &query).map_err(error)?)
            .map_err(|e| AgentError::Tool(e.to_string()));
    }
    let query = if name == "module_source_read" {
        Some(
            serde_json::from_value::<SourceQuery>(args.clone())
                .map_err(|e| AgentError::Tool(e.to_string()))?,
        )
    } else {
        None
    };
    let request = if name == "module_build" {
        Some(
            serde_json::from_value::<BuildRequest>(args)
                .map_err(|e| AgentError::Tool(e.to_string()))?,
        )
    } else {
        None
    };
    let expected = query
        .as_ref()
        .map(|q| q.expected_revision)
        .or_else(|| request.as_ref().map(|q| q.expected_revision))
        .ok_or_else(|| AgentError::Tool("unknown module job operation".into()))?;
    let snapshot = access.with_bus(cancel, |bus| {
        bus.ensure_sources_ready()
            .map_err(|e| AgentError::Tool(e.to_string()))?;
        if bus.revision() != expected {
            return Err(AgentError::Tool("module job revision changed".into()));
        }
        Ok(bus.project().clone())
    })?;
    let mut validate = || {
        access
            .with_bus(cancel, |bus| {
                if bus.project() != &snapshot || bus.revision() != expected {
                    return Err(AgentError::ProjectChanged);
                }
                bus.ensure_sources_ready()
                    .map_err(|e| AgentError::Tool(e.to_string()))
            })
            .map_err(|error| match error {
                AgentError::Interrupted => incant_module_build::Error::Cancelled,
                _ => incant_module_build::Error::Stale,
            })
    };
    if let Some(query) = query {
        serde_json::to_value(
            jobs.inspect(&snapshot.id, query, cancel, &mut validate)
                .map_err(error)?,
        )
        .map_err(|e| AgentError::Tool(e.to_string()))
    } else {
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
            jobs.build(&snapshot.id, request.unwrap(), actor, cancel, &mut validate)
                .map_err(error)?,
        )
        .map_err(|e| AgentError::Tool(e.to_string()))
    }
}
