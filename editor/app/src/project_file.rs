//! Application file operations use the same host-opened bus as commands/agents.
use crate::{Editor, engine_read};
use incant_cmd::{Actor, ProjectFileRequest};
use serde_json::{Value, json};
use std::sync::Arc;

#[tauri::command]
pub async fn engine_project_file(
    state: tauri::State<'_, Arc<Editor>>,
    request: ProjectFileRequest,
) -> Result<Value, String> {
    let bus = state.bus.clone();
    let response = tauri::async_runtime::spawn_blocking(move || {
        let mut load = bus.lock().map_err(|_| "engine lock failed".to_string())?;
        load.bus_mut()?
            .project_file_request(request, Actor::user("editor"))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(json!({"response":response, "snapshot":engine_read(state)?}))
}

pub fn schemas() -> std::collections::BTreeMap<String, Value> {
    let mut schemas = incant_doc::schema_registry();
    schemas.extend(incant_cmd::project_file_schemas());
    schemas
}
