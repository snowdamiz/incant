//! Concrete native gameplay host for the headless provider loop. The trusted
//! caller binds the root; agent requests cannot open files or invoke a compiler.
use incant_agent::{AgentError, EngineHost, play::RunRequest};
use incant_doc::Project;
use serde_json::Value;
use std::{path::PathBuf, sync::atomic::AtomicBool};

pub(crate) struct Host<P> {
    root: PathBuf,
    viewport: P,
    play: crate::native_rpc::Controller,
}
impl<P> Host<P> {
    pub fn new(root: PathBuf, viewport: P) -> Self {
        Self {
            root,
            viewport,
            play: Default::default(),
        }
    }
}
impl<P: EngineHost> EngineHost for Host<P> {
    fn native_play_available(&self) -> bool {
        true
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
            .map_err(|error| {
                if matches!(error, crate::native_rpc::Error::Interrupted) {
                    AgentError::Interrupted
                } else {
                    AgentError::Tool(error.to_string())
                }
            })
    }
    fn play_state_query(&mut self, path: &str) -> Result<Value, AgentError> {
        self.play
            .state_query(path)
            .map_err(|error| AgentError::Tool(error.to_string()))
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

#[cfg(test)]
mod tests;
