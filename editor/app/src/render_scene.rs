//! Revision-driven preparation on the render worker, outside the document lock.
//! Keep the last successfully published scene across missing/corrupt replacements.
use incant_assets::AssetStore;
use incant_doc::Project;
use incant_render::{RenderScene, Renderer};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub struct SceneRuntime {
    pub scene: RenderScene,
    assets: AssetStore,
    attempted_revision: Option<u64>,
    retry_at: Option<Instant>,
}
impl SceneRuntime {
    pub fn new(renderer: &Renderer) -> Result<Self, String> {
        let assets = AssetStore::default();
        let scene = renderer
            .prepare_scene(&Project::empty("Loading"), &assets)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            scene,
            assets,
            attempted_revision: None,
            retry_at: None,
        })
    }
    pub fn needs_update(&self, revision: u64) -> bool {
        self.attempted_revision != Some(revision)
            || self
                .retry_at
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
    pub fn update(
        &mut self,
        renderer: &Renderer,
        project: &Project,
        revision: u64,
        root: Option<&Path>,
    ) -> Result<(), String> {
        self.attempted_revision = Some(revision);
        let candidate = (|| {
            if let Some(root) = root {
                self.assets
                    .sync(project, &root.join(".incant/cache"))
                    .map_err(|e| e.to_string())?;
            } else if !project.assets.is_empty() {
                return Err("Open a saved project to load its cooked assets".into());
            }
            renderer
                .prepare_scene(project, &self.assets)
                .map_err(|e| e.to_string())
        })();
        match candidate {
            Ok(scene) => {
                self.scene = scene;
                self.retry_at = None;
                Ok(())
            }
            Err(error) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(1));
                Err(error)
            }
        }
    }
}
