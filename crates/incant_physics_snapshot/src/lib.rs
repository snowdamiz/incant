//! Bounded, inert pinned-solver records. This crate cannot construct a live solver.
//! A validated BVH is only one future world section, not an exact physics save.
#![forbid(unsafe_code)]

mod arena;
mod arena_codec;
mod attachment;
mod attachment_codec;
mod body_state;
mod coarena;
mod coarena_codec;
mod codec;
mod contact_manifolds;
mod contact_points;
mod contact_points_codec;
mod geometry;
mod geometry_codec;
mod graph;
mod graph_codec;
mod island_state;
mod pair_state;
mod records;
mod validate;

pub use arena::*;
pub use arena_codec::{decode_arena, encode_arena};
pub use attachment::*;
pub use attachment_codec::{decode_attachments, encode_attachments};
pub use body_state::*;
pub use coarena::*;
pub use coarena_codec::{decode_coarena, encode_coarena};
pub use codec::{decode, encode};
pub use contact_manifolds::*;
pub use contact_points::*;
pub use contact_points_codec::{decode_contact_points, encode_contact_points};
pub use geometry::*;
pub use geometry_codec::{decode_geometry, encode_geometry};
pub use graph::*;
pub use graph_codec::{decode_graph, encode_graph};
pub use island_state::*;
pub use pair_state::*;
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
    Generation,
    Adjacency,
    Contact,
    PendingMutation,
    Manifold,
    Workspace,
    Body,
    UnsupportedBody,
    Island,
    UnsupportedIsland,
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
