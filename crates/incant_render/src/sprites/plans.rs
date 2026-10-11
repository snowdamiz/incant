//! Shared sprite/tile ordering. Compact startup records borrow one map binding;
//! no per-cell asset/string/configuration clone is retained.
use crate::{ResourceError, Result};
use incant_2d::{SpriteMode, SpriteRenderer, SpriteSampling};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{TileCell, TilemapInput};
use std::{borrow::Cow, sync::Arc};

pub(crate) struct SpritePlan {
    pub node: usize,
    pub source: Arc<RuntimeAsset>,
    pub sprite: SpriteRenderer,
    pub world: glam::DMat4,
    pub id: String,
}
pub(crate) struct TilemapPlan {
    pub node: usize,
    pub data: Arc<TilemapInput>,
    pub world: glam::DMat4,
    pub id: String,
}
pub(super) enum DrawItem<'a> {
    Sprite(&'a SpritePlan),
    Tile(&'a TilemapPlan, &'a TileCell),
}
impl DrawItem<'_> {
    pub fn source(&self) -> &Arc<RuntimeAsset> {
        match self {
            Self::Sprite(p) => &p.source,
            Self::Tile(p, _) => p.data.source(),
        }
    }
    pub fn node(&self) -> usize {
        match self {
            Self::Sprite(p) => p.node,
            Self::Tile(p, _) => p.node,
        }
    }
    pub fn world(&self) -> glam::DMat4 {
        match self {
            Self::Sprite(p) => p.world,
            Self::Tile(p, _) => p.world,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::Sprite(p) => &p.id,
            Self::Tile(p, _) => &p.id,
        }
    }
    pub fn offset(&self) -> [f64; 2] {
        match self {
            Self::Sprite(_) => [0.; 2],
            Self::Tile(p, c) => {
                std::array::from_fn(|i| f64::from(c.cell[i]) * p.data.cell_size()[i])
            }
        }
    }
    pub fn sprite(&self) -> Cow<'_, SpriteRenderer> {
        match self {
            Self::Sprite(p) => Cow::Borrowed(&p.sprite),
            Self::Tile(p, c) => Cow::Owned(SpriteRenderer {
                texture: String::new(),
                region: c.region.into(),
                size: p.data.cell_size(),
                pivot: [0.; 2],
                tint: [1.; 4],
                flip_x: false,
                flip_y: false,
                layer: c.layer,
                order: c.order,
                sampling: SpriteSampling::Nearest,
                mode: SpriteMode::Simple,
            }),
        }
    }
    fn order(&self) -> (i16, i32, &str, Option<(incant_types::StableId, u32, u32)>) {
        match self {
            Self::Sprite(p) => (p.sprite.layer, p.sprite.order, &p.id, None),
            Self::Tile(p, c) => (
                c.layer,
                c.order,
                &p.id,
                Some((c.layer_id, c.cell[1], c.cell[0])),
            ),
        }
    }
}
pub(super) const LOGICAL_VERTEX_BYTES: usize = 64;
pub(super) const MAX_GEOMETRY_BYTES: usize = 128 * 1024 * 1024;

pub(super) fn ordered<'a>(
    sprites: &'a [SpritePlan],
    maps: &'a [TilemapPlan],
) -> Result<Vec<DrawItem<'a>>> {
    if sprites.len() > 32768 {
        return Err(ResourceError::BufferSize("32768 sprite instances").into());
    }
    if maps.len() > incant_render_data::MAX_TILEMAPS {
        return Err(ResourceError::BufferSize("tilemap count").into());
    }
    let slots = maps
        .iter()
        .try_fold(0usize, |sum, p| sum.checked_add(p.data.cell_slots()))
        .ok_or(ResourceError::BufferSize("tilemap slots"))?;
    if slots > incant_render_data::MAX_TILE_SLOTS {
        return Err(ResourceError::BufferSize("tilemap slots").into());
    }
    let count = maps
        .iter()
        .try_fold(sprites.len(), |sum, p| {
            sum.checked_add(p.data.cells().len())
        })
        .ok_or(ResourceError::BufferSize("sprite/tile geometry"))?;
    // Every nonempty cell/sprite has at least one six-vertex quad. Refuse before
    // even allocating compact expanded draw records when that minimum cannot fit.
    if count > MAX_GEOMETRY_BYTES / (6 * LOGICAL_VERTEX_BYTES) {
        return Err(ResourceError::BufferSize("sprite/tile geometry").into());
    }
    let mut result = Vec::with_capacity(count);
    result.extend(sprites.iter().map(DrawItem::Sprite));
    for map in maps {
        result.extend(
            map.data
                .cells()
                .iter()
                .map(|cell| DrawItem::Tile(map, cell)),
        );
    }
    result.sort_unstable_by(|a, b| a.order().cmp(&b.order()));
    Ok(result)
}
