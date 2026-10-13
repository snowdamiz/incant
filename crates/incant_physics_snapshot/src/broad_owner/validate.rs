use super::*;
use crate::{
    ArenaSlot, Budget, ColliderEnabled, Error, MAX_ITEMS, Result, RowState,
    arena_codec::frame_length, vector,
};
pub(super) const HEADER: usize = 84;
pub(super) const STRIDES: [usize; 5] = [20, 8, 8, 4, 8];
pub(super) fn counts(r: &BroadOwnerRecord) -> [usize; 5] {
    [
        r.pairs.len(),
        r.adjacency.len(),
        r.neighbors.len(),
        r.previous_leaves.len(),
        r.pending_aabbs.len(),
    ]
}
pub(super) fn length(n: [usize; 5]) -> Result<usize> {
    if n.into_iter().any(|v| v > MAX_ITEMS) || n[0].checked_mul(2) != Some(n[2]) {
        return Err(Error::Length);
    }
    n.into_iter()
        .zip(STRIDES)
        .try_fold(HEADER, |sum, (n, s)| frame_length(sum, s, n))
}
pub(super) fn context(id: Identity, n: [usize; 5], c: BroadOwnerContext<'_>) -> Result<()> {
    let nc = c.narrow.context();
    if id != c.tree.record().identity
        || id != c.narrow.record().identity
        || id != c.adjacency.record().identity
        || !std::ptr::eq(c.adjacency.arena(), nc.indices.arena())
        || !std::ptr::eq(c.tree.context().colliders, nc.rigid.colliders())
    {
        return Err(Error::Identity);
    }
    let edges = nc
        .rigid
        .pairs()
        .manifolds()
        .record()
        .pairs
        .len()
        .checked_add(nc.intersections.record().edges.len())
        .ok_or(Error::Limit)?;
    if n[0] != edges || n[1] != c.adjacency.record().rows.len() {
        return Err(Error::Length);
    }
    length(n)?;
    Ok(())
}
fn key(h: [Handle; 2]) -> [(u32, u32); 2] {
    h.map(|h| (h.slot, h.generation))
}
fn find(r: &BroadOwnerRecord, mut h: [Handle; 2]) -> Result<usize> {
    if h[0].slot > h[1].slot {
        h.swap(0, 1);
    }
    r.pairs
        .binary_search_by_key(&key(h), |p| key(p.colliders))
        .map_err(|_| Error::Broad)
}
pub fn validate_broad_owner<'a>(
    r: BroadOwnerRecord,
    c: BroadOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedBroadOwner<'a>> {
    context(r.identity, counts(&r), c)?;
    if r.pending_aabbs != c.tree.context().pending_aabbs {
        return Err(Error::Ownership);
    }
    let colliders = c.narrow.context().rigid.colliders();
    let arena = c.adjacency.arena();
    let leaves = &c.tree.record().leaves.slots;
    if leaves.len() > arena.record().slots.len() {
        return Err(Error::BroadTree);
    }
    for (slot, body) in arena.record().slots.iter().enumerate() {
        let enabled = match *body {
            ArenaSlot::Occupied { generation, .. } => {
                colliders
                    .collider(Handle {
                        slot: slot as u32,
                        generation,
                    })
                    .unwrap()
                    .enabled
                    == ColliderEnabled::Enabled
            }
            ArenaSlot::Free { .. } => false,
        };
        if leaves.get(slot).is_some_and(Option::is_some) != enabled {
            return Err(Error::BroadTree);
        }
    }
    let live = |h: Handle| -> Result<()> {
        let co = colliders.collider(h).ok_or(Error::Generation)?;
        if co.enabled != ColliderEnabled::Enabled {
            return Err(Error::Broad);
        }
        Ok(())
    };
    let mut previous = None;
    for p in &r.pairs {
        if p.colliders[0].slot >= p.colliders[1].slot
            || previous.is_some_and(|k| k >= key(p.colliders))
        {
            return Err(Error::Broad);
        }
        for h in p.colliders {
            live(h)?;
        }
        previous = Some(key(p.colliders));
    }
    // Canonical spans cover all neighbor storage in payload order, independently
    // of physical coarena indices. Pair reciprocity does not reorder any payload.
    let mut end = 0usize;
    for s in &r.adjacency {
        if s.start as usize != end {
            return Err(Error::Ownership);
        }
        end = end.checked_add(s.count as usize).ok_or(Error::Limit)?;
        if end > r.neighbors.len() {
            return Err(Error::Length);
        }
    }
    if end != r.neighbors.len() {
        return Err(Error::Length);
    }
    budget.charge(r.pairs.len())?;
    let mut seen = vector::<u8>(r.pairs.len())?;
    seen.resize(r.pairs.len(), 0);
    for (slot, co) in c.adjacency.record().rows.iter().enumerate() {
        let s = r.adjacency[co.row as usize];
        if s.count == 0 {
            continue;
        }
        if c.adjacency.state(slot as u32) != Some(RowState::Current) {
            return Err(Error::Generation);
        }
        let h = Handle {
            slot: slot as u32,
            generation: co.generation,
        };
        live(h)?;
        for other in &r.neighbors[s.start as usize..(s.start + s.count) as usize] {
            live(*other)?;
            let index = find(&r, [h, *other])?;
            let bit = if h.slot < other.slot { 1 } else { 2 };
            if seen[index] & bit != 0 {
                return Err(Error::Broad);
            }
            seen[index] |= bit;
        }
    }
    if seen.iter().any(|v| *v != 3) {
        return Err(Error::Broad);
    }
    // Actual completed native controls establish that the pair map is exactly
    // the union of contact and sensor edges, including filtered inactive edges.
    seen.fill(0);
    let mut visit = |h: [Handle; 2]| -> Result<()> {
        let i = find(&r, h)?;
        if seen[i] != 0 {
            return Err(Error::Broad);
        }
        seen[i] = 1;
        Ok(())
    };
    for p in &c.narrow.context().rigid.pairs().manifolds().record().pairs {
        visit(p.colliders)?;
    }
    let g = c.narrow.context().intersections.record();
    for e in &g.edges {
        visit(
            e.node
                .map(|n| c.narrow.record().nodes[g.nodes[n as usize].row as usize]),
        )?;
    }
    if seen.iter().any(|v| *v != 1) {
        return Err(Error::Broad);
    }
    for slot in &r.previous_leaves {
        if *slot as usize >= arena.record().slots.len() {
            return Err(Error::BroadTree);
        }
    }
    for h in &r.pending_aabbs {
        live(*h)?;
    }
    // No present AABB/filter equivalence: set_aabb can move geometry after the
    // last pair scan. No numeric stamp comparison across wrapping frames.
    Ok(ValidatedBroadOwner {
        record: r,
        context: c,
    })
}
