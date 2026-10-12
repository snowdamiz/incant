//! Immutable decoded assets keyed by the document's stable IDs. Reload is staged
//! before publication so a missing or corrupt replacement leaves the running
//! project intact. This store never imports sources or changes project documents.
use crate::{AssetError, CacheKind, project_cache_directory};
use incant_doc::{Asset, AssetImportSettings, Project, TextureUsage};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeAssetError {
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error("cannot load asset {id}: {source}")]
    Load { id: String, source: AssetError },
    #[error("decoded asset payload exceeds the {0}-byte budget")]
    Budget(usize),
    #[error("asset generation counter exhausted")]
    GenerationExhausted,
    #[error("cannot resolve asset cache root: {0}")]
    CacheRoot(#[from] std::io::Error),
}

pub use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData, RuntimeAssetInfo};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct AssetChanges {
    pub loaded: Vec<String>,
    pub removed: Vec<String>,
}

/// Consumers retain an Arc to use a version safely across a reload. Resolving
/// its stable ID again returns the replacement. Renaming/moving an asset without
/// changing its content does not invalidate the decoded data or its generation.
pub struct AssetStore {
    assets: BTreeMap<String, Arc<RuntimeAsset>>,
    generation: u64,
    budget: usize,
    scope: Option<CacheScope>,
}
#[derive(PartialEq, Eq)]
enum CacheScope {
    Explicit { project: String, cache: PathBuf },
    Project { project: String, root: PathBuf },
}
impl Default for AssetStore {
    fn default() -> Self {
        Self::with_budget(256 * 1024 * 1024)
    }
}
impl AssetStore {
    pub fn with_budget(payload_bytes: usize) -> Self {
        Self {
            assets: BTreeMap::new(),
            generation: 0,
            budget: payload_bytes,
            scope: None,
        }
    }
    pub fn get(&self, id: &str) -> Option<Arc<RuntimeAsset>> {
        self.assets.get(id).cloned()
    }
    pub fn snapshot(&self) -> BTreeMap<String, RuntimeAssetInfo> {
        self.assets
            .iter()
            .map(|(id, asset)| (id.clone(), asset.info().clone()))
            .collect()
    }
    /// Load models, textures and audio from their respective cache directories. All
    /// dependencies are cooked; source files are neither needed nor consulted.
    /// A failed batch publishes no changes and consumes no generation numbers.
    pub fn sync(
        &mut self,
        project: &Project,
        cache: &Path,
    ) -> Result<AssetChanges, RuntimeAssetError> {
        let scope = CacheScope::Explicit {
            project: project.id.clone(),
            cache: std::path::absolute(cache)?,
        };
        self.sync_with(project, scope, |asset, generation| {
            load(asset, generation, &cache.join(category(asset)?.directory()))
        })
    }
    /// Load only from real cache directories inside the granted project. Already
    /// retained versions need no filesystem access; replacements are checked before
    /// reading, and a failed batch leaves those prior versions intact.
    pub fn sync_project(
        &mut self,
        project: &Project,
        root: &Path,
    ) -> Result<AssetChanges, RuntimeAssetError> {
        let root = root.canonicalize()?;
        let scope = CacheScope::Project {
            project: project.id.clone(),
            root: root.clone(),
        };
        self.sync_with(project, scope, |asset, generation| {
            load(
                asset,
                generation,
                &project_cache_directory(&root, category(asset)?)?,
            )
        })
    }
    fn sync_with(
        &mut self,
        project: &Project,
        scope: CacheScope,
        load: impl Fn(&Asset, u64) -> crate::Result<RuntimeAsset>,
    ) -> Result<AssetChanges, RuntimeAssetError> {
        project.validate()?;
        let mut next = BTreeMap::new();
        let mut generation = self.generation;
        let mut bytes = 0_usize;
        let mut changes = AssetChanges::default();
        let reuse = self.scope.as_ref() == Some(&scope);
        for (id, asset) in &project.assets {
            let version = match self
                .assets
                .get(id)
                .filter(|old| reuse && matches(old, asset))
            {
                Some(old) => Arc::clone(old),
                None => {
                    generation = generation
                        .checked_add(1)
                        .ok_or(RuntimeAssetError::GenerationExhausted)?;
                    let version =
                        load(asset, generation).map_err(|source| RuntimeAssetError::Load {
                            id: id.clone(),
                            source,
                        })?;
                    changes.loaded.push(id.clone());
                    Arc::new(version)
                }
            };
            bytes = bytes
                .checked_add(version.info().payload_bytes)
                .filter(|&bytes| bytes <= self.budget)
                .ok_or(RuntimeAssetError::Budget(self.budget))?;
            next.insert(id.clone(), version);
        }
        changes.removed = self
            .assets
            .keys()
            .filter(|id| !next.contains_key(*id))
            .cloned()
            .collect();
        self.assets = next;
        self.generation = generation;
        self.scope = Some(scope);
        Ok(changes)
    }
}
fn category(asset: &Asset) -> crate::Result<CacheKind> {
    match asset.kind.as_str() {
        "model" => Ok(CacheKind::Models),
        "texture" => Ok(CacheKind::Textures),
        "audio" => Ok(CacheKind::Audio),
        "font" => Ok(CacheKind::Fonts),
        kind => Err(AssetError::Unsupported(format!(
            "runtime asset kind {kind}"
        ))),
    }
}
fn usage(asset: &Asset) -> Option<TextureUsage> {
    if asset.kind != "texture" {
        return None;
    }
    Some(match asset.import_settings {
        Some(AssetImportSettings::Texture { usage }) => usage,
        None => TextureUsage::Color,
    })
}
fn matches(version: &RuntimeAsset, asset: &Asset) -> bool {
    version.info().kind == asset.kind
        && version.info().fingerprint == asset.sha256
        && version.usage() == usage(asset)
}
fn load(asset: &Asset, generation: u64, cache: &Path) -> crate::Result<RuntimeAsset> {
    match asset.kind.as_str() {
        "model" => RuntimeAsset::load_model(&asset.id, generation, cache, &asset.sha256),
        "texture" => {
            RuntimeAsset::load_texture(&asset.id, generation, cache, &asset.sha256, usage(asset))
        }
        "audio" => RuntimeAsset::load_audio(&asset.id, generation, cache, &asset.sha256),
        "font" => RuntimeAsset::load_font(&asset.id, generation, cache, &asset.sha256),
        kind => {
            return Err(AssetError::Unsupported(format!(
                "runtime asset kind {kind}"
            )));
        }
    }
    .map_err(Into::into)
}
