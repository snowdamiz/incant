//! Concrete native gameplay host for editor/headless provider loops. The trusted
//! caller binds the root; agent requests cannot open files or invoke a compiler.
use incant_agent::{AgentError, EngineHost, play::RunRequest};
use incant_doc::Project;
use serde_json::Value;
use std::{path::PathBuf, sync::atomic::AtomicBool};

pub struct Host<P> {
    root: PathBuf,
    viewport: P,
    play: crate::controller::Controller,
}
impl<P> Host<P> {
    pub fn new(root: PathBuf, viewport: P, fallback: crate::FallbackCompiler) -> Self {
        Self {
            root,
            viewport,
            play: crate::Controller::new(fallback),
        }
    }
}
impl<P: EngineHost> EngineHost for Host<P> {
    fn native_play_available(&self) -> bool {
        true
    }
    fn shared_play_available(&self) -> bool {
        true
    }
    fn play_run_checked(
        &mut self,
        project: &Project,
        revision: u64,
        request: RunRequest,
        cancel: &AtomicBool,
        validate: &mut dyn FnMut() -> Result<(), AgentError>,
    ) -> Result<Value, AgentError> {
        self.play
            .run_checked(project, revision, &self.root, request, cancel, &mut || {
                validate().map_err(crate::controller::Error::Snapshot)
            })
            .map_err(agent_error)
    }
    fn play_run(
        &mut self,
        project: &Project,
        revision: u64,
        request: RunRequest,
        cancel: &AtomicBool,
    ) -> Result<Value, AgentError> {
        self.play
            .run(project, revision, &self.root, request, cancel)
            .map_err(agent_error)
    }
    fn play_state_query(&mut self, path: &str) -> Result<Value, AgentError> {
        self.play
            .state_query(path)
            .map_err(|error| AgentError::Tool(error.to_string()))
    }
    fn profiler_available(&self) -> bool {
        cfg!(feature = "agent-profiler")
    }
    fn profiler_read(
        &self,
        project_id: &str,
        revision: u64,
        frames: usize,
    ) -> Result<Value, AgentError> {
        self.play
            .profiler_read(project_id, revision, frames)
            .map_err(agent_error)
    }
    fn screenshot(
        &mut self,
        project: &Project,
        camera: Option<&str>,
        width: u32,
        height: u32,
    ) -> Result<Value, AgentError> {
        self.viewport.screenshot(project, camera, width, height)
    }
}

fn agent_error(error: crate::controller::Error) -> AgentError {
    match error {
        crate::controller::Error::Interrupted => AgentError::Interrupted,
        crate::controller::Error::Snapshot(error) => error,
        error => AgentError::Tool(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
