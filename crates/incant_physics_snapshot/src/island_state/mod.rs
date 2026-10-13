//! Completed-step rigid island ownership; no live construction or scratch-reset proof.
mod codec;
mod validate;
use crate::{Handle, Identity, RecordSpan, ValidatedBodyState, ValidatedManifolds};
pub use codec::{decode_islands, encode_islands};
pub use validate::validate_islands;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentIslandRow {
    pub bodies: RecordSpan,
    pub contacts: RecordSpan,
    /// May be nonzero at a completed step: native splitting is deferred.
    pub constraint_remove_count: u32,
    pub split_denied_until: u32,
    pub sleeping: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandContact {
    pub edge: u32,
    /// Missing parent is the native (MAX, MAX) handle; fixed parents stay real.
    pub bodies: [Handle; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandLocation {
    pub island: u32,
    pub index: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IslandTable {
    pub identity: Identity,
    pub active_epoch: u32,
    pub awake: Option<u32>,
    pub sleep_scan_stamp: u32,
    pub pending_split: Option<u32>,
    pub bootstrapped: bool,
    /// Unsupported payloads are explicit, never silently represented as empty.
    pub pending_removals: u32,
    pub joint_links: u32,
    pub joint_locations: u32,
    pub active_live: u32,
    pub persistent_live: u32,
    /// Native VecMap slot order, including trailing empty slots, and exact counts.
    pub active: Vec<Option<RecordSpan>>,
    pub active_free: Vec<u32>,
    pub active_bodies: Vec<Handle>,
    pub persistent: Vec<Option<PersistentIslandRow>>,
    pub persistent_free: Vec<u32>,
    pub persistent_bodies: Vec<Handle>,
    pub contacts: Vec<IslandContact>,
    /// Native vector can end before the final contact edge; missing tail is unlinked.
    pub locations: Vec<Option<IslandLocation>>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IslandDomain {
    /// Counts must ultimately come from the checked enclosing owner.
    pub impulse_joints: u32,
    pub multibody_joints: u32,
    pub soft_bodies: u32,
}
#[derive(Clone, Copy)]
pub struct IslandContext<'a> {
    pub bodies: &'a ValidatedBodyState<'a>,
    pub manifolds: &'a ValidatedManifolds<'a>,
    pub domain: IslandDomain,
}
pub struct ValidatedIslands<'a> {
    record: IslandTable,
    context: IslandContext<'a>,
}
impl<'a> ValidatedIslands<'a> {
    pub(crate) fn context(&self) -> IslandContext<'a> {
        self.context
    }

    pub fn record(&self) -> &IslandTable {
        &self.record
    }
    /// Outer None is a stale body handle. Each native membership is independent:
    /// a body changed to Fixed can retain active membership but no persistent one.
    pub fn body_membership(&self, h: Handle) -> Option<[Option<IslandLocation>; 2]> {
        let b = self.context.bodies.body(h)?;
        Some([0, 2].map(|i| {
            (b.ids[i] != u32::MAX).then_some(IslandLocation {
                island: b.ids[i],
                index: b.ids[i + 1],
            })
        }))
    }
    pub fn contact_location(&self, edge: u32) -> Option<IslandLocation> {
        self.record.locations.get(edge as usize).copied().flatten()
    }
}
