//! Immutable checked tile bindings. No project, importer, GPU or physics backend.
use crate::SceneError;
use incant_2d::Tilemap;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData, TextureUsage};
use incant_types::StableId;
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_TILEMAPS: usize = 1024;
pub const MAX_TILE_SLOTS: usize = 1_048_576;

/// Full-cell nearest-sampled atlas placement. These values are constructed only
/// after checking the map and sealed atlas; consumers cannot mutate the binding.
pub struct TileCell {
    pub layer_id: StableId,
    pub layer: i16,
    pub order: i32,
    pub cell: [u32; 2],
    pub region: [u32; 4],
}

/// One immutable version of a map's visual bindings. Retained renderers hold an
/// Arc and compare identity on motion updates; no cell scan is needed per frame.
/// Creating another value, even with equal inputs, requires a binding rebuild.
pub struct TilemapInput {
    source: Arc<RuntimeAsset>,
    cell_size: [f64; 2],
    slots: usize,
    cells: Vec<TileCell>,
}
impl TilemapInput {
    pub fn new(source: Arc<RuntimeAsset>, map: &Tilemap<StableId>) -> Result<Self, SceneError> {
        Self::resolve(source, map, |id| Ok(*id))
    }
    /// Authoring adapters may convert already validated textual IDs here. This
    /// reads only typed values and the supplied immutable cooked asset version.
    pub fn from_string_ids(
        source: Arc<RuntimeAsset>,
        map: &Tilemap<String>,
    ) -> Result<Self, SceneError> {
        map.validate_document().map_err(SceneError::Tilemap)?;
        Self::resolve(source, map, |id| parse_id(id))
    }
    fn resolve<I: Ord>(
        source: Arc<RuntimeAsset>,
        map: &Tilemap<I>,
        identify: impl Fn(&I) -> Result<StableId, SceneError>,
    ) -> Result<Self, SceneError> {
        map.validate().map_err(SceneError::Tilemap)?;
        let identity = identify(&map.tileset)?;
        // Native descriptor loaders cache a hexadecimal label; authoring stores
        // the canonical ULID. Both represent the same checked binary identity.
        let label = &source.info().id;
        if source.usage() != Some(TextureUsage::Color)
            || (*label != ulid::Ulid::from_bytes(identity.0).to_string()
                && *label != format!("{:032x}", u128::from_be_bytes(identity.0)))
        {
            return Err(SceneError::TilemapBinding);
        }
        let RuntimeAssetData::Texture(texture) = source.data() else {
            return Err(SceneError::TilemapBinding);
        };
        let atlas = texture
            .metadata
            .atlas
            .as_ref()
            .ok_or(SceneError::TilemapBinding)?;
        let mut regions = BTreeMap::new();
        for frame in map
            .layers
            .iter()
            .flat_map(|layer| layer.cells.iter().flatten())
        {
            if regions.contains_key(frame) {
                continue;
            }
            let id = identify(frame)?;
            let label = ulid::Ulid::from_bytes(id.0).to_string();
            let region = atlas
                .frames
                .get(&label)
                .ok_or(SceneError::TilemapFrame(id))?
                .region;
            if region[2] == 0
                || region[3] == 0
                || u64::from(region[0]) + u64::from(region[2]) > u64::from(texture.texture.width)
                || u64::from(region[1]) + u64::from(region[3]) > u64::from(texture.texture.height)
            {
                return Err(SceneError::TilemapBinding);
            }
            regions.insert(frame, region);
        }
        let occupied = map
            .layers
            .iter()
            .map(|l| l.cells.iter().flatten().count())
            .sum();
        let mut cells = Vec::with_capacity(occupied);
        for layer in &map.layers {
            let layer_id = identify(&layer.id)?;
            for (index, frame) in layer.cells.iter().enumerate() {
                if let Some(frame) = frame {
                    cells.push(TileCell {
                        layer_id,
                        layer: layer.layer,
                        order: layer.order,
                        cell: [index as u32 % map.size[0], index as u32 / map.size[0]],
                        region: regions[frame],
                    });
                }
            }
        }
        cells.sort_unstable_by_key(|c| (c.layer, c.order, c.layer_id, c.cell[1], c.cell[0]));
        Ok(Self {
            source,
            cell_size: map.cell_size,
            slots: map.cell_slots(),
            cells,
        })
    }
    pub fn source(&self) -> &Arc<RuntimeAsset> {
        &self.source
    }
    pub fn cell_size(&self) -> [f64; 2] {
        self.cell_size
    }
    pub fn cell_slots(&self) -> usize {
        self.slots
    }
    pub fn cells(&self) -> &[TileCell] {
        &self.cells
    }
}
fn parse_id(id: &str) -> Result<StableId, SceneError> {
    let value: ulid::Ulid = id.parse().map_err(|_| SceneError::TilemapBinding)?;
    if value.to_string() != id {
        return Err(SceneError::TilemapBinding);
    }
    Ok(StableId(value.to_bytes()))
}
