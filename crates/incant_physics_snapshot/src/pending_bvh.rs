//! Exact end-of-step tree bytes with narrowly named, unresolved leaf updates.
//! This is NOT a query-ready ValidatedBvh and has no live native conversion.
use crate::{
    Budget, BvhRecord, ColliderEnabled, Error, Handle, MAX_DEPTH, MAX_ITEMS, PendingJournals,
    Result, ValidatedColliderState, records::lengths, vector,
};
#[derive(Clone, Copy)]
pub struct PendingBvhContext<'a> {
    pub colliders: &'a ValidatedColliderState<'a>,
    pub pending_aabbs: &'a [Handle],
}
pub struct ValidatedPendingBvh<'a> {
    record: BvhRecord,
    context: PendingBvhContext<'a>,
}
impl<'a> ValidatedPendingBvh<'a> {
    pub fn record(&self) -> &BvhRecord {
        &self.record
    }
    pub fn context(&self) -> PendingBvhContext<'a> {
        self.context
    }
}
pub fn validate_pending_bvh<'a>(
    r: BvhRecord,
    c: PendingBvhContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedPendingBvh<'a>> {
    if r.identity != c.colliders.record().identity {
        return Err(Error::Identity);
    }
    if c.colliders.context().journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    lengths(
        r.nodes.len(),
        r.parents.len(),
        r.leaves.slots.len(),
        r.leaves.occupied,
        r.free.len(),
    )?;
    if c.pending_aabbs.len() > MAX_ITEMS {
        return Err(Error::Limit);
    }
    let size = r.nodes.len().checked_mul(2).ok_or(Error::Limit)?;
    budget.charge(size)?;
    let mut paths = vector::<u8>(size)?;
    paths.resize(size, 0);
    for h in c.pending_aabbs {
        let co = c.colliders.collider(*h).ok_or(Error::Generation)?;
        if co.enabled != ColliderEnabled::Enabled {
            return Err(Error::BroadTree);
        }
        let packed = r
            .leaves
            .slots
            .get(h.slot as usize)
            .copied()
            .flatten()
            .ok_or(Error::BroadTree)?;
        let wide = r.nodes.get(packed as usize / 2).ok_or(Error::Ownership)?;
        let leaf = if packed & 1 == 0 {
            wide.left
        } else {
            wide.right
        };
        if leaf.leaf_count() != 1 || leaf.children != h.slot || leaf.flags() != 3 {
            return Err(Error::Metadata);
        }
        paths[packed as usize] |= 2;
        let mut current = packed;
        for depth in 0..MAX_DEPTH {
            paths[current as usize] |= 1;
            if current / 2 == 0 {
                break;
            }
            if depth + 1 == MAX_DEPTH {
                return Err(Error::Depth);
            }
            current = r.parents[current as usize / 2];
            if current as usize >= size {
                return Err(Error::Parent);
            }
        }
    }
    // Every topology/count/finite/free/ownership property stays strict. Only
    // flags and containment along the named ancestor closure may be deferred by
    // native set_aabb (which propagates unpadded bounds and no ancestor flags).
    crate::validate::structure(&r, budget, Some(&paths))?;
    Ok(ValidatedPendingBvh {
        record: r,
        context: c,
    })
}
/// Same inert tree encoding, separately checked against the exact pending queue.
/// The strict decode entry point continues to reject unresolved trees.
pub fn decode_pending_bvh<'a>(
    bytes: &[u8],
    c: PendingBvhContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedPendingBvh<'a>> {
    let r = crate::codec::decode_record(bytes, c.colliders.record().identity, budget)?;
    validate_pending_bvh(r, c, budget)
}
pub fn encode_pending_bvh(v: &ValidatedPendingBvh<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    crate::codec::encode_record(v.record(), budget)
}
