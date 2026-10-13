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
    /// Exact shared ownership, for separately gated native admission. This does
    /// not grant construction or authenticate the enclosing runtime.
    pub fn attachments(self) -> &'a crate::ValidatedAttachments<'a> {
        self.broad
            .context()
            .narrow
            .context()
            .rigid
            .collider_state()
            .context()
            .bodies
            .attachments()
    }
    /// Exact witnessed arena layout, with no construction authority.
    pub fn body_arena(self) -> &'a crate::ValidatedArena {
        self.attachments().body_arena()
    }
    /// Exact witnessed arena layout, with no construction authority.
    pub fn collider_arena(self) -> &'a crate::ValidatedArena {
        self.attachments().collider_arena()
    }
    pub fn manifolds(self) -> &'a crate::ValidatedManifolds<'a> {
        self.broad
            .context()
            .narrow
            .context()
            .rigid
            .pairs()
            .manifolds()
    }
    pub fn broad(self) -> &'a ValidatedBroadOwner<'a> {
        self.broad
    }
    pub fn pipeline(self) -> &'a ValidatedPipelineState<'a> {
        self.pipeline
    }
}
