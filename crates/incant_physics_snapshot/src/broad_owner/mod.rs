//! Persistent broad-phase manager joined to exact tree and narrow owner witnesses.
//! Inert only: no native construction, phase authentication or save admission.
mod codec;
mod validate;
use crate::{
    Handle, Identity, RecordSpan, ValidatedCoarena, ValidatedNarrowOwner, ValidatedPendingBvh,
};
pub use codec::{decode_broad_owner, encode_broad_owner};
pub use validate::validate_broad_owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BroadPair {
    /// Canonical native map order, lower collider slot first.
    pub colliders: [Handle; 2],
    /// Historical insertion stamp; wrapping makes comparison to current frame invalid.
    pub inserted_frame: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BroadOptimization {
    Subtree = 0,
    None = 1,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BroadOwnerRecord {
    pub identity: Identity,
    pub pairs: Vec<BroadPair>,
    /// Coarena payload-row order, including empty invalidated/stale rows.
    pub adjacency: Vec<RecordSpan>,
    /// Native per-row order; deletions use swap_remove, never sort these lists.
    pub neighbors: Vec<Handle>,
    /// Historical slot sequence, including duplicates and removed leaves.
    pub previous_leaves: Vec<u32>,
    /// End-of-step set_aabb sequence. Nonempty is valid; duplicates retain order.
    pub pending_aabbs: Vec<Handle>,
    pub frame: u32,
    pub changes_since_optimize: u32,
    pub reinsert_leaf_updates: bool,
    pub adaptive_margin: bool,
    pub optimization: BroadOptimization,
}
#[derive(Clone, Copy)]
pub struct BroadOwnerContext<'a> {
    pub tree: &'a ValidatedPendingBvh<'a>,
    pub narrow: &'a ValidatedNarrowOwner<'a>,
    pub adjacency: &'a ValidatedCoarena<'a>,
}
pub struct ValidatedBroadOwner<'a> {
    record: BroadOwnerRecord,
    context: BroadOwnerContext<'a>,
}
impl<'a> ValidatedBroadOwner<'a> {
    pub fn record(&self) -> &BroadOwnerRecord {
        &self.record
    }
    pub fn context(&self) -> BroadOwnerContext<'a> {
        self.context
    }
    /// Exact live generation only; returned order is the retained native order.
    pub fn neighbors(&self, h: Handle) -> Option<&[Handle]> {
        self.context
            .narrow
            .context()
            .rigid
            .colliders()
            .collider(h)?;
        let row = self.context.adjacency.lookup(h)? as usize;
        let s = self.record.adjacency[row];
        Some(&self.record.neighbors[s.start as usize..(s.start + s.count) as usize])
    }
}
