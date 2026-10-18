//! Static, one-shot authoring from checked cooked texels. No runtime linkage.
mod geometry;

use crate::ImportSnapshot;
use incant_assets::{
    CacheKind, TextureFormat, TextureUsage, load_texture, project_cache_directory,
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, SpriteMode, SpriteRegion, SpriteRenderer, Transform};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub const MAX_ALPHA_COLLIDER_PIXELS: usize = 1024 * 1024;
pub const MAX_ALPHA_COLLIDER_RECTANGLES: usize = 64;

#[derive(Debug, Error)]
pub enum AlphaColliderError {
    #[error("scene or sprite entity not found")]
    Selection,
    #[error("alpha threshold must be in 1..=255 (inclusive alpha >= threshold)")]
    Threshold,
    #[error("alpha colliders require an unlinked root with only Transform and SpriteRenderer")]
    StaticSprite,
    #[error("alpha colliders require a unit-scale planar transform with a unit quaternion")]
    Transform,
    #[error("alpha colliders support only simple sprites with tint alpha 1")]
    Sprite,
    #[error("alpha colliders require a checked color RGBA8 sRGB texture")]
    Texture,
    #[error("selected sprite region or atlas frame is missing or out of bounds")]
    Region,
    #[error("selected sprite region exceeds {MAX_ALPHA_COLLIDER_PIXELS} texels")]
    PixelLimit,
    #[error("alpha mask is empty at the selected threshold; no edit was made")]
    EmptyMask,
    #[error("exact alpha cover exceeds {MAX_ALPHA_COLLIDER_RECTANGLES} rectangles")]
    RectangleLimit,
    #[error("project or revision changed after preparation; prepare again")]
    StaleProject,
    #[error(transparent)]
    Cache(#[from] incant_assets::AssetError),
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Command(#[from] incant_cmd::CommandError),
}
type Result<T> = std::result::Result<T, AlphaColliderError>;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AlphaColliderRequest {
    pub scene_id: String,
    pub entity_id: String,
    /// Base-level alpha byte, inclusive. Zero is refused so transparent pixels
    /// can never become colliders accidentally.
    pub threshold: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlphaRectangle {
    pub entity_id: String,
    /// Top-left texel rectangle relative to the selected source region.
    pub pixels: [u32; 4],
}

#[derive(Debug, Clone, Serialize)]
pub struct AlphaColliderDerivation {
    pub project_id: String,
    pub prepared_revision: u64,
    pub scene_id: String,
    pub source_entity: String,
    pub texture_id: String,
    pub texture_key: String,
    pub texture_content_sha256: String,
    pub texture_metadata_version: u32,
    pub texture_format: TextureFormat,
    pub mip_level: u32,
    pub texture_size: [u32; 2],
    pub region: [u32; 4],
    pub threshold: u8,
    pub solid_pixels: usize,
    pub rectangles: Vec<AlphaRectangle>,
}

#[derive(Debug, Serialize)]
pub struct AlphaColliderResult {
    pub revision: u64,
    pub derivation: AlphaColliderDerivation,
}

/// Prepared IDs/geometry are immutable. Publication checks the complete source
/// document and revision, then uses a single ordinary command transaction.
pub struct PreparedAlphaColliders {
    snapshot: ImportSnapshot,
    commands: Vec<Command>,
    derivation: AlphaColliderDerivation,
}
impl PreparedAlphaColliders {
    pub fn derivation(&self) -> &AlphaColliderDerivation {
        &self.derivation
    }
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
    pub fn commit(self, bus: &mut CommandBus, actor: Actor) -> Result<AlphaColliderResult> {
        if bus.revision() != self.snapshot.revision || bus.project() != &self.snapshot.project {
            return Err(AlphaColliderError::StaleProject);
        }
        let d = &self.derivation;
        bus.execute(
            self.commands,
            actor,
            format!(
                "Static alpha colliders from {} texture {} content {} threshold {}",
                d.source_entity, d.texture_key, d.texture_content_sha256, d.threshold,
            ),
            Some(self.snapshot.revision),
        )?;
        Ok(AlphaColliderResult {
            revision: bus.revision(),
            derivation: self.derivation,
        })
    }
}

impl ImportSnapshot {
    /// Derive fixed, root-level sibling Box colliders from a simple sprite's
    /// selected cooked base-alpha texels. Source files are never read. The cache
    /// loader checks key, digest, dimensions and complete mip layout within its
    /// existing bounded decode. Sampling/filtering are not geometry coverage.
    ///
    /// This is a one-time static edit, not a compound/dynamic body or live link.
    /// Moving/reimporting/deleting the source later does not change the results.
    pub fn prepare_alpha_colliders(
        self,
        root: &Path,
        cache_override: Option<&Path>,
        request: &AlphaColliderRequest,
    ) -> Result<PreparedAlphaColliders> {
        if request.threshold == 0 {
            return Err(AlphaColliderError::Threshold);
        }
        self.project.validate()?;
        let source = self
            .project
            .scenes
            .get(&request.scene_id)
            .and_then(|s| s.entities.get(&request.entity_id))
            .ok_or(AlphaColliderError::Selection)?;
        let (sprite, transform) = source_geometry(source)?;
        let asset = &self.project.assets[&sprite.texture];
        let cache = match cache_override {
            Some(path) => path.to_path_buf(),
            None => project_cache_directory(root, CacheKind::Textures)?,
        };
        let cooked = load_texture(&cache, &asset.sha256)?;
        if cooked.metadata.usage != TextureUsage::Color
            || cooked.texture.format != TextureFormat::Rgba8Srgb
        {
            return Err(AlphaColliderError::Texture);
        }
        let region = match &sprite.region {
            SpriteRegion::Pixels(rect) => *rect,
            SpriteRegion::Atlas { frame } => cooked
                .metadata
                .atlas
                .as_ref()
                .and_then(|atlas| atlas.frames.get(frame))
                .map(|frame| frame.region)
                .ok_or(AlphaColliderError::Region)?,
        };
        let size = [cooked.texture.width, cooked.texture.height];
        let (rectangles, solid_pixels) =
            geometry::cover(&cooked.texture.levels[0], size, region, request.threshold)?;
        let mut commands = Vec::with_capacity(rectangles.len());
        let mut derived = Vec::with_capacity(rectangles.len());
        let mut candidate = self.project.clone();
        for (index, pixels) in rectangles.into_iter().enumerate() {
            let entity = geometry::collider(&sprite, &transform, region, pixels, index)?;
            derived.push(AlphaRectangle {
                entity_id: entity.id.clone(),
                pixels,
            });
            candidate
                .scenes
                .get_mut(&request.scene_id)
                .expect("selected scene")
                .entities
                .insert(entity.id.clone(), entity.clone());
            commands.push(Command::CreateEntity {
                scene_id: request.scene_id.clone(),
                entity,
            });
        }
        // Enforce existing collider dimensions, coordinate and document bounds
        // before returning a preview. The command bus validates again at commit.
        candidate.validate()?;
        let derivation = AlphaColliderDerivation {
            project_id: self.project.id.clone(),
            prepared_revision: self.revision,
            scene_id: request.scene_id.clone(),
            source_entity: request.entity_id.clone(),
            texture_id: asset.id.clone(),
            texture_key: asset.sha256.clone(),
            texture_content_sha256: cooked.metadata.content_sha256,
            texture_metadata_version: cooked.metadata.version,
            texture_format: cooked.texture.format,
            mip_level: 0,
            texture_size: size,
            region,
            threshold: request.threshold,
            solid_pixels,
            rectangles: derived,
        };
        Ok(PreparedAlphaColliders {
            snapshot: self,
            commands,
            derivation,
        })
    }
}

fn source_geometry(source: &Entity) -> Result<(SpriteRenderer, Transform)> {
    if source.parent.is_some()
        || source.prefab.is_some()
        || source.components.len() != 2
        || !source.components.contains_key("Transform")
        || !source.components.contains_key("SpriteRenderer")
    {
        return Err(AlphaColliderError::StaticSprite);
    }
    let sprite: SpriteRenderer =
        serde_json::from_value(source.components["SpriteRenderer"].clone())?;
    let transform: Transform = serde_json::from_value(source.components["Transform"].clone())?;
    if sprite.mode != SpriteMode::Simple || sprite.tint[3] != 1. {
        return Err(AlphaColliderError::Sprite);
    }
    let norm: f64 = transform.rotation.iter().map(|v| v * v).sum();
    if transform.scale != [1.; 3]
        || transform.rotation[0] != 0.
        || transform.rotation[1] != 0.
        || !norm.is_finite()
        || (norm - 1.).abs() > 1e-4
    {
        return Err(AlphaColliderError::Transform);
    }
    Ok((sprite, transform))
}
