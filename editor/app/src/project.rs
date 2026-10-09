//! Project IO and journal recovery must never run on the native event thread.
use incant_cmd::CommandBus;
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::json;
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use thiserror::Error;

pub const LOAD_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Could not read project: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not validate project: {0}")]
    Document(#[from] incant_doc::DocumentError),
    #[error("Could not recover project: {0}")]
    Journal(#[from] incant_cmd::CommandError),
    #[error(
        "Project loading timed out. Check the file's availability and folder permissions, then reopen the project."
    )]
    Timeout,
    #[error("Project loading stopped unexpectedly. Reopen the project to retry.")]
    WorkerStopped,
}
impl LoadError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "project.io",
            Self::Document(_) => "project.invalid",
            Self::Journal(_) => "project.journal",
            Self::Timeout => "project.timeout",
            Self::WorkerStopped => "project.worker",
        }
    }
}

pub enum LoadState {
    Loading,
    Ready(Box<CommandBus>),
    Failed(LoadError),
}
impl LoadState {
    pub fn bus_mut(&mut self) -> Result<&mut CommandBus, String> {
        match self {
            Self::Ready(bus) => Ok(bus),
            Self::Loading => Err("The project is still loading.".into()),
            Self::Failed(error) => Err(error.to_string()),
        }
    }
}

/// The callback runs once after a terminal state is published. A blocked OS read
/// cannot be cancelled portably: after timeout its late result is dropped (and
/// any journal lock released), never installed. There is only one read per launch.
pub fn start(
    state: Arc<Mutex<LoadState>>,
    timeout: Duration,
    load: impl FnOnce() -> Result<CommandBus, LoadError> + Send + 'static,
    changed: impl FnOnce() + Send + 'static,
) -> std::thread::JoinHandle<()> {
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = tx.send(load());
    });
    std::thread::spawn(move || {
        let result = match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(LoadError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(LoadError::WorkerStopped),
        };
        // Dropping the receiver before publishing prevents a late success from
        // retaining an exclusive journal while the UI reports failure.
        drop(rx);
        if let Ok(mut state) = state.lock() {
            *state = match result {
                Ok(bus) => LoadState::Ready(Box::new(bus)),
                Err(error) => LoadState::Failed(error),
            };
        }
        changed();
    });
    worker
}

pub fn open(path: Option<&Path>) -> Result<CommandBus, LoadError> {
    if let Some(path) = path {
        let project = Project::from_text(&std::fs::read_to_string(path)?)?;
        Ok(CommandBus::persistent(
            path.with_extension("journal.jsonl"),
            project,
        )?)
    } else {
        let mut project = Project::empty("Incant Surface Spike");
        let mut scene = Scene::new("Main");
        let mut entity = Entity::new("Cube");
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        scene.entities.insert(entity.id.clone(), entity);
        project.scenes.insert(scene.id.clone(), scene);
        Ok(CommandBus::new(project)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_cmd::{Actor, Command};

    #[test]
    fn blocked_load_leaves_state_readable_and_commands_unavailable_until_ready() {
        let state = Arc::new(Mutex::new(LoadState::Loading));
        let (release, wait) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        start(
            state.clone(),
            Duration::from_secs(5),
            move || {
                wait.recv().unwrap();
                open(None)
            },
            move || done.send(()).unwrap(),
        );
        assert!(matches!(*state.lock().unwrap(), LoadState::Loading));
        assert!(state.lock().unwrap().bus_mut().is_err());
        assert!(completed.try_recv().is_err());
        release.send(()).unwrap();
        completed.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut state = state.lock().unwrap();
        let bus = state.bus_mut().unwrap();
        assert_eq!(bus.project().name, "Incant Surface Spike");
        assert!(bus.history().is_empty());
    }

    #[test]
    fn timeout_discards_late_success_and_releases_its_journal() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.incant.json");
        let original = Project::empty("Delayed");
        std::fs::write(&path, original.canonical_text().unwrap()).unwrap();
        let worker_path = path.clone();
        let state = Arc::new(Mutex::new(LoadState::Loading));
        let (release, wait) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let worker = start(
            state.clone(),
            Duration::ZERO,
            move || {
                wait.recv().unwrap();
                open(Some(&worker_path))
            },
            move || done.send(()).unwrap(),
        );
        completed.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            *state.lock().unwrap(),
            LoadState::Failed(LoadError::Timeout)
        ));
        release.send(()).unwrap();
        worker.join().unwrap();
        // After the late read returns, its journal is not hidden in a queue.
        let reopened = open(Some(&path)).unwrap();
        assert_eq!(reopened.project().name, "Delayed");
        assert!(matches!(
            *state.lock().unwrap(),
            LoadState::Failed(LoadError::Timeout)
        ));
    }

    #[test]
    fn open_validates_files_and_restores_shared_command_history() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.incant.json");
        assert!(matches!(open(Some(&path)), Err(LoadError::Io(_))));
        std::fs::write(&path, "{}").unwrap();
        assert!(matches!(open(Some(&path)), Err(LoadError::Document(_))));
        let project = Project::empty("Recovered");
        std::fs::write(&path, project.canonical_text().unwrap()).unwrap();
        let mut bus = open(Some(&path)).unwrap();
        bus.execute(
            vec![Command::CreateScene {
                scene: Scene::new("Added"),
            }],
            Actor::user("test"),
            "Add scene",
            None,
        )
        .unwrap();
        assert!(matches!(open(Some(&path)), Err(LoadError::Journal(_))));
        drop(bus);
        let mut recovered = open(Some(&path)).unwrap();
        assert_eq!(recovered.project().scenes.len(), 1);
        assert_eq!(recovered.history().len(), 1);
        recovered.undo().unwrap();
        assert!(recovered.project().scenes.is_empty());
    }
}
