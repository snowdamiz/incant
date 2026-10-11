//! Authoring viewport perception shared by trusted editor/headless hosts.
use incant_doc::Project;
use incant_render_authoring::RendererAuthoringExt as _;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Lazy offscreen capture of the authored project snapshot and selected camera.
/// This does not capture the retained native Play session.
pub struct GpuPerception {
    renderer: Option<incant_render::Renderer>,
    project_root: Option<PathBuf>,
    assets: incant_assets::AssetStore,
}
impl GpuPerception {
    pub fn new(project_root: Option<PathBuf>) -> Self {
        Self {
            renderer: None,
            project_root,
            assets: Default::default(),
        }
    }
}
impl incant_agent::Perception for GpuPerception {
    fn screenshot(
        &mut self,
        project: &Project,
        camera: Option<&str>,
        width: u32,
        height: u32,
    ) -> std::result::Result<Value, incant_agent::AgentError> {
        use base64::Engine;
        if self.renderer.is_none() {
            self.renderer =
                Some(incant_render::Renderer::headless().map_err(|_| {
                    incant_agent::AgentError::Tool("GPU renderer unavailable".into())
                })?);
        }
        let renderer = self.renderer.as_mut().expect("initialized renderer");
        if let Some(root) = &self.project_root {
            self.assets.sync_project(project, root).map_err(|_| {
                incant_agent::AgentError::Tool("Cooked assets could not be loaded".into())
            })?;
        } else if !project.assets.is_empty() {
            return Err(incant_agent::AgentError::Tool(
                "This viewport has no project asset access".into(),
            ));
        }
        let scene = renderer.prepare_scene(project, &self.assets).map_err(|_| {
            incant_agent::AgentError::Tool("Scene geometry could not be prepared".into())
        })?;
        let scene = if let Some(id) = camera {
            scene
                .with_camera(id)
                .map_err(|error| incant_agent::AgentError::Tool(error.to_string()))?
        } else {
            scene
        };
        let bytes = renderer
            .screenshot_scene_png(&scene, width, height)
            .map_err(|_| incant_agent::AgentError::Tool("GPU capture failed".into()))?;
        Ok(
            json!({"mime_type":"image/png","data_url":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes)),"width":width,"height":height,"camera":camera}),
        )
    }
}
