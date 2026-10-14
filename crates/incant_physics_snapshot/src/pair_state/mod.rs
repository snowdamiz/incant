//! Persistent rigid-pair state, with closed supported Parry workspaces. Inert only.
mod codec;
mod validate;
use crate::{Identity, LocalPose, RecordSpan, ValidatedManifolds};
pub use codec::{decode_pair_state, encode_pair_state};
pub use validate::validate_pair_state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecycleState {
    pub relative_pose: LocalPose,
    pub rotations: [[u32; 4]; 2],
    pub max_extent: u32,
    pub max_drift: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum WorkspaceKind {
    None = 0,
    CompositeShape = 1,
    CompositeComposite = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairState {
    pub event_status: u32,
    pub solver_color: u32,
    /// Native body-mask slots; reciprocal masks are checked by the joined owner.
    pub solver_color_bodies: [u32; 2],
    pub recycle: Option<RecycleState>,
    pub workspace: WorkspaceKind,
    pub timestamp: bool,
    pub detectors: RecordSpan,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubDetector {
    /// Native map key. Single-composite workspaces use word zero and require
    /// word one to be zero, regardless of which collider is the composite.
    pub key: [u32; 2],
    /// Index in this pair's ordinary (not solver-cluster) manifold array.
    pub manifold: u32,
    pub timestamp: bool,
}
/// A captured closed native workspace before joining its detector indices to
/// the independently captured manifold table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub kind: WorkspaceKind,
    pub timestamp: bool,
    pub detectors: Vec<SubDetector>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairStateTable {
    pub identity: Identity,
    /// Same payload row order as the checked contact graph/manifold table.
    pub pairs: Vec<PairState>,
    /// Deterministic builds preserve native insertion order. Fast hash-map
    /// builds capture canonical key order: their bucket layout is not semantic
    /// state, and contact order comes from the preserved BVH traversal.
    pub detectors: Vec<SubDetector>,
}
pub struct ValidatedPairState<'a> {
    record: PairStateTable,
    manifolds: &'a ValidatedManifolds<'a>,
}
impl<'a> ValidatedPairState<'a> {
    pub(crate) fn manifolds(&self) -> &'a ValidatedManifolds<'a> {
        self.manifolds
    }

    pub fn record(&self) -> &PairStateTable {
        &self.record
    }
    pub fn detectors(&self, pair: u32) -> Option<&[SubDetector]> {
        let s = self.record.pairs.get(pair as usize)?.detectors;
        self.record
            .detectors
            .get(s.start as usize..(s.start + s.count) as usize)
    }
    pub fn detector_manifold(
        &self,
        pair: u32,
        detector: u32,
    ) -> Option<&crate::ContactManifoldRecord> {
        let d = self.detectors(pair)?.get(detector as usize)?;
        let p = self.manifolds.record().pairs.get(pair as usize)?;
        self.manifolds
            .record()
            .manifolds
            .get((p.plain.start + d.manifold) as usize)
    }
}
