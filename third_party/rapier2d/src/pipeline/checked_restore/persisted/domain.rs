//! Additional admission for editable persisted data. No phase tag or build hash
//! authenticates history. These checks do not reconstruct, normalize or step.
use super::{DomainError as E, DomainResult as Result};
use crate::pipeline::checked_restore::{
    NumericFrame, WorkBudget, convert, derived_shape::DerivedShape,
};
use incant_physics_snapshot::{BodyKind, CcdCache, ColliderEnabled};
use parry::bounding_volume::BoundingVolume;
mod arithmetic;
mod contacts;

pub(super) fn admit(n: &NumericFrame<'_>, work: &mut WorkBudget) -> Result<()> {
    let f = n.boundary();
    let rigid = f.broad().context().narrow.context().rigid;
    let cs = rigid.collider_state();
    let bs = cs.context().bodies;
    let manifolds = f.manifolds().record();
    let tree = f.broad().context().tree.record();
    let rows = bs.record().bodies.len() as u64
        + cs.record().colliders.len() as u64
        + manifolds.points.len() as u64
        + manifolds.solver.len() as u64
        + manifolds.manifolds.len() as u64
        + tree.nodes.len() as u64
        + f.collider_arena().record().slots.len() as u64;
    work.charge(
        rows.checked_mul(32)
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Work)?,
    )?;
    let p = f.pipeline().context().integration;
    // Short TOI intervals and current solver clusters consume additional
    // unproved arithmetic/provenance. Refuse them, preserving every saved bit.
    if f.pipeline().record().post_solve_ccd_active == Some(true)
        || !matches!(
            f.pipeline().context().ccd.record().cache,
            CcdCache::Uninitialized
        )
        || bs
            .record()
            .bodies
            .iter()
            .any(|b| b.ccd.active || b.ccd.enabled || f32::from_bits(b.ccd.soft_prediction) != 0.)
    {
        return Err(E::Ccd);
    }
    if manifolds.pairs.iter().any(|p| p.clusters.count != 0) {
        return Err(E::Cluster);
    }
    // Native collision search can visit every leaf pair. Flat children multiply
    // the possible primitive pairs; charge before any narrow child walk. This
    // is a conservative logical first-update bound, not a device-time promise.
    let leaves = cs.context().geometries.iter().try_fold(0u64, |sum, g| {
        sum.checked_add(g.record().parts.len() as u64)
            .ok_or(E::Work)
    })?;
    work.charge(
        leaves
            .checked_mul(leaves.saturating_sub(1))
            .and_then(|n| n.checked_div(2))
            .ok_or(E::Work)?,
    )?;
    let prediction = f32::from_bits(p.prediction_distance());
    if !prediction.is_finite() {
        return Err(E::Arithmetic);
    }
    // Every retained enabled leaf must contain the actual current geometry and
    // skin/prediction bound. Pending queue membership alone does not prove it.
    for (slot, arena) in f.collider_arena().record().slots.iter().enumerate() {
        let incant_physics_snapshot::ArenaSlot::Occupied { row, .. } = *arena else {
            continue;
        };
        let row = row as usize;
        let c = &cs.record().colliders[row];
        if c.enabled != ColliderEnabled::Enabled {
            continue;
        }
        let packed = tree
            .leaves
            .slots
            .get(slot)
            .copied()
            .flatten()
            .ok_or(E::BroadBounds)?;
        let wide = tree.nodes.get(packed as usize / 2).ok_or(E::BroadBounds)?;
        let node = if packed & 1 == 0 {
            wide.left
        } else {
            wide.right
        };
        let shape = DerivedShape::new(&cs.context().geometries[row])?;
        let aabb = shape
            .compute_aabb(&convert::pose(c.position))
            .loosened(f32::from_bits(c.contact_skin) + prediction / 2.);
        let lo = convert::bits(aabb.mins);
        let hi = convert::bits(aabb.maxs);
        for axis in 0..crate::math::DIM {
            if !f32::from_bits(lo[axis]).is_finite()
                || !f32::from_bits(hi[axis]).is_finite()
                || f32::from_bits(node.mins[axis]) > f32::from_bits(lo[axis])
                || f32::from_bits(node.maxs[axis]) < f32::from_bits(hi[axis])
            {
                return Err(E::BroadBounds);
            }
        }
    }
    // Query-visible ancestors must already contain their children, including
    // pending paths. We retain flags/queue exactly; stale bounds refuse instead
    // of rebuilding a different tree on load. Change-bit propagation is left to
    // the native queue, but enabled children cannot hide under disabled parents.
    for wide in &tree.nodes {
        for node in [wide.left, wide.right] {
            if node.leaf_count() <= 1 {
                continue;
            }
            let children = tree
                .nodes
                .get(node.children as usize)
                .ok_or(E::BroadBounds)?;
            for child in [children.left, children.right] {
                if child.leaf_count() == 0 {
                    continue;
                }
                if child.flags() & 2 != 0 && node.flags() & 2 == 0 {
                    return Err(E::BroadBounds);
                }
                for axis in 0..crate::math::DIM {
                    if f32::from_bits(child.mins[axis]) < f32::from_bits(node.mins[axis])
                        || f32::from_bits(child.maxs[axis]) > f32::from_bits(node.maxs[axis])
                    {
                        return Err(E::BroadBounds);
                    }
                }
            }
        }
    }
    // Preserve sparse VecMap slots but bound their entire iteration span and
    // the cumulative counter after every possible first rigid pair unlink.
    let islands = rigid.islands().record();
    if islands.persistent.len() > incant_physics_snapshot::MAX_ITEMS {
        return Err(E::Counter);
    }
    work.charge(islands.persistent.len() as u64 + 1)?;
    let bodies = f.body_arena().record().slots.len() as u64;
    let colliders = f.collider_arena().record().slots.len() as u64;
    let extra = colliders
        .checked_mul(colliders.saturating_sub(1))
        .map(|n| n / 2)
        .and_then(|n| n.checked_mul(3))
        .and_then(|n| n.checked_add(bodies))
        .ok_or(E::Counter)?;
    let total = islands
        .persistent
        .iter()
        .flatten()
        .try_fold(extra, |sum, i| {
            sum.checked_add(u64::from(i.constraint_remove_count))
                .ok_or(E::Counter)
        })?;
    if total > u64::from(u32::MAX) {
        return Err(E::Counter);
    }
    contacts::admit(n)?;
    arithmetic::admit(n)?;
    Ok(())
}
fn dominance(b: Option<&incant_physics_snapshot::RigidBodyState>) -> i32 {
    match b {
        Some(b) if b.kind != BodyKind::Fixed => b.dominance,
        _ => 128,
    }
}
