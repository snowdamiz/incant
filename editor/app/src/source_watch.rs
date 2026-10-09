//! Source reads and cooking never borrow the live bus. Only revision-checked
//! publication takes its lock, using the same journal/history as manual edits.
use crate::{ConsoleEvent, Editor, project::LoadState};
use incant_cmd::CommandError;
use incant_import::{ImportError, ImportSnapshot, SourceWatcher, WatchReport};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn start(editor: Arc<Editor>, app: tauri::AppHandle) {
    let Some(root) = editor.asset_root.clone() else {
        return;
    };
    std::thread::spawn(move || {
        let mut watcher = SourceWatcher::new(root, Duration::from_millis(300));
        let mut last_error = None;
        while editor.alive.load(Ordering::Relaxed) {
            std::thread::sleep(POLL_INTERVAL);
            let result = cycle(
                &mut watcher,
                &editor.bus,
                &editor.alive,
                &editor.importing,
                Instant::now(),
            );
            match result {
                Ok(Cycle::Idle) => continue,
                Ok(Cycle::Stopped) => break,
                Ok(Cycle::Updated(report)) => {
                    last_error = None;
                    publish(&editor, &app, &watcher, report);
                }
                // Retry from a new snapshot, preserving the user's edit and the
                // watcher's prior observations. Conflicts are not source errors.
                Err(
                    ImportError::Command(CommandError::Conflict { .. })
                    | ImportError::ChangedProject
                    | ImportError::DifferentProject,
                ) => {}
                Err(error) => {
                    let message = error.to_string();
                    if last_error.as_ref() != Some(&message) {
                        log(&editor, "error", format!("Source watch: {message}"));
                        changed(&app);
                        last_error = Some(message);
                    }
                }
            }
        }
    });
}

enum Cycle {
    Idle,
    Stopped,
    Updated(WatchReport),
}

fn cycle(
    watcher: &mut SourceWatcher,
    state: &Mutex<LoadState>,
    alive: &AtomicBool,
    importing: &AtomicBool,
    now: Instant,
) -> Result<Cycle, ImportError> {
    if !alive.load(Ordering::Relaxed) {
        return Ok(Cycle::Stopped);
    }
    // Give explicit imports priority without preventing ordinary document edits.
    if importing.load(Ordering::Acquire) {
        return Ok(Cycle::Idle);
    }
    let snapshot = match state.lock() {
        Ok(load) => match &*load {
            LoadState::Ready(bus) => ImportSnapshot::capture(bus),
            LoadState::Loading => return Ok(Cycle::Idle),
            LoadState::Failed(_) => return Ok(Cycle::Stopped),
        },
        Err(_) => return Ok(Cycle::Stopped),
    };
    let prepared = watcher.prepare(snapshot, now);
    if !alive.load(Ordering::Relaxed) {
        return Ok(Cycle::Stopped);
    }
    if importing.load(Ordering::Acquire) {
        return Ok(Cycle::Idle);
    }
    match state.lock() {
        Ok(mut load) => match &mut *load {
            LoadState::Ready(bus) => watcher.commit(prepared, bus).map(Cycle::Updated),
            _ => Ok(Cycle::Idle),
        },
        Err(_) => Ok(Cycle::Stopped),
    }
}

fn publish(editor: &Editor, app: &tauri::AppHandle, watcher: &SourceWatcher, report: WatchReport) {
    let imported = report
        .imports
        .iter()
        .filter(|import| import.changed)
        .count();
    let diagnostics_changed = !report.diagnostics.is_empty() || !report.cleared.is_empty();
    if diagnostics_changed {
        if let Ok(mut diagnostics) = editor.source_diagnostics.lock() {
            *diagnostics = watcher.diagnostics().cloned().collect();
        }
        for issue in &report.diagnostics {
            log(
                editor,
                "error",
                format!("Source {}: {}", issue.source, issue.message),
            );
        }
        if !report.cleared.is_empty() {
            log(
                editor,
                "info",
                format!("Cleared {} asset source diagnostics", report.cleared.len()),
            );
        }
    }
    if imported != 0 {
        log(
            editor,
            "info",
            format!("Reimported {imported} changed source assets"),
        );
    }
    if imported != 0 || diagnostics_changed {
        changed(app);
    }
}
fn log(editor: &Editor, level: &'static str, message: String) {
    if let Ok(mut console) = editor.console.lock() {
        console.push(ConsoleEvent::new(level, message));
    }
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit_to(
        tauri::EventTarget::webview("editor"),
        "incant:engine-changed",
        (),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_cmd::{Actor, CommandBus};
    use incant_doc::Project;
    use incant_import::ImportRequest;
    use std::{fs, path::Path};

    fn buffer(root: &Path, x: f32) {
        let bytes: Vec<_> = [0_f32, 0., 0., x, 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        fs::write(root.join("mesh.bin"), bytes).unwrap();
    }
    fn fixture(root: &Path) -> CommandBus {
        buffer(root, 1.);
        fs::write(root.join("mesh.gltf"), serde_json::json!({
            "asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":36}],
            "bufferViews":[{"buffer":0,"byteLength":36}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[10,1,0]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
            "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0
        }).to_string()).unwrap();
        let mut bus = CommandBus::new(Project::empty("Native watcher")).unwrap();
        ImportSnapshot::capture(&bus)
            .prepare(
                root,
                None,
                &[ImportRequest {
                    source: "mesh.gltf".into(),
                    texture_usage: None,
                }],
            )
            .unwrap()
            .commit(&mut bus, Actor::import("fixture"), "Import")
            .unwrap();
        bus
    }
    fn updated(result: Result<Cycle, ImportError>) -> WatchReport {
        let Cycle::Updated(report) = result.unwrap() else {
            panic!("expected active watch cycle");
        };
        report
    }

    #[test]
    fn editor_worker_keeps_undo_and_recovers_broken_sources_through_shared_history() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let bus = fixture(root);
        let original = bus.project().clone();
        let state = Mutex::new(LoadState::Ready(Box::new(bus)));
        let alive = AtomicBool::new(true);
        let importing = AtomicBool::new(false);
        let mut watcher = SourceWatcher::new(root, Duration::ZERO);
        updated(cycle(
            &mut watcher,
            &state,
            &alive,
            &importing,
            Instant::now(),
        ));
        buffer(root, 2.);
        importing.store(true, Ordering::Release);
        assert!(matches!(
            cycle(&mut watcher, &state, &alive, &importing, Instant::now()).unwrap(),
            Cycle::Idle
        ));
        assert_eq!(
            state.lock().unwrap().bus_mut().unwrap().project(),
            &original
        );
        importing.store(false, Ordering::Release);
        let report = updated(cycle(
            &mut watcher,
            &state,
            &alive,
            &importing,
            Instant::now(),
        ));
        assert_eq!(report.imports.len(), 1);
        {
            let mut load = state.lock().unwrap();
            let bus = load.bus_mut().unwrap();
            assert_eq!(bus.history().len(), 2);
            assert_ne!(bus.project(), &original);
            bus.undo().unwrap();
            assert_eq!(bus.project(), &original);
        }
        assert!(
            updated(cycle(
                &mut watcher,
                &state,
                &alive,
                &importing,
                Instant::now()
            ))
            .imports
            .is_empty()
        );
        assert_eq!(
            state.lock().unwrap().bus_mut().unwrap().project(),
            &original
        );
        let model = fs::read(root.join("mesh.gltf")).unwrap();
        fs::write(root.join("mesh.gltf"), "unfinished").unwrap();
        let broken = updated(cycle(
            &mut watcher,
            &state,
            &alive,
            &importing,
            Instant::now(),
        ));
        assert_eq!(broken.diagnostics.len(), 1);
        assert!(broken.imports.is_empty());
        assert_eq!(
            state.lock().unwrap().bus_mut().unwrap().project(),
            &original
        );
        fs::write(root.join("mesh.gltf"), model).unwrap();
        buffer(root, 3.);
        let recovered = updated(cycle(
            &mut watcher,
            &state,
            &alive,
            &importing,
            Instant::now(),
        ));
        assert_eq!(recovered.imports.len(), 1);
        assert_eq!(recovered.cleared.len(), 1);
        assert_eq!(watcher.diagnostics().count(), 0);
        let final_project = state.lock().unwrap().bus_mut().unwrap().project().clone();
        alive.store(false, Ordering::Relaxed);
        buffer(root, 4.);
        assert!(matches!(
            cycle(&mut watcher, &state, &alive, &importing, Instant::now()).unwrap(),
            Cycle::Stopped
        ));
        assert_eq!(
            state.lock().unwrap().bus_mut().unwrap().project(),
            &final_project
        );
    }
}
