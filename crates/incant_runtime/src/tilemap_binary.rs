//! V6 tile records: bounded counts before allocation; canonical layer-ID order.
use crate::{NativeTilemap, NativeTilemapLayer, SceneError, StableId, binary::Reader};
use incant_2d::tilemap::{MAX_AXIS, MAX_CELL_SLOTS, MAX_LAYERS};
pub(crate) struct Budget {
    slots: usize,
    maps: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            slots: crate::MAX_SCENE_TILE_SLOTS,
            maps: crate::MAX_TILEMAP_ENTITIES,
        }
    }
}
pub(crate) fn encoded_size(map: &NativeTilemap) -> usize {
    44 + map
        .layers
        .iter()
        .map(|layer| {
            27 + layer
                .cells
                .iter()
                .map(|cell| if cell.is_some() { 17 } else { 1 })
                .sum::<usize>()
        })
        .sum::<usize>()
}
pub(crate) fn write(bytes: &mut Vec<u8>, map: &NativeTilemap) {
    bytes.extend(map.tileset.0);
    for n in map.size {
        bytes.extend(n.to_le_bytes());
    }
    for n in map.cell_size {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend((map.layers.len() as u32).to_le_bytes());
    let mut layers: Vec<_> = map.layers.iter().collect();
    layers.sort_unstable_by_key(|layer| layer.id);
    for layer in layers {
        bytes.extend(layer.id.0);
        bytes.extend(layer.layer.to_le_bytes());
        bytes.extend(layer.order.to_le_bytes());
        bytes.push(u8::from(layer.solid));
        bytes.extend((layer.cells.len() as u32).to_le_bytes());
        for cell in &layer.cells {
            bytes.push(u8::from(cell.is_some()));
            if let Some(frame) = cell {
                bytes.extend(frame.0);
            }
        }
    }
}
fn flag(reader: &mut Reader<'_>) -> Result<bool, SceneError> {
    match reader.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("tilemap flag")),
    }
}
pub(crate) fn read(
    reader: &mut Reader<'_>,
    budget: &mut Budget,
) -> Result<NativeTilemap, SceneError> {
    let tileset = StableId(reader.array()?);
    let size = [reader.u32()?, reader.u32()?];
    let cell_size = reader.floats()?;
    let count = reader.u32()? as usize;
    let area = u64::from(size[0]) * u64::from(size[1]);
    if size.contains(&0)
        || size.iter().any(|n| *n > MAX_AXIS)
        || count == 0
        || count > MAX_LAYERS
        || area * count as u64 > MAX_CELL_SLOTS as u64
        || count > reader.remaining_len() / 28
    {
        return Err(SceneError::Invalid("tilemap bounds"));
    }
    let slots = area as usize * count;
    if budget.maps == 0 || slots > budget.slots {
        return Err(SceneError::Invalid("scene tilemap limit"));
    }
    budget.maps -= 1;
    budget.slots -= slots;
    let mut layers = Vec::with_capacity(count);
    let mut previous = None;
    for _ in 0..count {
        let id = StableId(reader.array()?);
        if previous.is_some_and(|p| p >= id) {
            return Err(SceneError::Invalid("tilemap layer order"));
        }
        previous = Some(id);
        let layer = i16::from_le_bytes(reader.array()?);
        let order = i32::from_le_bytes(reader.array()?);
        let solid = flag(reader)?;
        let length = reader.u32()? as usize;
        if length as u64 != area || length > reader.remaining_len() {
            return Err(SceneError::Invalid("tilemap cell count"));
        }
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            cells.push(if flag(reader)? {
                Some(StableId(reader.array()?))
            } else {
                None
            });
        }
        layers.push(NativeTilemapLayer {
            id,
            cells,
            layer,
            order,
            solid,
        });
    }
    let map = NativeTilemap {
        tileset,
        size,
        cell_size,
        layers,
    };
    map.validate()
        .map_err(|_| SceneError::Invalid("tilemap values"))?;
    Ok(map)
}
