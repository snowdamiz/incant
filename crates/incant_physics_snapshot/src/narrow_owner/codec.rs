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
const MAGIC: &[u8; 8] = b"ICNARR01";
fn boolean(r: &mut Reader<'_>) -> Result<bool> {
    match r.word()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Header),
    }
}
pub fn decode_narrow_owner<'a>(
    bytes: &[u8],
    c: NarrowOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedNarrowOwner<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    let identity = c.rigid.record().identity;
    read_identity(&mut read, MAGIC, identity)?;
    let n = [
        read.word()? as usize,
        read.word()? as usize,
        read.word()? as usize,
    ];
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    context(identity, n, c)?;
    if bytes.len() != length(n)? {
        return Err(Error::Length);
    }
    let allocation = n
        .into_iter()
        .zip([
            size_of::<ColliderGraphIndices>(),
            size_of::<Handle>(),
            size_of::<IntersectionState>(),
        ])
        .try_fold(0usize, |sum, (n, size)| {
            n.checked_mul(size).and_then(|v| sum.checked_add(v))
        })
        .ok_or(Error::Limit)?;
    budget.charge(allocation)?;
    let mut indices = vector(n[0])?;
    let mut nodes = vector(n[1])?;
    let mut intersections = vector(n[2])?;
    for _ in 0..n[0] {
        indices.push(ColliderGraphIndices {
            contact: read.word()?,
            intersection: read.word()?,
        });
    }
    for _ in 0..n[1] {
        nodes.push(Handle {
            slot: read.word()?,
            generation: read.word()?,
        });
    }
    for _ in 0..n[2] {
        intersections.push(IntersectionState {
            intersecting: boolean(&mut read)?,
            start_event_emitted: boolean(&mut read)?,
        });
    }
    debug_assert_eq!(read.position, bytes.len());
    validate_narrow_owner(
        NarrowOwnerRecord {
            identity,
            indices,
            nodes,
            intersections,
        },
        c,
        budget,
    )
}
pub fn encode_narrow_owner(v: &ValidatedNarrowOwner<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let counts = counts(r);
    let size = length(counts)?;
    budget.charge(size)?;
    let mut bytes = vector(size)?;
    write_identity(&mut bytes, MAGIC, r.identity);
    let mut put = |n: u32| bytes.extend_from_slice(&n.to_le_bytes());
    for n in counts {
        put(n as u32);
    }
    put(0);
    for i in &r.indices {
        put(i.contact);
        put(i.intersection);
    }
    for h in &r.nodes {
        put(h.slot);
        put(h.generation);
    }
    for i in &r.intersections {
        put(u32::from(i.intersecting));
        put(u32::from(i.start_event_emitted));
    }
    debug_assert_eq!(bytes.len(), size);
    Ok(bytes)
}
