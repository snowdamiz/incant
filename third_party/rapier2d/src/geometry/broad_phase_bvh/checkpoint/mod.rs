//! Exact private completed-owner staging. Shape/provenance/full install stay gated.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::{BroadOptimization, Handle, Identity, ValidatedBroadOwner};
mod map;
fn handle(h: Handle) -> ColliderHandle {
    ColliderHandle::from_raw_parts(h.slot, h.generation)
}
fn check(w: &ValidatedBroadOwner<'_>, expected: Identity) -> ConstructionResult<()> {
    if w.record().identity != expected || expected.dimension as usize != crate::math::DIM {
        return Err(ConstructionError::Payload);
    }
    Ok(())
}
impl BroadPhaseBvh {
    pub(crate) fn checkpoint_plan(
        w: &ValidatedBroadOwner<'_>,
        expected: Identity,
    ) -> ConstructionResult<AllocationPlan> {
        check(w, expected)?;
        let r = w.record();
        let c = w.context();
        let mut p = Bvh::checkpoint_pending_plan(c.tree, expected)?;
        p.append(map::plan(r.pairs.len())?)?;
        p.append(Coarena::<Vec<ColliderHandle>>::checkpoint_plan(
            c.adjacency,
        )?)?;
        for row in &r.adjacency {
            p.append(AllocationPlan::array::<ColliderHandle>(row.count as usize)?)?;
        }
        p.append(AllocationPlan::array::<u32>(r.previous_leaves.len())?)?;
        p.append(AllocationPlan::array::<ColliderHandle>(
            r.pending_aabbs.len(),
        )?)?;
        Ok(p)
    }
    pub(crate) fn from_checkpoint(
        w: &ValidatedBroadOwner<'_>,
        expected: Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        check(w, expected)?;
        let r = w.record();
        let c = w.context();
        let tree = Bvh::from_pending_checkpoint(c.tree, expected, permit)?;
        let pairs = map::construct(&r.pairs, permit)?;
        let pair_adjacency = Coarena::from_checkpoint(c.adjacency, permit, |row, permit| {
            let span = r.adjacency[row as usize];
            let mut out = permit.vector(span.count as usize)?;
            out.extend(
                r.neighbors[span.start as usize..(span.start + span.count) as usize]
                    .iter()
                    .copied()
                    .map(handle),
            );
            Ok(out)
        })?;
        let mut prev_updated_leaves = permit.vector(r.previous_leaves.len())?;
        prev_updated_leaves.extend_from_slice(&r.previous_leaves);
        let mut pending_set_aabb = permit.vector(r.pending_aabbs.len())?;
        pending_set_aabb.extend(r.pending_aabbs.iter().copied().map(handle));
        Ok(Self {
            tree,
            pairs,
            pair_adjacency,
            prev_updated_leaves,
            pending_set_aabb,
            changes_since_optimize: r.changes_since_optimize,
            reinsert_leaf_updates: r.reinsert_leaf_updates,
            frame_index: r.frame,
            optimization_strategy: match r.optimization {
                BroadOptimization::Subtree => BvhOptimizationStrategy::SubtreeOptimizer,
                BroadOptimization::None => BvhOptimizationStrategy::None,
            },
            adaptive_change_detection_margin: r.adaptive_margin,
            // Authenticated completion is the exclusive enclosing owner's duty.
            // The private NumericFrame entry joins an inert phase which already
            // refuses deferred/quarantine states. This storage helper does not
            // independently authenticate completion or enclosing provenance.
            deferred_optimize_pending: false,
            workspace: BvhWorkspace::default(),
            updated_colliders: Vec::new(),
            #[cfg(not(feature = "parallel"))]
            candidates_workspace: Vec::new(),
            updated_mask: Vec::new(),
            stale_pairs: Vec::new(),
            curr_updated_leaves: Vec::new(),
            update_workspace: Vec::new(),
            #[cfg(feature = "parallel")]
            update_batch_workspace: Vec::new(),
            #[cfg(feature = "parallel")]
            update_batch_statuses: Vec::new(),
        })
    }
}
#[cfg(test)]
mod map_tests;
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
mod tests;
