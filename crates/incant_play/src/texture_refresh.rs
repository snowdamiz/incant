//! Cold authoring coordination only. This basis never enters a native world or
//! shipped runtime, and is not read on ordinary ticks.
use crate::*;
use incant_render_game::{PreparedTextureRefresh, TextureRefreshTicket};
use incant_runtime::{VisualAssetKind, VisualAssetRef};

pub struct TextureRefreshBasis {
    normalized: String,
    references: Vec<VisualAssetRef>,
    interpretations: std::collections::BTreeMap<StableId, Option<incant_doc::AssetImportSettings>>,
}
fn normalized(project: &incant_doc::Project) -> Result<String, StartupError> {
    project.validate().map_err(native)?;
    let mut project = project.clone();
    // All other project data, paths, names, settings and asset interpretation
    // remain exact. Revisions/history live on the command bus, not in this basis.
    for asset in project
        .assets
        .values_mut()
        .filter(|a| matches!(a.kind.as_str(), "texture" | "model"))
    {
        asset.sha256 = "0".repeat(64);
    }
    project.canonical_text().map_err(native)
}
impl TextureRefreshBasis {
    pub fn capture(
        project: &incant_doc::Project,
        references: &[VisualAssetRef],
    ) -> Result<Self, StartupError> {
        let interpretations = references
            .iter()
            .map(|r| {
                let label = ulid::Ulid::from_bytes(r.id.0).to_string();
                (
                    r.id,
                    project
                        .assets
                        .get(&label)
                        .and_then(|a| a.import_settings.clone()),
                )
            })
            .collect();
        Ok(Self {
            normalized: normalized(project)?,
            references: references.to_vec(),
            interpretations,
        })
    }
    pub fn prepare(
        &self,
        project: &incant_doc::Project,
        root: &Path,
        ticket: TextureRefreshTicket,
    ) -> Result<Option<PreparedTextureRefresh>, StartupError> {
        project.validate().map_err(native)?;
        let changed = self.references.iter().any(|reference| {
            let label = ulid::Ulid::from_bytes(reference.id.0).to_string();
            project.assets.get(&label).is_none_or(|asset| {
                let kind = match reference.kind {
                    VisualAssetKind::Model => "model",
                    VisualAssetKind::Texture => "texture",
                };
                asset.kind != kind
                    || !ticket.matches_recipe(reference.id, reference.kind, &asset.sha256)
                    || self
                        .interpretations
                        .get(&reference.id)
                        .is_some_and(|usage| usage != &asset.import_settings)
            })
        });
        // Ordinary entity/memory edits do not request an asset publication. A
        // later real asset change still checks the entire immutable startup basis.
        if !changed {
            return Ok(None);
        }
        if normalized(project)? != self.normalized {
            return Err(invalid(
                "live asset refresh supports compatible existing texture/model content only; the running game retains its prior complete catalog",
            ));
        }
        let mut references = self.references.clone();
        for reference in &mut references {
            let asset = project
                .assets
                .get(&ulid::Ulid::from_bytes(reference.id.0).to_string())
                .ok_or_else(|| invalid("visual asset removed"))?;
            for (byte, pair) in reference
                .fingerprint
                .iter_mut()
                .zip(asset.sha256.as_bytes().as_chunks::<2>().0)
            {
                let digit = |v: u8| {
                    char::from(v)
                        .to_digit(16)
                        .ok_or_else(|| invalid("invalid visual asset fingerprint"))
                };
                *byte = ((digit(pair[0])? << 4) | digit(pair[1])?) as u8;
            }
        }
        let cache = if references
            .iter()
            .any(|r| r.kind == VisualAssetKind::Texture)
        {
            incant_assets::project_cache_directory(root, incant_assets::CacheKind::Textures)
                .map_err(native)?
        } else {
            root.join(".incant/cache/textures")
        };
        let models = if references.iter().any(|r| r.kind == VisualAssetKind::Model) {
            incant_assets::project_cache_directory(root, incant_assets::CacheKind::Models)
                .map_err(native)?
        } else {
            root.join(".incant/cache/models")
        };
        ticket
            .prepare_visuals(
                &references,
                incant_render_game::VisualCaches {
                    models: &models,
                    textures: &cache,
                },
            )
            .map(Some)
            .map_err(native)
    }
}
