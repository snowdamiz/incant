//! Nonvisual typed IPC over the same project-bound service used by agent tools.
use crate::{Editor, project::LoadState};
use incant_module_build::{
    Actor, BuildRequest, BuildResult, JobQuery, JobStatus, SourceQuery, SourceView,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn snapshot(editor: &Editor, expected: u64) -> Result<incant_doc::Project, String> {
    let mut state = editor.bus.lock().map_err(|_| "project unavailable")?;
    let bus = state.bus_mut()?;
    if !editor.alive.load(Ordering::Acquire) || bus.revision() != expected {
        return Err("module job revision changed".into());
    }
    bus.ensure_sources_ready().map_err(|e| e.to_string())?;
    Ok(bus.project().clone())
}
fn validate(
    editor: &Editor,
    project: &incant_doc::Project,
    expected: u64,
) -> incant_module_build::Result<()> {
    if snapshot(editor, expected).is_ok_and(|current| current == *project) {
        Ok(())
    } else {
        Err(incant_module_build::Error::Stale)
    }
}
#[tauri::command]
pub async fn engine_module_source_read(
    state: tauri::State<'_, Arc<Editor>>,
    query: SourceQuery,
) -> Result<SourceView, String> {
    let editor = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = snapshot(&editor, query.expected_revision)?;
        let revision = query.expected_revision;
        editor
            .module_jobs
            .inspect(&project.id, query, &AtomicBool::new(false), &mut || {
                validate(&editor, &project, revision)
            })
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn engine_module_build(
    state: tauri::State<'_, Arc<Editor>>,
    request: BuildRequest,
) -> Result<BuildResult, String> {
    let editor = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = snapshot(&editor, request.expected_revision)?;
        let revision = request.expected_revision;
        let actor = Actor {
            origin: "user".into(),
            actor: "editor".into(),
            model: None,
            conversation_id: None,
        };
        editor
            .module_jobs
            .build(
                &project.id,
                request,
                actor,
                &AtomicBool::new(false),
                &mut || validate(&editor, &project, revision),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn current_project(editor: &Editor) -> Result<String, String> {
    match &*editor.bus.lock().map_err(|_| "project unavailable")? {
        LoadState::Ready(bus) if editor.alive.load(Ordering::Acquire) => {
            Ok(bus.project().id.clone())
        }
        _ => Err("project unavailable".into()),
    }
}
#[tauri::command]
pub fn engine_module_build_status(
    state: tauri::State<'_, Arc<Editor>>,
    query: JobQuery,
) -> Result<JobStatus, String> {
    state
        .module_jobs
        .status(&current_project(&state)?, &query)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn engine_module_build_cancel(
    state: tauri::State<'_, Arc<Editor>>,
    query: JobQuery,
) -> Result<JobStatus, String> {
    let project = current_project(&state)?;
    state
        .module_jobs
        .cancel_build(&project, &query.snapshot_id)
        .map_err(|e| e.to_string())?;
    state
        .module_jobs
        .status(&project, &query)
        .map_err(|e| e.to_string())
}
