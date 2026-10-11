//! Authoring-time checks of sprite/tilemap bindings against immutable cooked textures.
//! No source files, renderer, project mutation or gameplay-tick validation.
use crate::{ImportDetails, ImportOutcome};
use incant_assets::{CacheKind, TextureUsage, load_texture, project_cache_directory};
use incant_doc::{AtlasLayout, Project, SpriteMode, SpriteRegion, SpriteRenderer};
use serde::Serialize;
use std::{collections::BTreeMap, fmt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpriteDiagnosticSeverity {
    Error,
    Warning,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpriteDiagnosticCode {
    CacheUnavailable,
    TextureUsage,
    NotAtlas,
    MissingFrame,
    RawAtlasRegion,
    RegionBounds,
    NineSliceBounds,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpriteDiagnostic {
    pub scene_id: String,
    pub entity_id: String,
    pub texture_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    pub path: String,
    pub severity: SpriteDiagnosticSeverity,
    pub code: SpriteDiagnosticCode,
    pub message: String,
}
impl fmt::Display for SpriteDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scene {} entity {} texture {}",
            self.scene_id, self.entity_id, self.texture_id
        )?;
        if let Some(frame) = &self.frame {
            write!(f, " frame {frame}")?;
        }
        write!(f, ": {}", self.message)
    }
}
#[derive(Debug)]
pub struct SpriteReferenceErrors(pub Vec<SpriteDiagnostic>);
impl fmt::Display for SpriteReferenceErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "import rejected: {} invalid sprite/tilemap reference(s)",
            self.0.len()
        )?;
        for diagnostic in &self.0 {
            write!(f, "\n  {diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for SpriteReferenceErrors {}

pub(crate) struct TextureInfo {
    pub(crate) size: [u32; 2],
    pub(crate) usage: TextureUsage,
    pub(crate) atlas: Option<AtlasLayout>,
}

/// Validate the document and every sprite/tilemap cached texture binding. Each unique
/// referenced texture is loaded once and its pixel payload immediately dropped;
/// only bounded layout metadata is retained. Missing/corrupt caches are errors,
/// not a reason to reopen source images. Other asset types are outside this check.
/// A caller-authorized cache override has the same meaning as import's --cache.
pub fn validate_cached_sprites(
    project: &Project,
    root: &Path,
    cache_override: Option<&Path>,
) -> Result<Vec<SpriteDiagnostic>, incant_doc::DocumentError> {
    project.validate()?;
    let mut textures = BTreeMap::new();
    for scene in project.scenes.values() {
        for entity in scene.entities.values() {
            let texture = if let Some(value) = entity.components.get("SpriteRenderer") {
                serde_json::from_value::<SpriteRenderer>(value.clone())?.texture
            } else if let Some(value) = entity.components.get("Tilemap") {
                serde_json::from_value::<incant_doc::Tilemap>(value.clone())?.tileset
            } else {
                continue;
            };
            if textures.contains_key(&texture) {
                continue;
            }
            let asset = &project.assets[&texture];
            let loaded = (|| {
                let cache = match cache_override {
                    Some(path) => path.to_path_buf(),
                    None => project_cache_directory(root, CacheKind::Textures)?,
                };
                let cooked = load_texture(&cache, &asset.sha256)?;
                Ok::<_, incant_assets::AssetError>(TextureInfo {
                    size: [cooked.texture.width, cooked.texture.height],
                    usage: cooked.metadata.usage,
                    atlas: cooked.metadata.atlas,
                })
            })()
            .map_err(|e| e.to_string());
            textures.insert(texture, loaded);
        }
    }
    Ok(check_all(project, &textures))
}

pub(crate) fn check_imports(
    project: &Project,
    imports: &mut [ImportOutcome],
) -> Result<(), SpriteReferenceErrors> {
    let textures = imports
        .iter()
        .filter_map(|outcome| {
            let ImportDetails::Texture {
                width,
                height,
                usage,
                atlas,
                ..
            } = &outcome.details
            else {
                return None;
            };
            Some((
                outcome.asset.id.clone(),
                Ok(TextureInfo {
                    size: [*width, *height],
                    usage: *usage,
                    atlas: atlas.clone(),
                }),
            ))
        })
        .collect();
    let diagnostics = check_all(project, &textures);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == SpriteDiagnosticSeverity::Error)
        .cloned()
        .collect();
    if !errors.is_empty() {
        return Err(SpriteReferenceErrors(errors));
    }
    for import in imports {
        import.diagnostics = diagnostics
            .iter()
            .filter(|d| d.texture_id == import.asset.id)
            .cloned()
            .collect();
    }
    Ok(())
}

fn check(
    project: &Project,
    textures: &BTreeMap<String, Result<TextureInfo, String>>,
) -> Vec<SpriteDiagnostic> {
    let mut diagnostics = Vec::new();
    for (sid, scene) in &project.scenes {
        for (eid, entity) in &scene.entities {
            let Some(value) = entity.components.get("SpriteRenderer") else {
                continue;
            };
            // Both callers hold a validated document: a command-bus snapshot or
            // validate_cached_sprites' explicit document validation above.
            let sprite: SpriteRenderer =
                serde_json::from_value(value.clone()).expect("validated sprite");
            let Some(texture) = textures.get(&sprite.texture) else {
                continue;
            };
            let mut issue = |code, field, severity, message| {
                diagnostics.push(SpriteDiagnostic {
                    scene_id: sid.clone(),
                    entity_id: eid.clone(),
                    texture_id: sprite.texture.clone(),
                    frame: match &sprite.region {
                        SpriteRegion::Atlas { frame } => Some(frame.clone()),
                        _ => None,
                    },
                    path: format!("/scenes/{sid}/entities/{eid}/components/SpriteRenderer/{field}"),
                    severity,
                    code,
                    message,
                })
            };
            use SpriteDiagnosticCode as Code;
            use SpriteDiagnosticSeverity::{Error, Warning};
            let texture = match texture {
                Ok(texture) => texture,
                Err(error) => {
                    issue(Code::CacheUnavailable, "texture", Error, error.clone());
                    continue;
                }
            };
            if texture.usage != TextureUsage::Color {
                issue(
                    Code::TextureUsage,
                    "texture",
                    Error,
                    "sprites require a cooked color texture".into(),
                );
                continue;
            }
            let region = match &sprite.region {
                SpriteRegion::Atlas { frame } => {
                    let Some(atlas) = &texture.atlas else {
                        issue(
                            Code::NotAtlas,
                            "region/frame",
                            Error,
                            "frame IDs require a cooked sprite atlas; this is an ordinary texture"
                                .into(),
                        );
                        continue;
                    };
                    let Some(entry) = atlas.frames.get(frame) else {
                        issue(
                            Code::MissingFrame,
                            "region/frame",
                            Error,
                            "referenced frame does not exist in the cooked atlas".into(),
                        );
                        continue;
                    };
                    entry.region
                }
                SpriteRegion::Pixels(region) => {
                    if texture.atlas.is_some() {
                        issue(Code::RawAtlasRegion, "region", Warning, "raw pixel rectangles address the packed page and can show unrelated content after repacking; use a stable atlas frame ID".into());
                    }
                    *region
                }
            };
            let [x, y, w, h] = region;
            if u64::from(x) + u64::from(w) > u64::from(texture.size[0])
                || u64::from(y) + u64::from(h) > u64::from(texture.size[1])
            {
                issue(
                    Code::RegionBounds,
                    "region",
                    Error,
                    format!(
                        "source rectangle {region:?} exceeds cooked texture {:?}",
                        texture.size
                    ),
                );
            }
            if let SpriteMode::NineSlice {
                border: [left, right, top, bottom],
                ..
            } = sprite.mode
                && (u64::from(left) + u64::from(right) > u64::from(w)
                    || u64::from(top) + u64::from(bottom) > u64::from(h))
            {
                issue(
                    Code::NineSliceBounds,
                    "mode/border",
                    Error,
                    format!(
                        "nine-slice borders [{left}, {right}, {top}, {bottom}] exceed resolved frame {region:?}"
                    ),
                );
            }
        }
    }
    diagnostics.sort_by(|a, b| {
        (&a.texture_id, &a.scene_id, &a.entity_id, &a.frame, &a.path).cmp(&(
            &b.texture_id,
            &b.scene_id,
            &b.entity_id,
            &b.frame,
            &b.path,
        ))
    });
    diagnostics
}

fn check_all(
    project: &Project,
    textures: &BTreeMap<String, Result<TextureInfo, String>>,
) -> Vec<SpriteDiagnostic> {
    let mut diagnostics = check(project, textures);
    diagnostics.extend(crate::tilemaps::check(project, textures));
    diagnostics
}
