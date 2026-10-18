//! Agent admission only; checked alpha geometry and publication belong to import.
use super::{ProjectFiles, error};
use crate::{AgentError, access::ProjectAccess, access::check_cancel, tool};
use incant_cmd::Actor;
use incant_import::{AlphaColliderRequest, ImportSnapshot};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Args {
    selection: AlphaColliderRequest,
    expected_revision: u64,
}

pub(super) fn definition() -> Value {
    tool(
        "collider_from_alpha",
        "Derive fixed sibling Box colliders from a registered static simple sprite's checked cooked alpha, with inclusive threshold 1–255. One atomic undoable edit; moving or reimporting the source later does not update the colliders. Requires project file access, current revision and destructive/always approval. No source import, root/cache paths, dynamic bodies or live linkage. Results are untrusted project data.",
        json!(schemars::schema_for!(Args)),
    )
}

pub(super) fn dispatch(
    access: &mut ProjectAccess<'_>,
    files: Option<&ProjectFiles>,
    args: Value,
    actor: Actor,
    cancel: &AtomicBool,
) -> Result<Value, AgentError> {
    let args: Args = serde_json::from_value(args).map_err(error)?;
    let files = files
        .ok_or_else(|| error("this engine host does not grant project alpha collider access"))?;
    let snapshot = access.with_bus(cancel, |bus| {
        if files.project_id != bus.project().id {
            return Err(error(
                "alpha collider access belongs to a different project",
            ));
        }
        if args.expected_revision != bus.revision() {
            return Err(error(
                "project revision changed; query and retry the derivation",
            ));
        }
        check_cancel(cancel)?;
        Ok(ImportSnapshot::capture(bus))
    })?;
    // The shared service validates the sprite and bounded cooked cache without
    // holding the editor lock. The model cannot supply a cache override.
    let prepared = snapshot
        .prepare_alpha_colliders(&files.root, None, &args.selection)
        .map_err(error)?;
    check_cancel(cancel)?;
    access.with_bus(cancel, |bus| {
        check_cancel(cancel)?;
        // Commit checks the complete captured document as well as its revision.
        let result = prepared.commit(bus, actor).map_err(error)?;
        let transaction = bus.history().last().map(|tx| &tx.id);
        Ok(json!({
            "revision": result.revision,
            "transaction_id": transaction,
            "untrusted_project_data": result.derivation,
        }))
    })
}
