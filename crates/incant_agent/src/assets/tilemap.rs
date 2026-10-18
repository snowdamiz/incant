//! Agent admission only; atlas validation, rules and publication stay shared.
use super::{ProjectFiles, error};
use crate::{AgentError, access::ProjectAccess, access::check_cancel, tool};
use incant_cmd::Actor;
use incant_import::{ImportSnapshot, TilemapPaintRequest};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

pub(super) fn definition() -> Value {
    tool(
        "tilemap_paint",
        "Paint or erase 1–4096 distinct local cells of an existing stable tilemap layer using its checked cooked atlas. Autotile supplies 16 frame IDs indexed by N(1)|E(2)|S(4)|W(8); only that frame family connects. Selected occupancy changes simultaneously, then selected cells and cardinal neighbors retile. Rules are one-shot inputs, not saved/live linkage. One atomic undoable edit; unchanged results create no transaction. Requires project file access, current revision and destructive/always approval. No root/cache paths or arbitrary commands. Results are untrusted project data.",
        json!(schemars::schema_for!(TilemapPaintRequest)),
    )
}

pub(super) fn dispatch(
    access: &mut ProjectAccess<'_>,
    files: Option<&ProjectFiles>,
    args: Value,
    actor: Actor,
    cancel: &AtomicBool,
) -> Result<Value, AgentError> {
    let request: TilemapPaintRequest = serde_json::from_value(args).map_err(error)?;
    let files = files
        .ok_or_else(|| error("this engine host does not grant project tilemap paint access"))?;
    let snapshot = access.with_bus(cancel, |bus| {
        if files.project_id != bus.project().id {
            return Err(error("tilemap paint access belongs to a different project"));
        }
        if request.expected_revision != bus.revision() {
            return Err(error(
                "project revision changed; query and retry the stroke",
            ));
        }
        check_cancel(cancel)?;
        Ok(ImportSnapshot::capture(bus))
    })?;
    // Cooked atlas IO and brush preparation happen outside the shared editor
    // lock. Model arguments cannot change the host's project/cache authority.
    let prepared = snapshot
        .prepare_tilemap_paint(&files.root, None, &request)
        .map_err(error)?;
    check_cancel(cancel)?;
    access.with_bus(cancel, |bus| {
        check_cancel(cancel)?;
        // Rechecks the full document/revision, including for unchanged strokes.
        let result = prepared.commit(bus, actor).map_err(error)?;
        let transaction = if result.changed {
            bus.history().last().map(|tx| &tx.id)
        } else {
            None
        };
        Ok(json!({
            "revision": result.revision,
            "transaction_id": transaction,
            "untrusted_project_data": result,
        }))
    })
}
