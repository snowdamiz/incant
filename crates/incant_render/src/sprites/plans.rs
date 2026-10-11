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
    pub frames: Option<Arc<incant_render_data::SpriteFrames>>,
    pub frame: u16,
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
    Sprite(&'a SpritePlan, Option<u16>),
    Tile(&'a TilemapPlan, &'a TileCell),
}
impl DrawItem<'_> {
    pub fn source(&self) -> &Arc<RuntimeAsset> {
        match self {
            Self::Sprite(p, _) => &p.source,
            Self::Tile(p, _) => p.data.source(),
        }
    }
    pub fn node(&self) -> usize {
        match self {
            Self::Sprite(p, _) => p.node,
            Self::Tile(p, _) => p.node,
        }
    }
    pub fn world(&self) -> glam::DMat4 {
        match self {
            Self::Sprite(p, _) => p.world,
            Self::Tile(p, _) => p.world,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::Sprite(p, _) => &p.id,
            Self::Tile(p, _) => &p.id,
        }
    }
    pub fn offset(&self) -> [f64; 2] {
        match self {
            Self::Sprite(_, _) => [0.; 2],
            Self::Tile(p, c) => {
                std::array::from_fn(|i| f64::from(c.cell[i]) * p.data.cell_size()[i])
            }
        }
    }
    pub fn sprite(&self) -> Cow<'_, SpriteRenderer> {
        match self {
            Self::Sprite(p, frame) => match frame {
                None => Cow::Borrowed(&p.sprite),
                Some(frame) => {
                    let mut sprite = p.sprite.clone();
                    sprite.region = p.frames.as_ref().expect("prepared frames").regions()
                        [usize::from(*frame)]
                    .into();
                    Cow::Owned(sprite)
                }
            },
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
    pub fn selection(&self) -> Option<(usize, u16)> {
        match self {
            Self::Sprite(p, Some(frame)) => Some((p.node, *frame)),
            _ => None,
        }
    }
    pub fn initial_frame(&self) -> u16 {
        match self {
            Self::Sprite(p, _) => p.frame,
            _ => 0,
        }
    }
    fn order(&self) -> (i16, i32, &str, Option<(incant_types::StableId, u32, u32)>) {
        match self {
            Self::Sprite(p, _) => (p.sprite.layer, p.sprite.order, &p.id, None),
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
    let mut frame_count = 0usize;
    for p in sprites {
        let n = p.frames.as_ref().map_or(1, |f| f.regions().len());
        if usize::from(p.frame) >= n
            || p.frames
                .as_ref()
                .is_some_and(|f| f.source_version() != p.source.version_id())
        {
            return Err(ResourceError::BufferSize("flipbook binding or frame").into());
        }
        frame_count = frame_count
            .checked_add(n)
            .ok_or(ResourceError::BufferSize("flipbook frames"))?;
    }
    if frame_count > incant_2d::flipbook::MAX_SCENE_FLIPBOOK_FRAMES {
        return Err(ResourceError::BufferSize("flipbook frames").into());
    }
    let count = maps
        .iter()
        .try_fold(frame_count, |sum, p| sum.checked_add(p.data.cells().len()))
        .ok_or(ResourceError::BufferSize("sprite/tile geometry"))?;
    // Every nonempty cell/sprite has at least one six-vertex quad. Refuse before
    // even allocating compact expanded draw records when that minimum cannot fit.
    if count > MAX_GEOMETRY_BYTES / (6 * LOGICAL_VERTEX_BYTES) {
        return Err(ResourceError::BufferSize("sprite/tile geometry").into());
    }
    let mut result = Vec::with_capacity(count);
    for p in sprites {
        if let Some(frames) = &p.frames {
            result.extend((0..frames.regions().len()).map(|i| DrawItem::Sprite(p, Some(i as u16))));
        } else {
            result.push(DrawItem::Sprite(p, None));
        }
    }
    for map in maps {
        result.extend(
            map.data
                .cells()
                .iter()
                .map(|cell| DrawItem::Tile(map, cell)),
        );
    }
    result.sort_unstable_by(|a, b| {
        a.order()
            .cmp(&b.order())
            .then_with(|| a.selection().cmp(&b.selection()))
    });
    Ok(result)
}
