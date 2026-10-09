//! Native import preparation runs outside the event thread and bus lock. Only
//! revision-checked commit takes the bus again; failures preserve project edits.
use crate::{Editor, engine_read, project::LoadState};
use incant_cmd::{Actor, CommandError};
use incant_import::{ImportRequest, ImportSnapshot, PreparedImports};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Emitter;
use thiserror::Error;

#[derive(Debug, Error)]
enum ImportTaskError {
    #[error("Open a saved project to import assets.")]
    Unsaved,
    #[error("Another asset import is running. Wait for it to finish.")]
    Busy,
    #[error("The project is unavailable: {0}")]
    Project(String),
    #[error("The editor closed before import completed.")]
    Closed,
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error(transparent)]
    Import(#[from] incant_import::ImportError),
}

struct ImportGuard<'a>(&'a AtomicBool);
impl<'a> ImportGuard<'a> {
    fn acquire(flag: &'a AtomicBool) -> Result<Self, ImportTaskError> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ImportTaskError::Busy)?;
        Ok(Self(flag))
    }
}
impl Drop for ImportGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
fn capture(state: &Mutex<LoadState>, expected: u64) -> Result<ImportSnapshot, ImportTaskError> {
    let mut state = state
        .lock()
        .map_err(|_| ImportTaskError::Project("engine lock failed".into()))?;
    let bus = state.bus_mut().map_err(ImportTaskError::Project)?;
    if bus.revision() != expected {
        return Err(CommandError::Conflict {
            expected,
            actual: bus.revision(),
        }
        .into());
    }
    Ok(ImportSnapshot::capture(bus))
}
fn commit(state: &Mutex<LoadState>, prepared: PreparedImports) -> Result<(), ImportTaskError> {
    let mut state = state
        .lock()
        .map_err(|_| ImportTaskError::Project("engine lock failed".into()))?;
    prepared.commit(
        state.bus_mut().map_err(ImportTaskError::Project)?,
        Actor::user("editor"),
        "Import assets",
    )?;
    Ok(())
}

#[tauri::command]
pub async fn engine_import(
    state: tauri::State<'_, Arc<Editor>>,
    app: tauri::AppHandle,
    requests: Vec<ImportRequest>,
    expected_revision: u64,
) -> Result<Value, String> {
    let editor = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), ImportTaskError> {
        let _guard = ImportGuard::acquire(&editor.importing)?;
        let root = editor.asset_root.as_ref().ok_or(ImportTaskError::Unsaved)?;
        let snapshot = capture(&editor.bus, expected_revision)?;
        let prepared = snapshot.prepare(root, None, &requests)?;
        if !editor.alive.load(Ordering::Relaxed) {
            return Err(ImportTaskError::Closed);
        }
        commit(&editor.bus, prepared)
    })
    .await
    .map_err(|_| "Asset import worker stopped unexpectedly.".to_string())?
    .map_err(|e| e.to_string())?;
    let _ = app.emit_to(
        tauri::EventTarget::webview("editor"),
        "incant:engine-changed",
        (),
    );
    engine_read(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_cmd::{Command, CommandBus};
    use incant_doc::{Project, Scene};

    fn fixture(root: &std::path::Path) -> ImportRequest {
        // Real glTF with a local triangle buffer; no display or provider required.
        let bytes: Vec<_> = [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        std::fs::write(root.join("mesh.bin"), bytes).unwrap();
        std::fs::write(root.join("mesh.gltf"), serde_json::json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
        ImportRequest {
            source: "mesh.gltf".into(),
            texture_usage: None,
        }
    }

    #[test]
    fn preparation_releases_the_bus_and_concurrent_edits_reject_its_late_commit() {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        let state = Arc::new(Mutex::new(LoadState::Ready(Box::new(
            CommandBus::new(Project::empty("Import")).unwrap(),
        ))));
        let snapshot = capture(&state, 0).unwrap();
        let (release, wait) = std::sync::mpsc::channel();
        let root = temp.path().to_path_buf();
        let worker = std::thread::spawn(move || {
            wait.recv().unwrap();
            snapshot.prepare(&root, None, &[request]).unwrap()
        });
        {
            let mut state = state
                .try_lock()
                .expect("preparation must not hold the UI bus");
            let bus = state.bus_mut().unwrap();
            bus.execute(
                vec![Command::CreateScene {
                    scene: Scene::new("User edit"),
                }],
                Actor::user("test"),
                "Add scene",
                None,
            )
            .unwrap();
        }
        release.send(()).unwrap();
        assert!(matches!(
            commit(&state, worker.join().unwrap()),
            Err(ImportTaskError::Import(
                incant_import::ImportError::Command(CommandError::Conflict { .. })
            ))
        ));
        let mut state = state.lock().unwrap();
        let bus = state.bus_mut().unwrap();
        assert!(bus.project().assets.is_empty());
        assert_eq!(bus.history().len(), 1);
    }

    #[test]
    fn successful_import_uses_durable_shared_history_and_guard_releases_after_errors() {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        let project = Project::empty("Persistent");
        let journal = temp.path().join("journal");
        let state = Mutex::new(LoadState::Ready(Box::new(
            CommandBus::persistent(&journal, project.clone()).unwrap(),
        )));
        let flag = AtomicBool::new(false);
        let guard = ImportGuard::acquire(&flag).unwrap();
        assert!(matches!(
            ImportGuard::acquire(&flag),
            Err(ImportTaskError::Busy)
        ));
        assert!(capture(&state, 7).is_err());
        drop(guard);
        let _next = ImportGuard::acquire(&flag).unwrap();
        let prepared = capture(&state, 0)
            .unwrap()
            .prepare(temp.path(), None, &[request])
            .unwrap();
        commit(&state, prepared).unwrap();
        drop(state);
        let mut recovered = CommandBus::persistent(&journal, project).unwrap();
        assert_eq!(recovered.project().assets.len(), 1);
        assert_eq!(
            recovered.history()[0].actor.origin,
            incant_doc::Origin::User
        );
        recovered.undo().unwrap();
        assert!(recovered.project().assets.is_empty());
        assert!(capture(&Mutex::new(LoadState::Loading), 0).is_err());
    }
}
