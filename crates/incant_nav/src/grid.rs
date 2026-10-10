//! Deterministic rectangular-cell navigation. Coordinates are tile indices;
//! tile/world transforms, actor clearance and movement belong to the caller.
use crate::{NavigationError, invalid, limit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BinaryHeap};

pub const MAX_GRID_CELLS: usize = 65_536;
const MAX_PATH_CELLS: usize = 4096;
const STRAIGHT: u64 = 1000;
const DIAGONAL: u64 = 1414;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationGrid {
    /// Width and height; each 1..1024, with at most 65536 cells in total.
    pub dimensions: [u16; 2],
    /// Row-major entering-cell weights. Zero blocks a cell; 1..1000 are walkable.
    #[schemars(length(max = 65536))]
    pub costs: Vec<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GridPathRequest {
    pub start: [u16; 2],
    pub end: [u16; 2],
    /// Diagonal steps require both adjacent orthogonal cells to be walkable.
    #[serde(default)]
    pub diagonal: bool,
    #[serde(default = "default_expansions")]
    pub max_expansions: u32,
}
fn default_expansions() -> u32 {
    MAX_GRID_CELLS as u32
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GridPath {
    /// Ordered adjacent cells including both endpoints. Never smoothed through obstacles.
    pub cells: Vec<[u16; 2]>,
    /// Exact integer planner cost: entering weight * 1000 straight or * 1414 diagonal.
    pub cost: u64,
    /// Host session revision; zero for a standalone library query.
    pub generation: u64,
}
impl NavigationGrid {
    pub fn validate(&self) -> Result<(), NavigationError> {
        let [width, height] = self.dimensions.map(usize::from);
        if !(1..=1024).contains(&width)
            || !(1..=1024).contains(&height)
            || width * height > MAX_GRID_CELLS
        {
            return Err(limit(
                "grid dimensions must be 1..1024 with at most 65536 cells",
            ));
        }
        if self.costs.len() != width * height || self.costs.iter().any(|&cost| cost > 1000) {
            return Err(invalid("grid requires one 0..1000 cost per row-major cell"));
        }
        Ok(())
    }
    pub fn find_path(
        &self,
        request: &GridPathRequest,
    ) -> Result<Option<GridPath>, NavigationError> {
        self.validate()?;
        let [width, height] = self.dimensions.map(usize::from);
        if request.max_expansions == 0 || request.max_expansions as usize > MAX_GRID_CELLS {
            return Err(limit("grid expansion budget must be 1..65536"));
        }
        let index = |p: [u16; 2]| -> Result<usize, NavigationError> {
            if usize::from(p[0]) >= width || usize::from(p[1]) >= height {
                return Err(invalid("grid path endpoint is outside the grid"));
            }
            Ok(usize::from(p[1]) * width + usize::from(p[0]))
        };
        let start = index(request.start)?;
        let end = index(request.end)?;
        if self.costs[start] == 0 || self.costs[end] == 0 {
            return Ok(None);
        }
        let minimum = u64::from(*self.costs.iter().filter(|&&c| c != 0).min().unwrap());
        let heuristic = |node: usize| {
            let dx = (node % width).abs_diff(end % width) as u64;
            let dy = (node / width).abs_diff(end / width) as u64;
            minimum
                * if request.diagonal {
                    dx.min(dy) * DIAGONAL + dx.abs_diff(dy) * STRAIGHT
                } else {
                    (dx + dy) * STRAIGHT
                }
        };
        let mut costs = vec![u64::MAX; self.costs.len()];
        let mut parents = vec![usize::MAX; self.costs.len()];
        let mut open = BinaryHeap::new();
        costs[start] = 0;
        open.push(Reverse((heuristic(start), 0_u64, start)));
        let mut expanded = 0_u32;
        while let Some(Reverse((_, cost, node))) = open.pop() {
            if cost != costs[node] {
                continue;
            }
            if expanded == request.max_expansions {
                return Err(limit("grid path exhausted its expansion budget"));
            }
            expanded += 1;
            if node == end {
                let mut cells = vec![];
                let mut cursor = node;
                loop {
                    if cells.len() == MAX_PATH_CELLS {
                        return Err(limit("grid path exceeds 4096 cells"));
                    }
                    cells.push([(cursor % width) as u16, (cursor / width) as u16]);
                    if cursor == start {
                        break;
                    }
                    cursor = parents[cursor];
                }
                cells.reverse();
                return Ok(Some(GridPath {
                    cells,
                    cost,
                    generation: 0,
                }));
            }
            let x = (node % width) as i32;
            let y = (node / width) as i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let diagonal = dx != 0 && dy != 0;
                    if (dx == 0 && dy == 0) || (diagonal && !request.diagonal) {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let next = ny as usize * width + nx as usize;
                    if self.costs[next] == 0 {
                        continue;
                    }
                    if diagonal
                        && (self.costs[y as usize * width + nx as usize] == 0
                            || self.costs[ny as usize * width + x as usize] == 0)
                    {
                        continue;
                    }
                    let step = if diagonal { DIAGONAL } else { STRAIGHT };
                    let candidate = cost + step * u64::from(self.costs[next]);
                    if candidate < costs[next] {
                        costs[next] = candidate;
                        parents[next] = node;
                        // Integer metric and row-major tie breaking are platform independent.
                        open.push(Reverse((candidate + heuristic(next), candidate, next)));
                    }
                }
            }
        }
        Ok(None)
    }
}
