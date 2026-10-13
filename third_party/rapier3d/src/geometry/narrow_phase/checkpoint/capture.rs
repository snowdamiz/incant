//! Complete currently supported rigid records, with exact ordered payloads.
//! Non-None dispatcher workspaces and soft pairs refuse; no approximation.
use super::*;
use crate::math::Rotation;
use crate::pipeline::checked_restore::capture::util::{
    copy, count, handle as raw_handle, map, total,
};
use crate::pipeline::checked_restore::convert::pose_bits;
use incant_physics_snapshot::{self as s, Budget, Error, Identity};
mod points;
pub(crate) struct CapturedNarrow {
    pub(crate) graph: s::GraphRecord,
    pub(crate) manifolds: s::ManifoldTable,
    pub(crate) pairs: s::PairStateTable,
    pub(crate) rigid: s::RigidOwnerRecord,
    pub(crate) indices: s::CoarenaRecord,
    pub(crate) intersections: s::GraphRecord,
    pub(crate) narrow: s::NarrowOwnerRecord,
}
fn rotation(r: Rotation) -> [u32; 4] {
    #[cfg(feature = "dim2")]
    {
        [r.re.to_bits(), r.im.to_bits(), 0, 0]
    }
    #[cfg(feature = "dim3")]
    {
        r.to_array().map(f32::to_bits)
    }
}
impl NarrowPhase {
    pub(crate) fn checkpoint_capture(
        &self,
        identity: Identity,
        budget: &mut Budget,
        scan: Vec<s::SleepScanSlot>,
        stamp: u32,
        touched: u32,
    ) -> s::Result<CapturedNarrow> {
        let raw = self.contact_graph.raw_graph();
        let graph = raw.checkpoint_capture(identity, budget)?;
        let nodes = map(raw.raw_nodes(), budget, |n| {
            raw_handle(n.weight.into_raw_parts())
        })?;
        let edges = raw.raw_edges();
        // First bound every outer list, then all inner totals before allocating.
        let mut manifold_count = 0usize;
        for e in edges {
            let p = e.weight.rigid().ok_or(Error::Workspace)?;
            if p.workspace.is_some() || p.soft.is_some() {
                return Err(Error::Workspace);
            }
            manifold_count =
                total([manifold_count, p.manifolds.len(), p.solver_clusters.len()].into_iter())?;
        }
        let manifolds_iter = || {
            edges
                .iter()
                .flat_map(|e| e.weight.rigid().into_iter())
                .flat_map(|p| p.manifolds.iter().chain(&p.solver_clusters))
        };
        let points_count = total(manifolds_iter().map(|m| m.points.len()))?;
        let solver_count = total(manifolds_iter().map(|m| m.data.solver_contacts.len()))?;
        let mut manifolds = s::ManifoldTable {
            identity,
            nodes,
            pairs: budget.vector(edges.len())?,
            manifolds: budget.vector(manifold_count)?,
            points: budget.vector(points_count)?,
            solver: budget.vector(solver_count)?,
        };
        let mut pairs = s::PairStateTable {
            identity,
            pairs: budget.vector(edges.len())?,
            detectors: Vec::new(),
        };
        for e in edges {
            let p = e.weight.rigid().ok_or(Error::Workspace)?;
            let plain = s::RecordSpan {
                start: manifolds.manifolds.len() as u32,
                count: p.manifolds.len() as u32,
            };
            for m in &p.manifolds {
                points::push(&mut manifolds, m);
            }
            let clusters = s::RecordSpan {
                start: manifolds.manifolds.len() as u32,
                count: p.solver_clusters.len() as u32,
            };
            for m in &p.solver_clusters {
                points::push(&mut manifolds, m);
            }
            manifolds.pairs.push(s::ContactPairManifolds {
                colliders: [
                    raw_handle(e.weight.collider1.into_raw_parts()),
                    raw_handle(e.weight.collider2.into_raw_parts()),
                ],
                plain,
                clusters,
            });
            pairs.pairs.push(s::PairState {
                event_status: u32::from(e.weight.event_status.bits()),
                solver_color: u32::from(p.solver_color),
                solver_color_bodies: p.solver_color_bodies,
                recycle: p.recycle_state.as_ref().map(|v| s::RecycleState {
                    relative_pose: pose_bits(v.pos12),
                    rotations: [rotation(v.rot1), rotation(v.rot2)],
                    max_extent: v.max_extent.to_bits(),
                    max_drift: v.max_drift.to_bits(),
                }),
                workspace: s::WorkspaceKind::None,
                timestamp: false,
                detectors: s::RecordSpan::default(),
            });
        }
        let (buckets, references) = self.solver_contact_graph.checkpoint_capture(budget)?;
        let rigid = s::RigidOwnerRecord {
            identity,
            sleep_scan_stamp: stamp,
            sleep_scan: scan,
            sleep_scan_touched: touched,
            body_color_masks: copy(&self.body_solver_color_masks, budget)?,
            pair_hints: map(&self.pair_solver_hints, budget, |x| u32::from(*x))?,
            buckets,
            references,
            graph_valid: self.solver_graph_valid,
            graph_epoch: self.solver_graph_epoch,
            graph_multibody_epoch: self.solver_graph_mb_epoch,
            force_valid: self.force_list_valid,
            force_pairs: copy(&self.force_event_pairs, budget)?,
            force_positions: copy(&self.force_event_pos, budget)?,
            force_flagged: copy(&self.force_event_flagged, budget)?,
        };
        let indices = self.graph_indices.checkpoint_capture(identity, budget)?;
        let raw = self.intersection_graph.raw_graph();
        let intersections = raw.checkpoint_capture(identity, budget)?;
        let narrow = s::NarrowOwnerRecord {
            identity,
            indices: map(self.graph_indices.checkpoint_rows(), budget, |(_, x)| {
                s::ColliderGraphIndices {
                    contact: x.contact_graph_index.index() as u32,
                    intersection: x.intersection_graph_index.index() as u32,
                }
            })?,
            nodes: map(raw.raw_nodes(), budget, |n| {
                raw_handle(n.weight.into_raw_parts())
            })?,
            intersections: map(raw.raw_edges(), budget, |e| s::IntersectionState {
                intersecting: e.weight.intersecting,
                start_event_emitted: e.weight.start_event_emitted,
            })?,
        };
        count(manifolds.manifolds.len())?;
        Ok(CapturedNarrow {
            graph,
            manifolds,
            pairs,
            rigid,
            indices,
            intersections,
            narrow,
        })
    }
}
