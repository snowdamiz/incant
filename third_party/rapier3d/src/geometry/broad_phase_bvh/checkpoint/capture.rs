//! Exact bounded broad owner observation. Map keys are canonical sorted records;
//! neighbor lists, insertion stamps, prior leaves and pending AABBs keep order.
use super::*;
use crate::pipeline::checked_restore::capture::util::{copy, count, handle as raw_handle, total};
use incant_physics_snapshot::{self as s, Budget, Error};
impl BroadPhaseBvh {
    pub(crate) fn checkpoint_capture(
        &self,
        identity: Identity,
        budget: &mut Budget,
    ) -> s::Result<(s::BvhRecord, s::CoarenaRecord, s::BroadOwnerRecord)> {
        if self.deferred_optimize_pending {
            return Err(Error::Broad);
        }
        count(self.pairs.len())?;
        let co = self.pair_adjacency.checkpoint_capture(identity, budget)?;
        let rows = self.pair_adjacency.checkpoint_rows();
        let n = total(rows.iter().map(|(_, v)| v.len()))?;
        let mut neighbors = budget.vector(n)?;
        let mut adjacency = budget.vector(rows.len())?;
        for (_, v) in rows {
            adjacency.push(s::RecordSpan {
                start: neighbors.len() as u32,
                count: v.len() as u32,
            });
            neighbors.extend(v.iter().map(|h| raw_handle(h.into_raw_parts())));
        }
        let mut pairs = budget.vector(self.pairs.len())?;
        pairs.extend(self.pairs.iter().map(|((a, b), frame)| s::BroadPair {
            colliders: [
                raw_handle(a.into_raw_parts()),
                raw_handle(b.into_raw_parts()),
            ],
            inserted_frame: *frame,
        }));
        pairs.sort_unstable_by_key(|p| {
            (
                p.colliders[0].slot,
                p.colliders[0].generation,
                p.colliders[1].slot,
                p.colliders[1].generation,
            )
        });
        let previous_leaves = copy(&self.prev_updated_leaves, budget)?;
        count(self.pending_set_aabb.len())?;
        let mut pending_aabbs = budget.vector(self.pending_set_aabb.len())?;
        pending_aabbs.extend(
            self.pending_set_aabb
                .iter()
                .map(|h| raw_handle(h.into_raw_parts())),
        );
        let tree = self.tree.checkpoint_capture(identity, budget)?;
        Ok((
            tree,
            co,
            s::BroadOwnerRecord {
                identity,
                pairs,
                adjacency,
                neighbors,
                previous_leaves,
                pending_aabbs,
                frame: self.frame_index,
                changes_since_optimize: self.changes_since_optimize,
                reinsert_leaf_updates: self.reinsert_leaf_updates,
                adaptive_margin: self.adaptive_change_detection_margin,
                optimization: match self.optimization_strategy {
                    BvhOptimizationStrategy::SubtreeOptimizer => s::BroadOptimization::Subtree,
                    BvhOptimizationStrategy::None => s::BroadOptimization::None,
                },
            },
        ))
    }
}
