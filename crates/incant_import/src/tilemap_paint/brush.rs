use super::{
    MAX_TILEMAP_PAINT_CELLS, Result, TilemapBrush, TilemapCellChange, TilemapPaintError,
    TilemapTerrainAction,
};
use incant_doc::AtlasLayout;
use std::collections::BTreeSet;

pub(super) fn selection(size: [u32; 2], cells: &[[u32; 2]]) -> Result<BTreeSet<usize>> {
    if cells.is_empty() || cells.len() > MAX_TILEMAP_PAINT_CELLS {
        return Err(TilemapPaintError::Cells);
    }
    let mut selected = BTreeSet::new();
    for &[x, y] in cells {
        if x >= size[0] || y >= size[1] || !selected.insert((y * size[0] + x) as usize) {
            return Err(TilemapPaintError::Cells);
        }
    }
    Ok(selected)
}

fn neighbors(index: usize, [width, height]: [u32; 2]) -> [Option<usize>; 4] {
    let width = width as usize;
    let x = index % width;
    let y = index / width;
    [
        (y + 1 < height as usize).then_some(index + width),
        (x + 1 < width).then_some(index + 1),
        index.checked_sub(width),
        (x > 0).then(|| index - 1),
    ]
}

pub(super) fn apply(
    size: [u32; 2],
    cells: &mut [Option<String>],
    selected: &BTreeSet<usize>,
    brush: &TilemapBrush,
    atlas: &AtlasLayout,
) -> Result<Vec<TilemapCellChange>> {
    let mut after = cells.to_vec();
    let mut affected = selected.clone();
    match brush {
        TilemapBrush::Paint { frame } => {
            if !atlas.frames.contains_key(frame) {
                return Err(TilemapPaintError::Frame);
            }
            for &i in selected {
                after[i] = Some(frame.clone());
            }
        }
        TilemapBrush::Erase => {
            for &i in selected {
                after[i] = None;
            }
        }
        TilemapBrush::Autotile { action, frames } => {
            if frames.iter().any(|id| !atlas.frames.contains_key(id)) {
                return Err(TilemapPaintError::Frame);
            }
            let belongs = |cell: &Option<String>| cell.as_ref().is_some_and(|f| frames.contains(f));
            if matches!(action, TilemapTerrainAction::Erase)
                && selected
                    .iter()
                    .any(|&i| cells[i].is_some() && !belongs(&cells[i]))
            {
                return Err(TilemapPaintError::ForeignTerrain);
            }
            for &i in selected {
                after[i] = match action {
                    TilemapTerrainAction::Paint => Some(frames[0].clone()),
                    TilemapTerrainAction::Erase => None,
                };
                affected.extend(neighbors(i, size).into_iter().flatten());
            }
            // Outputs stay in the same family, so mask membership is invariant
            // while assigning frames. This is equivalent to a final occupancy
            // snapshot and cannot depend on stroke order or cascade recursively.
            for &i in &affected {
                if !belongs(&after[i]) {
                    continue;
                }
                let mut mask = 0;
                for (bit, n) in neighbors(i, size).into_iter().enumerate() {
                    if n.is_some_and(|j| belongs(&after[j])) {
                        mask |= 1 << bit;
                    }
                }
                after[i] = Some(frames[mask].clone());
            }
        }
    }
    let mut changes = Vec::new();
    for i in affected {
        if cells[i] != after[i] {
            changes.push(TilemapCellChange {
                cell: [i as u32 % size[0], i as u32 / size[0]],
                before: cells[i].clone(),
                after: after[i].clone(),
            });
            cells[i] = after[i].take();
        }
    }
    Ok(changes)
}
