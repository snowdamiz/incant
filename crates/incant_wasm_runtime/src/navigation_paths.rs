//! One callback's immutable native navigation cache and latest copied reply.
use crate::BoundaryError as Error;
use incant_runtime::{NavigationSnapshot, RuntimeError, StableId};
use incant_wasm_abi::{self as abi, navigation_paths as wire};

pub(crate) struct State {
    snapshot: NavigationSnapshot,
    expansions: u32,
    reply: Option<Vec<u8>>,
    pub rejected: bool,
}
impl State {
    pub fn new(snapshot: NavigationSnapshot) -> Self {
        Self {
            snapshot,
            expansions: 0,
            reply: None,
            rejected: false,
        }
    }
    pub fn reject(&mut self) {
        self.rejected = true;
        self.reply = None;
    }
    pub fn reply(&self) -> Result<&[u8], Error> {
        if self.rejected {
            return Err(Error::Abi("rejected navigation transaction"));
        }
        self.reply
            .as_deref()
            .ok_or(Error::Abi("no navigation result"))
    }
    pub fn query(&mut self, bytes: &[u8], grid: bool) -> Result<i32, Error> {
        // Even a malformed later query cannot expose a previous query's reply.
        self.reply = None;
        if self.rejected {
            return Err(Error::Abi("rejected navigation transaction"));
        }
        let reply = if grid {
            let q = wire::decode_grid(bytes).map_err(|_| Error::Abi("grid path request"))?;
            self.charge(q.max_expansions)?;
            let result = self
                .snapshot
                .grid_path(
                    StableId(q.owner),
                    &incant_runtime::GridPathRequest {
                        start: q.start.map(|n| n as u16),
                        end: q.end.map(|n| n as u16),
                        diagonal: q.flags != 0,
                        max_expansions: q.max_expansions,
                    },
                )
                .map_err(RuntimeError::from)?;
            encode_grid(result)?
        } else {
            let q = wire::decode_path(bytes).map_err(|_| Error::Abi("navmesh path request"))?;
            self.charge(q.max_visited)?;
            let result = self
                .snapshot
                .navigation_path(
                    StableId(q.owner),
                    &incant_runtime::PathRequest {
                        start: q.start.map(|n| n as f32),
                        end: q.end.map(|n| n as f32),
                        snap_distance: q.snap_distance as f32,
                        max_visited: q.max_visited,
                    },
                )
                .map_err(RuntimeError::from)?;
            encode_path(result)?
        };
        let len = reply.len();
        self.reply = Some(reply);
        Ok(len as i32)
    }
    fn charge(&mut self, count: u32) -> Result<(), Error> {
        self.expansions = self
            .expansions
            .checked_add(count)
            .filter(|&v| v <= abi::NAVIGATION_MAX_EXPANSIONS as u32)
            .ok_or(Error::Limit("navigation expansion quota"))?;
        Ok(())
    }
}
fn encode_path(path: Option<incant_runtime::NavigationPath>) -> Result<Vec<u8>, Error> {
    let mut bytes = abi::PATH_RESULT_MAGIC.to_vec();
    let Some(path) = path else {
        bytes.extend(
            abi::PathResultHeader {
                found: 0,
                visited: 0,
                generation: 0,
                points: 0,
                corridor: 0,
                traversals: 0,
                reserved: 0,
            }
            .encode(),
        );
        return Ok(bytes);
    };
    if path.points.len() > abi::PATH_MAX_POINTS
        || path.corridor.len() > abi::PATH_MAX_CORRIDOR
        || path.traversals.len() > abi::PATH_MAX_TRAVERSALS
    {
        return Err(Error::Limit("navigation path output"));
    }
    bytes.extend(
        abi::PathResultHeader {
            found: 1,
            visited: path.visited,
            generation: path.generation,
            points: path.points.len() as u32,
            corridor: path.corridor.len() as u32,
            traversals: path.traversals.len() as u32,
            reserved: 0,
        }
        .encode(),
    );
    for point in path.points {
        bytes.extend(
            abi::PathPoint {
                position: point.map(f64::from),
            }
            .encode(),
        );
    }
    for polygon in path.corridor {
        bytes.extend(polygon.to_le_bytes());
    }
    for traversal in path.traversals {
        bytes.extend(
            abi::PathTraversal {
                link_id: link_id(&traversal.link_id)?,
                from_index: traversal.from_index,
                to_index: traversal.to_index,
                reversed: u32::from(traversal.reversed),
                reserved: 0,
            }
            .encode(),
        );
    }
    Ok(bytes)
}
fn encode_grid(path: Option<incant_runtime::GridPath>) -> Result<Vec<u8>, Error> {
    let mut bytes = abi::GRID_PATH_RESULT_MAGIC.to_vec();
    let Some(path) = path else {
        bytes.extend(
            abi::GridPathResultHeader {
                found: 0,
                cells: 0,
                generation: 0,
                cost: 0,
            }
            .encode(),
        );
        return Ok(bytes);
    };
    if path.cells.len() > abi::GRID_PATH_MAX_CELLS {
        return Err(Error::Limit("grid path output"));
    }
    bytes.extend(
        abi::GridPathResultHeader {
            found: 1,
            cells: path.cells.len() as u32,
            generation: path.generation,
            cost: path.cost,
        }
        .encode(),
    );
    for cell in path.cells {
        bytes.extend(
            abi::GridPathCell {
                position: cell.map(u32::from),
            }
            .encode(),
        );
    }
    Ok(bytes)
}
// Native navigation emits canonical ULID text for links. Convert that bounded
// metadata to the same stable big-endian bytes used by all other guest IDs.
fn link_id(text: &str) -> Result<[u8; 16], Error> {
    if text.len() != 26 {
        return Err(Error::Abi("native navigation link identity"));
    }
    let mut value = 0u128;
    for byte in text.bytes() {
        let digit = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"
            .iter()
            .position(|&c| c == byte)
            .ok_or(Error::Abi("native navigation link identity"))?;
        value = value
            .checked_mul(32)
            .and_then(|n| n.checked_add(digit as u128))
            .ok_or(Error::Abi("native navigation link identity"))?;
    }
    Ok(value.to_be_bytes())
}
