//! Bounded rectangular maps and retained local data. No renderer or physics
//! backend is implied: consumers must explicitly install the prepared values.
mod prepare;
pub use prepare::{PreparedTilemap, TileCollisionRect, TilePlacement};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const MAX_LAYERS: usize = 32;
pub const MAX_AXIS: u32 = 4096;
pub const MAX_CELL_SLOTS: usize = 262_144;

/// One atlas texture, uniform rectangular cells, and explicit render ordering.
/// Cell (0,0) starts at the local origin; +x is right and +y is up. Texture rows
/// remain top-down as specified by the atlas; frame pivots are not tile offsets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tilemap<I = String> {
    pub tileset: I,
    pub size: [u32; 2],
    #[schemars(extend("x-incant-unit" = "m"))]
    pub cell_size: [f64; 2],
    pub layers: Vec<TilemapLayer<I>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TilemapLayer<I = String> {
    pub id: I,
    /// Exactly width*height cells in bottom-to-top row-major order. A nonempty
    /// cell references a stable atlas frame; no raw packed-page rectangles.
    pub cells: Vec<Option<I>>,
    #[serde(default)]
    pub layer: i16,
    #[serde(default)]
    pub order: i32,
    /// Every occupied cell in this layer contributes one full solid cell.
    /// Overlapping solid layers are unioned before collision preparation.
    #[serde(default)]
    pub solid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TilemapError {
    #[error("tilemap dimensions, layers or cell storage exceed supported bounds")]
    Bounds,
    #[error(
        "tilemap cell size must be finite in 0.000001..1000000 and total extent at most 1000000"
    )]
    CellSize,
    #[error("tilemap layer identities must be unique")]
    DuplicateLayer,
    #[error("tilemap identities must be canonical uppercase ULIDs")]
    Identity,
}

impl<I: Ord> Tilemap<I> {
    pub fn validate(&self) -> Result<(), TilemapError> {
        let [width, height] = self.size;
        let area = u64::from(width) * u64::from(height);
        if width == 0
            || height == 0
            || width > MAX_AXIS
            || height > MAX_AXIS
            || self.layers.is_empty()
            || self.layers.len() > MAX_LAYERS
            || area * self.layers.len() as u64 > MAX_CELL_SLOTS as u64
            || self
                .layers
                .iter()
                .any(|layer| layer.cells.len() as u64 != area)
        {
            return Err(TilemapError::Bounds);
        }
        if self.cell_size.iter().zip(self.size).any(|(cell, count)| {
            !(0.000001..=1_000_000.).contains(cell) || *cell * f64::from(count) > 1_000_000.
        }) {
            return Err(TilemapError::CellSize);
        }
        let mut ids = BTreeSet::new();
        if self.layers.iter().any(|layer| !ids.insert(&layer.id)) {
            return Err(TilemapError::DuplicateLayer);
        }
        Ok(())
    }

    /// Valid only after validation; callers use this for scene aggregate bounds.
    pub fn cell_slots(&self) -> usize {
        self.layers.iter().map(|layer| layer.cells.len()).sum()
    }
    pub fn has_collision(&self) -> bool {
        self.layers
            .iter()
            .any(|layer| layer.solid && layer.cells.iter().any(Option::is_some))
    }
}

impl Tilemap<String> {
    pub fn validate_document(&self) -> Result<(), TilemapError> {
        self.validate()?;
        let valid = crate::atlas::valid_frame_id;
        if !valid(&self.tileset)
            || self.layers.iter().any(|layer| {
                !valid(&layer.id) || layer.cells.iter().flatten().any(|frame| !valid(frame))
            })
        {
            return Err(TilemapError::Identity);
        }
        Ok(())
    }
}
