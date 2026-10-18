//! One-shot rectangular tile authoring. Rules never enter the cooked runtime.
mod brush;

use crate::ImportSnapshot;
use incant_assets::{CacheKind, TextureUsage, load_texture, project_cache_directory};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Tilemap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub const MAX_TILEMAP_PAINT_CELLS: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TilemapPaintRequest {
    pub scene_id: String,
    pub entity_id: String,
    pub layer_id: String,
    pub expected_revision: u64,
    /// Distinct grid coordinates, +X right and +Y up; never world coordinates.
    pub cells: Vec<[u32; 2]>,
    pub brush: TilemapBrush,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TilemapBrush {
    Paint {
        frame: String,
    },
    Erase,
    /// All sixteen cardinal masks are required. Their frame IDs define one
    /// terrain family. Duplicate outputs are allowed; foreign frames do not join.
    Autotile {
        action: TilemapTerrainAction,
        /// Index = N(1) | E(2) | S(4) | W(8). Out-of-map is empty.
        frames: Box<[String; 16]>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TilemapTerrainAction {
    Paint,
    Erase,
}

#[derive(Debug, Error)]
pub enum TilemapPaintError {
    #[error("scene, tilemap entity or stable layer identity not found")]
    Selection,
    #[error("painting requires 1..=4096 distinct in-bounds grid cells")]
    Cells,
    #[error("tilemap painting requires a checked cooked color atlas")]
    Atlas,
    #[error("a tilemap or brush frame is absent from the checked atlas")]
    Frame,
    #[error("autotile erase refuses selected foreign terrain; use ordinary erase explicitly")]
    ForeignTerrain,
    #[error("project or revision changed; prepare again")]
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
type Result<T> = std::result::Result<T, TilemapPaintError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TilemapCellChange {
    pub cell: [u32; 2],
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TilemapPaintResult {
    pub project_id: String,
    pub prepared_revision: u64,
    pub revision: u64,
    pub scene_id: String,
    pub entity_id: String,
    pub layer_id: String,
    pub texture_id: String,
    pub texture_key: String,
    pub texture_content_sha256: String,
    pub atlas_id: String,
    /// False means no transaction, revision increment or redo invalidation.
    pub changed: bool,
    /// Sorted bottom-to-top row-major. At most 5 * MAX_TILEMAP_PAINT_CELLS.
    pub changes: Vec<TilemapCellChange>,
}

pub struct PreparedTilemapPaint {
    snapshot: ImportSnapshot,
    commands: Vec<Command>,
    preview: TilemapPaintResult,
}
impl PreparedTilemapPaint {
    pub fn preview(&self) -> &TilemapPaintResult {
        &self.preview
    }
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
    pub fn commit(mut self, bus: &mut CommandBus, actor: Actor) -> Result<TilemapPaintResult> {
        // Full document equality also catches another bus/project with the same revision.
        if bus.revision() != self.snapshot.revision || bus.project() != &self.snapshot.project {
            return Err(TilemapPaintError::StaleProject);
        }
        if self.preview.changed {
            bus.execute(
                self.commands,
                actor,
                format!(
                    "Paint tilemap {} layer {} atlas {} content {}",
                    self.preview.entity_id,
                    self.preview.layer_id,
                    self.preview.texture_key,
                    self.preview.texture_content_sha256,
                ),
                Some(self.snapshot.revision),
            )?;
        }
        self.preview.revision = bus.revision();
        Ok(self.preview)
    }
}

impl ImportSnapshot {
    /// Validate the selected map and every frame against immutable cooked atlas
    /// data. Source images/descriptors are not reopened. A simultaneous stroke
    /// prepares one ordinary SetComponent transaction; commit repeats validation.
    pub fn prepare_tilemap_paint(
        self,
        root: &Path,
        cache_override: Option<&Path>,
        request: &TilemapPaintRequest,
    ) -> Result<PreparedTilemapPaint> {
        if request.expected_revision != self.revision {
            return Err(TilemapPaintError::StaleProject);
        }
        self.project.validate()?;
        let source = self
            .project
            .scenes
            .get(&request.scene_id)
            .and_then(|s| s.entities.get(&request.entity_id))
            .and_then(|e| e.components.get("Tilemap"))
            .ok_or(TilemapPaintError::Selection)?;
        let mut map: Tilemap = serde_json::from_value(source.clone())?;
        let layer = map
            .layers
            .iter()
            .position(|l| l.id == request.layer_id)
            .ok_or(TilemapPaintError::Selection)?;
        let selected = brush::selection(map.size, &request.cells)?;
        let asset = &self.project.assets[&map.tileset];
        let cache = match cache_override {
            Some(path) => path.to_path_buf(),
            None => project_cache_directory(root, CacheKind::Textures)?,
        };
        let cooked = load_texture(&cache, &asset.sha256)?;
        if cooked.metadata.usage != TextureUsage::Color {
            return Err(TilemapPaintError::Atlas);
        }
        let atlas = cooked
            .metadata
            .atlas
            .as_ref()
            .ok_or(TilemapPaintError::Atlas)?;
        if map
            .layers
            .iter()
            .flat_map(|l| l.cells.iter().flatten())
            .any(|frame| !atlas.frames.contains_key(frame))
        {
            return Err(TilemapPaintError::Frame);
        }
        let changes = brush::apply(
            map.size,
            &mut map.layers[layer].cells,
            &selected,
            &request.brush,
            atlas,
        )?;
        let changed = !changes.is_empty();
        let commands = if changed {
            let value = serde_json::to_value(&map)?;
            // Filling an empty solid layer can activate existing planar/collider
            // constraints. Refuse invalid previews before the commit boundary.
            let mut candidate = self.project.clone();
            candidate
                .scenes
                .get_mut(&request.scene_id)
                .expect("selected scene")
                .entities
                .get_mut(&request.entity_id)
                .expect("selected entity")
                .components
                .insert("Tilemap".into(), value.clone());
            candidate.validate()?;
            vec![Command::SetComponent {
                scene_id: request.scene_id.clone(),
                entity_id: request.entity_id.clone(),
                component: "Tilemap".into(),
                value,
            }]
        } else {
            Vec::new()
        };
        let preview = TilemapPaintResult {
            project_id: self.project.id.clone(),
            prepared_revision: self.revision,
            revision: self.revision,
            scene_id: request.scene_id.clone(),
            entity_id: request.entity_id.clone(),
            layer_id: request.layer_id.clone(),
            texture_id: map.tileset,
            texture_key: asset.sha256.clone(),
            texture_content_sha256: cooked.metadata.content_sha256.clone(),
            atlas_id: atlas.id.clone(),
            changed,
            changes,
        };
        Ok(PreparedTilemapPaint {
            snapshot: self,
            commands,
            preview,
        })
    }
}
