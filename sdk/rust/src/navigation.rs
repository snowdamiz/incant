//! Queries of committed native navmesh/grid caches. Paths do not move entities;
//! off-mesh point pairs require explicit gameplay traversal. Grid cells are indices.
pub use incant_wasm_abi::navigation_paths::{GridPath, Path};
pub use incant_wasm_abi::{GRID_PATH_MAX_RESULT_BYTES, PATH_MAX_RESULT_BYTES};
#[derive(Clone, Copy, Debug)]
pub struct PathRequest {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub snap_distance: f64,
    pub max_visited: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct GridPathRequest {
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub diagonal: bool,
    pub max_expansions: u32,
}
pub(crate) fn error(e: incant_wasm_abi::navigation_paths::Error) -> crate::Error {
    match e {
        incant_wasm_abi::navigation_paths::Error::Bounds => crate::Error::Bounds,
        incant_wasm_abi::navigation_paths::Error::Malformed => crate::Error::Malformed,
    }
}
