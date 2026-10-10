//! Immutable decoded assets keyed by the document's stable IDs. Reload is staged
//! before publication so a missing or corrupt replacement leaves the running
//! project intact. This store never imports sources or changes project documents.
use crate::{
    AssetError, CacheKind, CookedAudio, CookedModel, CookedTexture, load_audio, load_model,
    load_texture, project_cache_directory,
};
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

#[derive(Debug)]
pub enum RuntimeAssetData {
    Model(CookedModel),
    Texture(CookedTexture),
    Audio(Arc<CookedAudio>),
}
impl RuntimeAssetData {
    /// Decoded vertex/index/mip payload, excluding Rust metadata and allocator
    /// overhead. Audio counts its manifest; PCM stays in a validated file and
    /// mixer/decoder buffers have separate bounds. Staged replacements and
    /// retained versions add to the process peak, outside this published-set cap.
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Model(model) => {
                model
                    .meshes
                    .iter()
                    .map(|mesh| {
                        mesh.vertices.len() * size_of::<crate::Vertex>()
                            + mesh.indices.len() * size_of::<u32>()
                    })
                    .sum::<usize>()
                    + model
                        .images
                        .iter()
                        .flat_map(|image| &image.levels)
                        .map(Vec::len)
                        .sum::<usize>()
            }
            Self::Texture(texture) => texture.texture.levels.iter().map(Vec::len).sum(),
            Self::Audio(audio) => audio.resident_bytes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeAssetInfo {
    pub id: String,
    pub kind: String,
    pub fingerprint: String,
    /// Monotonically increasing within a store, including remove/re-add cycles.
    pub generation: u64,
    pub payload_bytes: usize,
}

#[derive(Debug)]
pub struct RuntimeAsset {
    info: RuntimeAssetInfo,
    usage: Option<TextureUsage>,
    data: RuntimeAssetData,
}
impl RuntimeAsset {
    pub fn info(&self) -> &RuntimeAssetInfo {
        &self.info
    }
    pub fn data(&self) -> &RuntimeAssetData {
        &self.data
    }
    fn matches(&self, asset: &Asset) -> bool {
        self.info.kind == asset.kind
            && self.info.fingerprint == asset.sha256
            && self.usage == usage(asset)
    }
}

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
            .map(|(id, asset)| (id.clone(), asset.info.clone()))
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
        self.sync_with(project, scope, |asset| {
            load(asset, &cache.join(category(asset)?.directory()))
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
        self.sync_with(project, scope, |asset| {
            load(asset, &project_cache_directory(&root, category(asset)?)?)
        })
    }
    fn sync_with(
        &mut self,
        project: &Project,
        scope: CacheScope,
        load: impl Fn(&Asset) -> crate::Result<RuntimeAssetData>,
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
                .filter(|old| reuse && old.matches(asset))
            {
                Some(old) => Arc::clone(old),
                None => {
                    let data = load(asset).map_err(|source| RuntimeAssetError::Load {
                        id: id.clone(),
                        source,
                    })?;
                    generation = generation
                        .checked_add(1)
                        .ok_or(RuntimeAssetError::GenerationExhausted)?;
                    changes.loaded.push(id.clone());
                    Arc::new(RuntimeAsset {
                        info: RuntimeAssetInfo {
                            id: id.clone(),
                            kind: asset.kind.clone(),
                            fingerprint: asset.sha256.clone(),
                            generation,
                            payload_bytes: data.payload_bytes(),
                        },
                        usage: usage(asset),
                        data,
                    })
                }
            };
            bytes = bytes
                .checked_add(version.info.payload_bytes)
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
fn load(asset: &Asset, cache: &Path) -> crate::Result<RuntimeAssetData> {
    match asset.kind.as_str() {
        "model" => load_model(cache, &asset.sha256).map(RuntimeAssetData::Model),
        "audio" => {
            load_audio(cache, &asset.sha256).map(|audio| RuntimeAssetData::Audio(Arc::new(audio)))
        }
        "texture" => {
            let texture = load_texture(cache, &asset.sha256)?;
            if Some(texture.metadata.usage) != usage(asset) {
                return Err(crate::invalid(
                    "texture settings do not match the cooked payload",
                ));
            }
            Ok(RuntimeAssetData::Texture(texture))
        }
        kind => Err(AssetError::Unsupported(format!(
            "runtime asset kind {kind}"
        ))),
    }
}
