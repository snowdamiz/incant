//! Startup-only sealed root catalogs extracted from project-owned cooked models.
use crate::{CookError, stable_id};
use incant_anim::{MAX_ROOT_MOTION_INSTANCES, MAX_SCENE_ROOT_MOTION_BYTES, RootMotionCatalog};
use incant_doc::{AnimationRootMotion, Project};
use incant_runtime::StableId;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

/// No public insertion path: every table entry has passed captured cache loading
/// and model topology admission for the selected project revision.
#[derive(Default)]
pub struct RootMotionResources {
    bindings: BTreeMap<StableId, Arc<RootMotionCatalog>>,
    paths: BTreeSet<PathBuf>,
    retained_bytes: usize,
}
impl RootMotionResources {
    pub fn catalog(&self, entity: StableId) -> Option<&Arc<RootMotionCatalog>> {
        self.bindings.get(&entity)
    }
    pub fn cache_paths(&self) -> impl Iterator<Item = &Path> {
        self.paths.iter().map(PathBuf::as_path)
    }
    pub fn retained_catalog_bytes(&self) -> usize {
        self.retained_bytes
    }
}

pub fn load_project_root_motion_resources(
    project: &Project,
    scene_id: &str,
    root: &Path,
) -> Result<RootMotionResources, CookError> {
    project.validated()?;
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| CookError::MissingScene(scene_id.into()))?;
    let mut requests = BTreeMap::<String, BTreeMap<u16, Vec<StableId>>>::new();
    let mut count = 0usize;
    for entity in scene.entities.values() {
        let Some(value) = entity.components.get("AnimationRootMotion") else {
            continue;
        };
        count += 1;
        if count > MAX_ROOT_MOTION_INSTANCES {
            return Err(error(entity.id.clone(), "root-motion instance limit"));
        }
        let binding: AnimationRootMotion = serde_json::from_value(value.clone())?;
        requests
            .entry(binding.fingerprint)
            .or_default()
            .entry(binding.root)
            .or_default()
            .push(stable_id(&entity.id)?);
    }
    let mut out = RootMotionResources::default();
    if requests.is_empty() {
        return Ok(out);
    }
    let cache = crate::animation::model_cache(root)
        .map_err(|_| error(scene_id.into(), "project-owned model cache"))?;
    for (key, selections) in requests {
        let model = incant_assets_runtime::load_model(&cache, &key).map_err(|_| {
            error(
                scene_id.into(),
                "missing, malformed or stale captured model",
            )
        })?;
        let mut fingerprint = [0; 32];
        for (byte, pair) in fingerprint
            .iter_mut()
            .zip(key.as_bytes().as_chunks::<2>().0)
        {
            let hex = |n: u8| char::from(n).to_digit(16).expect("validated fingerprint") as u8;
            *byte = (hex(pair[0]) << 4) | hex(pair[1]);
        }
        for (node, entities) in selections {
            let catalog = incant_assets_runtime::animation::compile_root_motion_catalog(
                &model,
                fingerprint,
                node,
            )
            .map_err(|reason| error(ulid::Ulid::from_bytes(entities[0].0).to_string(), reason))?;
            out.retained_bytes = out
                .retained_bytes
                .checked_add(catalog.retained_bytes())
                .filter(|n| *n <= MAX_SCENE_ROOT_MOTION_BYTES)
                .ok_or_else(|| error(scene_id.into(), "scene root catalog byte limit"))?;
            let catalog = Arc::new(catalog);
            for entity in entities {
                out.bindings.insert(entity, Arc::clone(&catalog));
            }
        }
        out.paths.insert(
            incant_assets_runtime::model::key_path(&cache, &key)
                .map_err(|_| error(scene_id.into(), "captured model key"))?,
        );
    }
    Ok(out)
}
fn error(entity: String, reason: &'static str) -> CookError {
    CookError::RootMotionResource { entity, reason }
}
