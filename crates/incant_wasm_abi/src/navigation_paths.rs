//! Allocation-free path framing. Results borrow only caller-owned copied bytes.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Malformed,
}
pub type Result<T> = core::result::Result<T, Error>;
pub const PATH_REQUEST_BYTES: usize = 8 + PathRequest::SIZE;
pub const GRID_REQUEST_BYTES: usize = 8 + GridPathRequest::SIZE;

pub fn validate_path(q: &PathRequest) -> Result<()> {
    if q.reserved != 0
        || !q.is_finite()
        || q.start.iter().chain(&q.end).any(|n| n.abs() > 100_000.)
        || !(0.001..=10.).contains(&q.snap_distance)
        || !(1..=PATH_MAX_VISITED as u32).contains(&q.max_visited)
    {
        return Err(Error::Malformed);
    }
    Ok(())
}
pub fn validate_grid(q: &GridPathRequest) -> Result<()> {
    if q.flags > 1
        || q.start.iter().chain(&q.end).any(|&n| n > u16::MAX as u32)
        || !(1..=NAVIGATION_MAX_EXPANSIONS as u32).contains(&q.max_expansions)
    {
        return Err(Error::Malformed);
    }
    Ok(())
}
pub fn encode_path(q: PathRequest) -> Result<[u8; PATH_REQUEST_BYTES]> {
    validate_path(&q)?;
    let mut bytes = [0; PATH_REQUEST_BYTES];
    bytes[..8].copy_from_slice(PATH_REQUEST_MAGIC);
    bytes[8..].copy_from_slice(&q.encode());
    Ok(bytes)
}
pub fn encode_grid(q: GridPathRequest) -> Result<[u8; GRID_REQUEST_BYTES]> {
    validate_grid(&q)?;
    let mut bytes = [0; GRID_REQUEST_BYTES];
    bytes[..8].copy_from_slice(GRID_PATH_REQUEST_MAGIC);
    bytes[8..].copy_from_slice(&q.encode());
    Ok(bytes)
}
pub fn decode_path(bytes: &[u8]) -> Result<PathRequest> {
    if bytes.len() != PATH_REQUEST_BYTES || &bytes[..8] != PATH_REQUEST_MAGIC {
        return Err(Error::Malformed);
    }
    let q = PathRequest::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    validate_path(&q)?;
    Ok(q)
}
pub fn decode_grid(bytes: &[u8]) -> Result<GridPathRequest> {
    if bytes.len() != GRID_REQUEST_BYTES || &bytes[..8] != GRID_PATH_REQUEST_MAGIC {
        return Err(Error::Malformed);
    }
    let q = GridPathRequest::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    validate_grid(&q)?;
    Ok(q)
}

#[derive(Clone, Copy)]
pub struct Path<'a> {
    pub generation: u64,
    pub visited: u32,
    points: &'a [u8],
    corridor: &'a [u8],
    traversals: &'a [u8],
}
impl<'a> Path<'a> {
    pub fn points(&self) -> impl ExactSizeIterator<Item = [f64; 3]> + 'a {
        self.points
            .as_chunks::<{ PathPoint::SIZE }>()
            .0
            .iter()
            .map(|b| PathPoint::decode(b).expect("validated point").position)
    }
    pub fn corridor(&self) -> impl ExactSizeIterator<Item = u32> + 'a {
        self.corridor
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
    }
    pub fn traversals(&self) -> impl ExactSizeIterator<Item = PathTraversal> + 'a {
        self.traversals
            .as_chunks::<{ PathTraversal::SIZE }>()
            .0
            .iter()
            .map(|b| PathTraversal::decode(b).expect("validated traversal"))
    }
}
pub fn decode_path_result(bytes: &[u8]) -> Result<Option<Path<'_>>> {
    if bytes.len() < 8 + PathResultHeader::SIZE || &bytes[..8] != PATH_RESULT_MAGIC {
        return Err(Error::Malformed);
    }
    let h = PathResultHeader::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    if h.found > 1
        || h.reserved != 0
        || h.points as usize > PATH_MAX_POINTS
        || h.corridor as usize > PATH_MAX_CORRIDOR
        || h.traversals as usize > PATH_MAX_TRAVERSALS
        || h.visited as usize > PATH_MAX_VISITED
    {
        return Err(Error::Malformed);
    }
    let point_bytes = h.points as usize * PathPoint::SIZE;
    let corridor_bytes = h.corridor as usize * 4;
    let length = 8
        + PathResultHeader::SIZE
        + point_bytes
        + corridor_bytes
        + h.traversals as usize * PathTraversal::SIZE;
    if bytes.len() != length {
        return Err(Error::Malformed);
    }
    if h.found == 0 {
        if h.visited != 0
            || h.generation != 0
            || h.points != 0
            || h.corridor != 0
            || h.traversals != 0
        {
            return Err(Error::Malformed);
        }
        return Ok(None);
    }
    if h.points == 0 || h.corridor == 0 || h.visited == 0 || h.generation == 0 {
        return Err(Error::Malformed);
    }
    let (points, tail) = bytes[8 + PathResultHeader::SIZE..].split_at(point_bytes);
    let (corridor, traversals) = tail.split_at(corridor_bytes);
    let result = Path {
        generation: h.generation,
        visited: h.visited,
        points,
        corridor,
        traversals,
    };
    if result
        .points()
        .any(|p| p.iter().any(|n| !n.is_finite() || n.abs() > 100_000.))
    {
        return Err(Error::Malformed);
    }
    let mut previous = None;
    for t in result.traversals() {
        if t.reserved != 0
            || t.reversed > 1
            || t.from_index >= h.points
            || t.from_index.checked_add(1) != Some(t.to_index)
            || t.to_index >= h.points
            || previous.is_some_and(|end| t.from_index < end)
        {
            return Err(Error::Malformed);
        }
        previous = Some(t.to_index);
    }
    Ok(Some(result))
}

#[derive(Clone, Copy)]
pub struct GridPath<'a> {
    pub generation: u64,
    pub cost: u64,
    cells: &'a [u8],
}
impl<'a> GridPath<'a> {
    pub fn cells(&self) -> impl ExactSizeIterator<Item = [u16; 2]> + 'a {
        self.cells
            .as_chunks::<{ GridPathCell::SIZE }>()
            .0
            .iter()
            .map(|b| {
                GridPathCell::decode(b)
                    .expect("validated cell")
                    .position
                    .map(|n| n as u16)
            })
    }
}
pub fn decode_grid_result(bytes: &[u8]) -> Result<Option<GridPath<'_>>> {
    if bytes.len() < 8 + GridPathResultHeader::SIZE || &bytes[..8] != GRID_PATH_RESULT_MAGIC {
        return Err(Error::Malformed);
    }
    let h = GridPathResultHeader::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    if h.found > 1
        || h.cells as usize > GRID_PATH_MAX_CELLS
        || bytes.len() != 8 + GridPathResultHeader::SIZE + h.cells as usize * GridPathCell::SIZE
    {
        return Err(Error::Malformed);
    }
    if h.found == 0 {
        if h.generation != 0 || h.cost != 0 || h.cells != 0 {
            return Err(Error::Malformed);
        }
        return Ok(None);
    }
    if h.cells == 0
        || h.generation == 0
        || (h.cells == 1 && h.cost != 0)
        || (h.cells > 1 && h.cost == 0)
    {
        return Err(Error::Malformed);
    }
    let cells = &bytes[8 + GridPathResultHeader::SIZE..];
    for b in cells.as_chunks::<{ GridPathCell::SIZE }>().0 {
        if GridPathCell::decode(b)
            .unwrap()
            .position
            .iter()
            .any(|&n| n > u16::MAX as u32)
        {
            return Err(Error::Malformed);
        }
    }
    let result = GridPath {
        generation: h.generation,
        cost: h.cost,
        cells,
    };
    let mut previous: Option<[u16; 2]> = None;
    for cell in result.cells() {
        if previous
            .is_some_and(|p| p == cell || p[0].abs_diff(cell[0]) > 1 || p[1].abs_diff(cell[1]) > 1)
        {
            return Err(Error::Malformed);
        }
        previous = Some(cell);
    }
    Ok(Some(result))
}
