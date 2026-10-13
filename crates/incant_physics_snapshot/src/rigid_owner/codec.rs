use super::validate::{HEADER, context, counts, length};
use super::*;
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    vector,
};
const MAGIC: &[u8; 8] = b"ICROWR01";
fn boolean(rd: &mut Reader<'_>) -> Result<bool> {
    match rd.word()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Header),
    }
}
pub fn decode_rigid_owner<'a>(
    bytes: &[u8],
    c: RigidOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedRigidOwner<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut rd = Reader { bytes, position: 0 };
    let identity = c.islands.record().identity;
    read_identity(&mut rd, MAGIC, identity)?;
    context(identity, c)?;
    let sleep_scan_stamp = rd.word()?;
    let sleep_scan_touched = rd.word()?;
    let graph_valid = boolean(&mut rd)?;
    let graph_epoch = rd.word()?;
    let graph_multibody_epoch = rd.word()?;
    let force_valid = boolean(&mut rd)?;
    let mut n = [0usize; 8];
    for v in &mut n {
        *v = rd.word()? as usize;
    }
    if bytes.len() != length(n)?
        || n[2] != c.pairs.record().pairs.len()
        || n[6] != n[2]
        || n[5] > n[2]
    {
        return Err(Error::Length);
    }
    let allocation = n
        .into_iter()
        .zip([
            size_of::<SleepScanSlot>(),
            size_of::<u128>(),
            4,
            8,
            8,
            4,
            4,
            4,
        ])
        .try_fold(0usize, |s, (n, size)| {
            s.checked_add(n.checked_mul(size).ok_or(Error::Limit)?)
                .ok_or(Error::Limit)
        })?;
    budget.charge(allocation)?;
    let mut r = RigidOwnerRecord {
        identity,
        sleep_scan_stamp,
        sleep_scan_touched,
        graph_valid,
        graph_epoch,
        graph_multibody_epoch,
        force_valid,
        sleep_scan: vector(n[0])?,
        body_color_masks: vector(n[1])?,
        pair_hints: vector(n[2])?,
        buckets: vector(n[3])?,
        references: vector(n[4])?,
        force_pairs: vector(n[5])?,
        force_positions: vector(n[6])?,
        force_flagged: vector(n[7])?,
    };
    for _ in 0..n[0] {
        r.sleep_scan.push(SleepScanSlot {
            stamp: rd.word()?,
            eligible: boolean(&mut rd)?,
        });
    }
    for _ in 0..n[1] {
        r.body_color_masks.push(u128::from_le_bytes(rd.take()?));
    }
    for _ in 0..n[2] {
        r.pair_hints.push(rd.word()?);
    }
    for _ in 0..n[3] {
        r.buckets.push(RecordSpan {
            start: rd.word()?,
            count: rd.word()?,
        });
    }
    for _ in 0..n[4] {
        r.references.push(SolverReference {
            edge: rd.word()?,
            manifold: rd.word()?,
        });
    }
    for _ in 0..n[5] {
        r.force_pairs.push(rd.word()?);
    }
    for _ in 0..n[6] {
        r.force_positions.push(rd.word()?);
    }
    for _ in 0..n[7] {
        r.force_flagged.push(rd.word()?);
    }
    validate_rigid_owner(r, c, budget)
}
pub fn encode_rigid_owner(v: &ValidatedRigidOwner<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let n = counts(r);
    let len = length(n)?;
    budget.charge(len)?;
    let mut out = vector(len)?;
    write_identity(&mut out, MAGIC, r.identity);
    for v in [
        r.sleep_scan_stamp,
        r.sleep_scan_touched,
        r.graph_valid as u32,
        r.graph_epoch,
        r.graph_multibody_epoch,
        r.force_valid as u32,
    ]
    .into_iter()
    .chain(n.map(|n| n as u32))
    {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for s in &r.sleep_scan {
        out.extend_from_slice(&s.stamp.to_le_bytes());
        out.extend_from_slice(&(s.eligible as u32).to_le_bytes());
    }
    for v in &r.body_color_masks {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &r.pair_hints {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &r.buckets {
        out.extend_from_slice(&v.start.to_le_bytes());
        out.extend_from_slice(&v.count.to_le_bytes());
    }
    for v in &r.references {
        out.extend_from_slice(&v.edge.to_le_bytes());
        out.extend_from_slice(&v.manifold.to_le_bytes());
    }
    for values in [&r.force_pairs, &r.force_positions, &r.force_flagged] {
        for v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    Ok(out)
}
