use super::{Tilemap, TilemapError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePlacement<I> {
    pub layer: I,
    pub frame: I,
    pub cell: [u32; 2],
}

/// Integer half-open grid bounds, without any guessed depth or physics material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCollisionRect {
    pub origin: [u32; 2],
    pub size: [u32; 2],
}

/// Immutable local placements sorted by (layer, order, stable layer ID, y, x).
/// Reuse these slices until bindings change; ordinary motion changes only the
/// entity's committed world transform. No atlas payload or world clone is held.
#[derive(Debug)]
pub struct PreparedTilemap<I> {
    cell_size: [f64; 2],
    placements: Vec<TilePlacement<I>>,
    collision: Vec<TileCollisionRect>,
}
impl<I> PreparedTilemap<I> {
    pub fn placements(&self) -> &[TilePlacement<I>] {
        &self.placements
    }
    pub fn collision(&self) -> &[TileCollisionRect] {
        &self.collision
    }
    pub fn cell_size(&self) -> [f64; 2] {
        self.cell_size
    }
    /// Local [min,max] bounds; frame pivots cannot change grid/collision layout.
    pub fn cell_bounds(&self, cell: [u32; 2]) -> [[f64; 2]; 2] {
        [
            std::array::from_fn(|i| f64::from(cell[i]) * self.cell_size[i]),
            std::array::from_fn(|i| (f64::from(cell[i]) + 1.) * self.cell_size[i]),
        ]
    }
}

impl<I: Ord + Clone> Tilemap<I> {
    /// Prepare once at a cook/load/configuration boundary. Exact union coverage
    /// is guaranteed; equal horizontal runs are merged vertically. This is a
    /// deterministic decomposition, not a mathematically minimal rectangle set.
    pub fn prepare(&self) -> Result<PreparedTilemap<I>, TilemapError> {
        self.validate()?;
        let [width, height] = self.size;
        let area = width as usize * height as usize;
        let mut solid = vec![false; area];
        let mut layers: Vec<_> = self.layers.iter().collect();
        layers.sort_by_key(|layer| (layer.layer, layer.order, &layer.id));
        let count = layers
            .iter()
            .map(|layer| layer.cells.iter().flatten().count())
            .sum();
        let mut placements = Vec::with_capacity(count);
        for layer in layers {
            for (index, frame) in layer.cells.iter().enumerate() {
                if let Some(frame) = frame {
                    placements.push(TilePlacement {
                        layer: layer.id.clone(),
                        frame: frame.clone(),
                        cell: [index as u32 % width, index as u32 / width],
                    });
                    solid[index] |= layer.solid;
                }
            }
        }
        let collision = rectangles(&solid, width, height);
        Ok(PreparedTilemap {
            cell_size: self.cell_size,
            placements,
            collision,
        })
    }
}

// Each active row run stores the index of its rectangle. Both run vectors are
// reused. Their maximum size is one grid row; rectangles never exceed cells.
fn rectangles(solid: &[bool], width: u32, height: u32) -> Vec<TileCollisionRect> {
    let mut rectangles: Vec<TileCollisionRect> = Vec::new();
    let mut previous: Vec<(u32, u32, usize)> = Vec::new();
    let mut next = Vec::new();
    for y in 0..height {
        next.clear();
        let mut x = 0;
        let mut cursor = 0;
        while x < width {
            if !solid[(y * width + x) as usize] {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && solid[(y * width + x) as usize] {
                x += 1;
            }
            let length = x - start;
            while cursor < previous.len() && previous[cursor].0 < start {
                cursor += 1;
            }
            let index = if cursor < previous.len()
                && previous[cursor].0 == start
                && previous[cursor].1 == length
            {
                let index = previous[cursor].2;
                rectangles[index].size[1] += 1;
                index
            } else {
                let index = rectangles.len();
                rectangles.push(TileCollisionRect {
                    origin: [start, y],
                    size: [length, 1],
                });
                index
            };
            next.push((start, length, index));
        }
        std::mem::swap(&mut previous, &mut next);
    }
    rectangles
}
