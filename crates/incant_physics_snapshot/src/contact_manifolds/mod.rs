//! Current rigid manifold ownership, not complete narrow-phase or solver state.
mod codec;
mod validate;
use crate::{
    Handle, Identity, LocalPose, SolverContactPoint, TrackedContactPoint, ValidatedAttachments,
    ValidatedGeometry, ValidatedGraph,
};
pub use codec::{decode_manifolds, encode_manifolds};
pub use validate::validate_manifolds;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordSpan {
    pub start: u32,
    pub count: u32,
}
/// Independent caller-supplied phase witness. The eventual whole owner must
/// derive these counts from its checked journals, not trust a phase label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingJournals {
    pub modified_bodies: u32,
    pub modified_colliders: u32,
    pub removed_colliders: u32,
}
#[derive(Clone, Copy)]
pub struct ManifoldContext<'a> {
    pub graph: &'a ValidatedGraph,
    pub attachments: &'a ValidatedAttachments<'a>,
    /// One independently checked shape per collider arena payload row.
    pub geometries: &'a [ValidatedGeometry],
    pub journals: PendingJournals,
}

/// The two native endpoint handles and exact ordered manifold ranges for each
/// contact edge payload row. Pair events, colors, caches and dispatcher
/// workspaces are NOT represented by this ownership record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactPairManifolds {
    pub colliders: [Handle; 2],
    pub plain: RecordSpan,
    pub clusters: RecordSpan,
}

/// All fixed native ContactManifold / ContactManifoldData fields, plus spans
/// into shared ordered contact arrays. Only solver color/index/position scalar
/// domains are checked here; reciprocal solver-graph/island ownership is pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactManifoldRecord {
    pub points: RecordSpan,
    pub solver: RecordSpan,
    pub local_normals: [[u32; 3]; 2],
    pub subshapes: [u32; 2],
    /// Preserve native None vs allocated Some([None, None]) exactly.
    pub subshape_poses: Option<[Option<LocalPose>; 2]>,
    pub bodies: [Option<Handle>; 2],
    pub solver_flags: u32,
    pub solver_color: u32,
    pub solver_body_ids: [u32; 2],
    pub graph_position: u32,
    pub normal: [u32; 3],
    pub relative_dominance: i32,
    pub user_data: u32,
    pub friction: u32,
    pub restitution: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifoldTable {
    pub identity: Identity,
    /// Indexed by the checked graph's node payload rows, not arena slot.
    pub nodes: Vec<Handle>,
    /// Indexed by the checked graph's edge payload rows.
    pub pairs: Vec<ContactPairManifolds>,
    pub manifolds: Vec<ContactManifoldRecord>,
    pub points: Vec<TrackedContactPoint>,
    pub solver: Vec<SolverContactPoint>,
}
pub struct ValidatedManifolds<'a> {
    record: ManifoldTable,
    context: ManifoldContext<'a>,
}
impl ValidatedManifolds<'_> {
    pub(crate) fn pair_shapes(&self, row: usize) -> Option<[&ValidatedGeometry; 2]> {
        let p = self.record.pairs.get(row)?;
        let a = self.context.attachments.collider_row(p.colliders[0])? as usize;
        let b = self.context.attachments.collider_row(p.colliders[1])? as usize;
        Some([
            self.context.geometries.get(a)?,
            self.context.geometries.get(b)?,
        ])
    }
    pub fn record(&self) -> &ManifoldTable {
        &self.record
    }
    pub fn edge_pair(&self, edge: u32) -> Option<&ContactPairManifolds> {
        let row = self.context.graph.record().edges.get(edge as usize)?.row;
        self.record.pairs.get(row as usize)
    }
    pub fn manifold_points(&self, row: u32) -> Option<&[TrackedContactPoint]> {
        let s = self.record.manifolds.get(row as usize)?.points;
        self.record
            .points
            .get(s.start as usize..(s.start + s.count) as usize)
    }
    pub fn manifold_solver(&self, row: u32) -> Option<&[SolverContactPoint]> {
        let s = self.record.manifolds.get(row as usize)?.solver;
        self.record
            .solver
            .get(s.start as usize..(s.start + s.count) as usize)
    }
}
