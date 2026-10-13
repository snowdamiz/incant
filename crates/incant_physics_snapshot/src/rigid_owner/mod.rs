//! Joined rigid owner state: body/island/pair witnesses plus persistent solver
//! caches and stamp-sensitive scratch. Still inert: derived arithmetic and the
//! broad-phase owner must be closed before any hostile live restore admission.
mod codec;
mod validate;
use crate::{Identity, RecordSpan, ValidatedColliderState, ValidatedIslands, ValidatedPairState};
pub use codec::{decode_rigid_owner, encode_rigid_owner};
pub use validate::validate_rigid_owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SleepScanSlot {
    pub stamp: u32,
    pub eligible: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolverReference {
    pub edge: u32,
    pub manifold: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RigidOwnerRecord {
    pub identity: Identity,
    pub sleep_scan_stamp: u32,
    /// Exact historical slots, including vacant tails. Never zero/compact them.
    pub sleep_scan: Vec<SleepScanSlot>,
    pub sleep_scan_touched: u32,
    pub body_color_masks: Vec<u128>,
    pub pair_hints: Vec<u32>,
    pub buckets: Vec<RecordSpan>,
    pub references: Vec<SolverReference>,
    pub graph_valid: bool,
    pub graph_epoch: u32,
    pub graph_multibody_epoch: u32,
    pub force_valid: bool,
    pub force_pairs: Vec<u32>,
    pub force_positions: Vec<u32>,
    pub force_flagged: Vec<u32>,
}
/// Exact checked body/collider/shape ownership shared with the solver witnesses.
#[derive(Clone, Copy)]
pub struct RigidOwnerContext<'a> {
    pub islands: &'a ValidatedIslands<'a>,
    pub pairs: &'a ValidatedPairState<'a>,
    pub colliders: &'a ValidatedColliderState<'a>,
    pub multibody_epoch: u32,
}
pub struct ValidatedRigidOwner<'a> {
    record: RigidOwnerRecord,
    context: RigidOwnerContext<'a>,
}
impl<'a> ValidatedRigidOwner<'a> {
    pub(crate) fn colliders(&self) -> &'a ValidatedColliderState<'a> {
        self.context.colliders
    }
    pub fn collider_state(&self) -> &'a ValidatedColliderState<'a> {
        self.context.colliders
    }
    pub fn record(&self) -> &RigidOwnerRecord {
        &self.record
    }
    pub fn islands(&self) -> &'a ValidatedIslands<'a> {
        self.context.islands
    }
    pub fn pairs(&self) -> &'a ValidatedPairState<'a> {
        self.context.pairs
    }
    /// Stale native caches are preserved and must be rebuilt before solver use.
    pub fn requires_graph_rebuild(&self) -> bool {
        !self.record.graph_valid
            || self.record.graph_epoch != self.context.islands.record().active_epoch
            || self.record.graph_multibody_epoch != self.context.multibody_epoch
    }
}
