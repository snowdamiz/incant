use super::{WorldBoundary, metadata, shapes};
use crate::{arena_codec::*, codec::Reader, *};
pub(super) const COUNT: usize = 22;
pub(super) const HEADER: usize = 48 + COUNT * 8;
const MAGIC: &[u8; 8] = b"ICWORLD1";

/// Canonical fixed section tags and lengths are checked before any allocation.
/// Exact section codecs remain authoritative; unknown, missing or duplicate
/// sections cannot be skipped. Total input and every output stay bounded.
pub(super) fn sections(bytes: &[u8], expected: Identity) -> Result<[&[u8]; COUNT]> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut reader = Reader { bytes, position: 0 };
    read_identity(&mut reader, MAGIC, expected)?;
    if reader.word()? as usize != COUNT {
        return Err(Error::Header);
    }
    let mut lengths = [0usize; COUNT];
    let mut total = HEADER;
    for (i, length) in lengths.iter_mut().enumerate() {
        if reader.word()? as usize != i {
            return Err(Error::Header);
        }
        *length = reader.word()? as usize;
        total = total
            .checked_add(*length)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or(Error::Limit)?;
    }
    if total != bytes.len() {
        return Err(Error::Length);
    }
    let mut result = [&[][..]; COUNT];
    let mut start = HEADER;
    for (slot, length) in result.iter_mut().zip(lengths) {
        *slot = &bytes[start..start + length];
        start += length;
    }
    Ok(result)
}

/// Uses one caller-owned budget for all section buffers and the final frame.
/// This budget measures cumulative requested Vec payload, not allocator/RSS.
pub fn encode_world_boundary(v: WorldBoundary<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let broad = v.broad();
    let bc = broad.context();
    let nc = bc.narrow.context();
    let rigid = nc.rigid;
    let colliders = rigid.collider_state();
    let bodies = colliders.context().bodies;
    let attachments = bodies.attachments();
    let manifolds = rigid.pairs().manifolds();
    let pc = v.pipeline().context();
    let chunks = [
        metadata::encode(v, budget)?,
        encode_arena(attachments.body_arena(), budget)?,
        encode_arena(attachments.collider_arena(), budget)?,
        encode_attachments(attachments, budget)?,
        shapes::encode(colliders.context().geometries, budget)?,
        encode_body_state(bodies, budget)?,
        encode_collider_state(colliders, budget)?,
        encode_graph(manifolds.graph(), budget)?,
        encode_manifolds(manifolds, budget)?,
        encode_pair_state(rigid.pairs(), budget)?,
        encode_islands(rigid.islands(), budget)?,
        encode_rigid_owner(rigid, budget)?,
        encode_coarena(nc.indices, budget)?,
        encode_graph(nc.intersections, budget)?,
        encode_narrow_owner(bc.narrow, budget)?,
        metadata::encode_pending(broad.record(), budget)?,
        encode_pending_bvh(bc.tree, budget)?,
        encode_coarena(bc.adjacency, budget)?,
        encode_broad_owner(broad, budget)?,
        encode_integration_config(pc.integration, budget)?,
        encode_ccd_state(pc.ccd, budget)?,
        encode_pipeline_state(v.pipeline(), budget)?,
    ];
    let total = chunks.iter().try_fold(HEADER, |sum, chunk| {
        sum.checked_add(chunk.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or(Error::Limit)
    })?;
    budget.charge(total)?;
    let mut bytes = vector(total)?;
    write_identity(&mut bytes, MAGIC, broad.record().identity);
    bytes.extend_from_slice(&(COUNT as u32).to_le_bytes());
    for (tag, chunk) in chunks.iter().enumerate() {
        bytes.extend_from_slice(&(tag as u32).to_le_bytes());
        bytes.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
    }
    for chunk in chunks {
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
