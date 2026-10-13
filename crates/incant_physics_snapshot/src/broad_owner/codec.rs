use super::{
    validate::{HEADER, context, counts, length},
    *,
};
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    vector,
};
const MAGIC: &[u8; 8] = b"ICBROAD1";
fn handle(r: &mut Reader<'_>) -> Result<Handle> {
    Ok(Handle {
        slot: r.word()?,
        generation: r.word()?,
    })
}
pub fn decode_broad_owner<'a>(
    bytes: &[u8],
    c: BroadOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedBroadOwner<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let identity = c.narrow.record().identity;
    read_identity(&mut r, MAGIC, identity)?;
    let mut n = [0usize; 5];
    for v in &mut n {
        *v = r.word()? as usize;
    }
    let frame = r.word()?;
    let changes_since_optimize = r.word()?;
    let flags = r.word()?;
    let optimization = match r.word()? {
        0 => BroadOptimization::Subtree,
        1 => BroadOptimization::None,
        _ => return Err(Error::Header),
    };
    if flags & !3 != 0 || r.word()? != 0 {
        return Err(Error::Header);
    }
    context(identity, n, c)?;
    if bytes.len() != length(n)? {
        return Err(Error::Length);
    }
    let allocation = n
        .into_iter()
        .zip([
            size_of::<BroadPair>(),
            size_of::<RecordSpan>(),
            size_of::<Handle>(),
            size_of::<u32>(),
            size_of::<Handle>(),
        ])
        .try_fold(0usize, |sum, (n, s)| {
            n.checked_mul(s).and_then(|v| sum.checked_add(v))
        })
        .ok_or(Error::Limit)?;
    budget.charge(allocation)?;
    let mut pairs = vector(n[0])?;
    let mut adjacency = vector(n[1])?;
    let mut neighbors = vector(n[2])?;
    let mut previous_leaves = vector(n[3])?;
    let mut pending_aabbs = vector(n[4])?;
    for _ in 0..n[0] {
        pairs.push(BroadPair {
            colliders: [handle(&mut r)?, handle(&mut r)?],
            inserted_frame: r.word()?,
        });
    }
    for _ in 0..n[1] {
        adjacency.push(RecordSpan {
            start: r.word()?,
            count: r.word()?,
        });
    }
    for _ in 0..n[2] {
        neighbors.push(handle(&mut r)?);
    }
    for _ in 0..n[3] {
        previous_leaves.push(r.word()?);
    }
    for _ in 0..n[4] {
        pending_aabbs.push(handle(&mut r)?);
    }
    validate_broad_owner(
        BroadOwnerRecord {
            identity,
            pairs,
            adjacency,
            neighbors,
            previous_leaves,
            pending_aabbs,
            frame,
            changes_since_optimize,
            reinsert_leaf_updates: flags & 1 != 0,
            adaptive_margin: flags & 2 != 0,
            optimization,
        },
        c,
        budget,
    )
}
pub fn encode_broad_owner(v: &ValidatedBroadOwner<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let n = counts(r);
    let size = length(n)?;
    budget.charge(size)?;
    let mut b = vector(size)?;
    write_identity(&mut b, MAGIC, r.identity);
    let mut put = |n: u32| b.extend_from_slice(&n.to_le_bytes());
    for n in n {
        put(n as u32);
    }
    put(r.frame);
    put(r.changes_since_optimize);
    put(u32::from(r.reinsert_leaf_updates) | (u32::from(r.adaptive_margin) << 1));
    put(r.optimization as u32);
    put(0);
    for p in &r.pairs {
        for h in p.colliders {
            put(h.slot);
            put(h.generation);
        }
        put(p.inserted_frame);
    }
    for s in &r.adjacency {
        put(s.start);
        put(s.count);
    }
    for h in &r.neighbors {
        put(h.slot);
        put(h.generation);
    }
    for slot in &r.previous_leaves {
        put(*slot);
    }
    for h in &r.pending_aabbs {
        put(h.slot);
        put(h.generation);
    }
    debug_assert_eq!(b.len(), size);
    Ok(b)
}
