//! Joined inert sections, not a restorable native world. Decoding never invokes
//! a solver or accepts a phase claim as enclosing-owner authentication.
mod codec;
mod decode;
mod metadata;
mod shapes;
mod validate;
use crate::{ValidatedBroadOwner, ValidatedPipelineState};
pub use codec::encode_world_boundary;
pub use decode::with_decoded_world_boundary;
pub use validate::join_world_boundary;

/// Borrowed view prevents a self-referential owner and keeps every checked
/// witness alive. No conversion or installer into a live solver is provided.
#[derive(Clone, Copy)]
pub struct WorldBoundary<'a> {
    broad: &'a ValidatedBroadOwner<'a>,
    pipeline: &'a ValidatedPipelineState<'a>,
}
impl<'a> WorldBoundary<'a> {
    pub fn broad(self) -> &'a ValidatedBroadOwner<'a> {
        self.broad
    }
    pub fn pipeline(self) -> &'a ValidatedPipelineState<'a> {
        self.pipeline
    }
}
