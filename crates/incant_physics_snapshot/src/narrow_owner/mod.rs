//! Persistent narrow-phase indices and sensor event history, joined to the
//! checked rigid owner. No live solver construction or dispatcher admission.
mod codec;
mod validate;
use crate::{Handle, Identity, ValidatedCoarena, ValidatedGraph, ValidatedRigidOwner};
pub use codec::{decode_narrow_owner, encode_narrow_owner};
pub use validate::validate_narrow_owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColliderGraphIndices {
    /// Native graph node index; MAX denotes no node, not payload row zero.
    pub contact: u32,
    pub intersection: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntersectionState {
    pub intersecting: bool,
    /// Historical emitted event, independent of current intersection/event mask.
    pub start_event_emitted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NarrowOwnerRecord {
    pub identity: Identity,
    /// Checked Coarena payload-row order, retaining invalidated/stale slots.
    pub indices: Vec<ColliderGraphIndices>,
    /// Checked intersection graph node payload-row order.
    pub nodes: Vec<Handle>,
    /// Checked intersection graph edge payload-row order.
    pub intersections: Vec<IntersectionState>,
}
#[derive(Clone, Copy)]
pub struct NarrowOwnerContext<'a> {
    pub rigid: &'a ValidatedRigidOwner<'a>,
    pub indices: &'a ValidatedCoarena<'a>,
    pub intersections: &'a ValidatedGraph,
}
pub struct ValidatedNarrowOwner<'a> {
    record: NarrowOwnerRecord,
    context: NarrowOwnerContext<'a>,
}
impl<'a> ValidatedNarrowOwner<'a> {
    pub fn record(&self) -> &NarrowOwnerRecord {
        &self.record
    }
    pub fn context(&self) -> NarrowOwnerContext<'a> {
        self.context
    }
    /// Requires a live exact generation; does not alias a replacement collider.
    pub fn collider_indices(&self, h: Handle) -> Option<ColliderGraphIndices> {
        self.context.rigid.colliders().collider(h)?;
        self.record
            .indices
            .get(self.context.indices.lookup(h)? as usize)
            .copied()
    }
    pub fn intersection(&self, edge: u32) -> Option<&IntersectionState> {
        let row = self
            .context
            .intersections
            .record()
            .edges
            .get(edge as usize)?
            .row;
        self.record.intersections.get(row as usize)
    }
}
