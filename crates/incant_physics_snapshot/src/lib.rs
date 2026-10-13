//! Bounded, inert pinned-solver records. This crate cannot construct a live solver.
//! A validated BVH is only one future world section, not an exact physics save.
#![forbid(unsafe_code)]

mod codec;
mod geometry;
mod geometry_codec;
mod records;
mod validate;

pub use codec::{decode, encode};
pub use geometry::*;
pub use geometry_codec::{decode_geometry, encode_geometry};
pub use records::*;
pub use validate::validate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Header,
    Version,
    Identity,
    Length,
    Limit,
    Allocation,
    Node,
    Bounds,
    Parent,
    Leaf,
    Free,
    Ownership,
    Metadata,
    Depth,
    Geometry,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid inert physics snapshot: {self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn vector<T>(length: usize) -> Result<Vec<T>> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(length)
        .map_err(|_| Error::Allocation)?;
    Ok(result)
}
