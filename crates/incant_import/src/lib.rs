//! Editor/tooling import coordination. Runtime cooked-asset loading stays in
//! incant_assets, without a dependency on the authoring command bus.
mod watch;
use incant_assets::{AssetError, Dependency, TextureFormat, TextureUsage, cook_gltf, cook_texture};
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{Asset, AssetImportSettings, Project, new_id};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
use thiserror::Error;
pub use watch::{SourceWatcher, WatchDiagnostic, WatchReport};

pub const MAX_BATCH_IMPORTS: usize = 64;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("an import batch must contain between 1 and {MAX_BATCH_IMPORTS} sources")]
    BatchSize,
    #[error("source must be a canonical, portable project-relative path: {0}")]
    Path(String),
    #[error("source appears more than once in the import batch: {0}")]
    DuplicateSource(String),
    #[error("multiple assets use source {0}; resolve their identities before reimport")]
    AmbiguousSource(String),
    #[error("unsupported import source: {0}; expected glTF, GLB, PNG, JPEG or EXR")]
    Unsupported(String),
    #[error("texture usage applies only to standalone images: {0}")]
    ModelUsage(String),
    #[error("could not cook {path}: {source}")]
    Cook { path: String, source: AssetError },
    #[error("the prepared import belongs to a different project")]
    DifferentProject,
    #[error("the project changed since import preparation; prepare again")]
    ChangedProject,
    #[error(transparent)]
    Command(#[from] CommandError),
}
pub type Result<T> = std::result::Result<T, ImportError>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    pub source: String,
    #[serde(default)]
    pub texture_usage: Option<TextureUsage>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ImportDetails {
    Model {
        meshes: usize,
        textures: usize,
        vertices: usize,
        dependencies: Vec<Dependency>,
    },
    Texture {
        width: u32,
        height: u32,
        format: TextureFormat,
        mip_levels: usize,
        usage: TextureUsage,
        dependencies: Vec<Dependency>,
    },
}
#[derive(Debug, Serialize)]
pub struct ImportOutcome {
    pub asset: Asset,
    pub changed: bool,
    pub cache_hit: bool,
    pub details: ImportDetails,
}
#[derive(Debug, Serialize)]
pub struct CommittedImports {
    pub imports: Vec<ImportOutcome>,
    pub revision: u64,
}

/// Capture under the caller's bus lock, then move into a worker. Cooking needs
/// neither a live bus borrow nor the editor's event thread.
#[derive(Clone)]
pub struct ImportSnapshot {
    project: Project,
    revision: u64,
}
impl ImportSnapshot {
    pub fn capture(bus: &CommandBus) -> Self {
        Self {
            project: bus.project().clone(),
            revision: bus.revision(),
        }
    }
    /// Cache override is a direct directory for compatibility with CLI --cache.
    /// Otherwise use the project's .incant/cache/{models,textures} directories.
    /// Cache files may be created during preparation; authored data and history
    /// are changed only by commit, after the entire batch succeeds.
    pub fn prepare(
        self,
        root: &Path,
        cache_override: Option<&Path>,
        requests: &[ImportRequest],
    ) -> Result<PreparedImports> {
        if requests.is_empty() || requests.len() > MAX_BATCH_IMPORTS {
            return Err(ImportError::BatchSize);
        }
        let mut seen = BTreeSet::new();
        let mut inputs = Vec::with_capacity(requests.len());
        // Validate identities and options before spending work on any source.
        for request in requests {
            validate_path(&request.source)?;
            if !seen.insert(&request.source) {
                return Err(ImportError::DuplicateSource(request.source.clone()));
            }
            let previous: Vec<_> = self
                .project
                .assets
                .values()
                .filter(|asset| asset.path == request.source)
                .collect();
            if previous.len() > 1 {
                return Err(ImportError::AmbiguousSource(request.source.clone()));
            }
            let extension = Path::new(&request.source)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let model = match extension.as_str() {
                "gltf" | "glb" => true,
                "png" | "jpg" | "jpeg" | "exr" => false,
                _ => return Err(ImportError::Unsupported(request.source.clone())),
            };
            if model && request.texture_usage.is_some() {
                return Err(ImportError::ModelUsage(request.source.clone()));
            }
            inputs.push((request, previous.first().copied(), model));
        }
        let mut imports = Vec::with_capacity(inputs.len());
        for (request, previous, model) in inputs {
            let cooked =
                cook(root, cache_override, request, previous, model).map_err(|source| {
                    ImportError::Cook {
                        path: request.source.clone(),
                        source,
                    }
                })?;
            imports.push(cooked);
        }
        Ok(PreparedImports {
            snapshot: self,
            imports,
        })
    }
}

/// A prepared batch cannot be edited or partially committed. On a stale snapshot
/// or invalid command the bus preserves its complete previous document/history.
pub struct PreparedImports {
    snapshot: ImportSnapshot,
    imports: Vec<ImportOutcome>,
}
impl PreparedImports {
    pub fn outcomes(&self) -> &[ImportOutcome] {
        &self.imports
    }
    pub fn commit(
        self,
        bus: &mut CommandBus,
        actor: Actor,
        description: impl Into<String>,
    ) -> Result<CommittedImports> {
        if bus.project().id != self.snapshot.project.id {
            return Err(ImportError::DifferentProject);
        }
        if bus.revision() != self.snapshot.revision {
            return Err(CommandError::Conflict {
                expected: self.snapshot.revision,
                actual: bus.revision(),
            }
            .into());
        }
        // Protect same-ID/revision snapshots supplied to a different bus instance.
        if bus.project() != &self.snapshot.project {
            return Err(ImportError::ChangedProject);
        }
        let commands: Vec<_> = self
            .imports
            .iter()
            .filter(|import| import.changed)
            .map(|import| Command::UpsertAsset {
                asset: import.asset.clone(),
            })
            .collect();
        if !commands.is_empty() {
            bus.execute(commands, actor, description, Some(self.snapshot.revision))?;
        }
        Ok(CommittedImports {
            imports: self.imports,
            revision: bus.revision(),
        })
    }
}

fn validate_path(source: &str) -> Result<()> {
    let path = Path::new(source);
    let parts: Vec<_> = path.components().collect();
    if source.is_empty()
        || source.contains(['\\', ':', '\0'])
        || parts.iter().any(|c| !matches!(c, Component::Normal(_)))
        || parts
            .iter()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
            != source
    {
        return Err(ImportError::Path(source.into()));
    }
    Ok(())
}

fn cook(
    root: &Path,
    cache_override: Option<&Path>,
    request: &ImportRequest,
    previous: Option<&Asset>,
    model: bool,
) -> incant_assets::Result<ImportOutcome> {
    let source = Path::new(&request.source);
    let (kind, fingerprint, cache_hit, import_settings, details) = if model {
        let cache = cache_directory(root, cache_override, "models")?;
        let cooked = cook_gltf(root, source, &cache)?;
        (
            "model",
            cooked.metadata.fingerprint,
            cooked.cache_hit,
            None,
            ImportDetails::Model {
                meshes: cooked.meshes.len(),
                textures: cooked.images.len(),
                vertices: cooked.meshes.iter().map(|m| m.vertices.len()).sum(),
                dependencies: cooked.metadata.dependencies,
            },
        )
    } else {
        let cache = cache_directory(root, cache_override, "textures")?;
        let usage = request.texture_usage.unwrap_or_else(|| {
            match previous.and_then(|asset| asset.import_settings.as_ref()) {
                Some(AssetImportSettings::Texture { usage }) => *usage,
                None => TextureUsage::Color,
            }
        });
        let cooked = cook_texture(root, source, &cache, usage)?;
        (
            "texture",
            cooked.metadata.fingerprint,
            cooked.cache_hit,
            Some(AssetImportSettings::Texture { usage }),
            ImportDetails::Texture {
                width: cooked.texture.width,
                height: cooked.texture.height,
                format: cooked.texture.format,
                mip_levels: cooked.texture.levels.len(),
                usage,
                dependencies: vec![cooked.metadata.dependency],
            },
        )
    };
    let asset = Asset {
        id: previous.map(|a| a.id.clone()).unwrap_or_else(new_id),
        name: previous.map(|a| a.name.clone()).unwrap_or_else(|| {
            source
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        }),
        path: request.source.clone(),
        kind: kind.into(),
        sha256: fingerprint,
        import_settings,
    };
    Ok(ImportOutcome {
        changed: previous != Some(&asset),
        asset,
        cache_hit,
        details,
    })
}

/// Default cache writes belong to the selected project. Check each directory
/// before descending, so an authored .incant/cache symlink cannot redirect a
/// host-granted import outside the project. CLI's explicit --cache remains a
/// separately caller-authorized path; engine-agent tools never accept it.
fn cache_directory(
    root: &Path,
    override_path: Option<&Path>,
    kind: &str,
) -> incant_assets::Result<std::path::PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    let root = root.canonicalize()?;
    let mut path = root.clone();
    for part in [".incant", "cache", kind] {
        path.push(part);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || !path.canonicalize()?.starts_with(&root)
        {
            return Err(AssetError::Invalid(
                "default cache directories must be real project directories, not symlinks or files"
                    .into(),
            ));
        }
    }
    Ok(path)
}
