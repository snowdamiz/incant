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
const MAGIC: &[u8; 8] = b"ICISLD01";
fn boolean(r: &mut Reader<'_>) -> Result<bool> {
    match r.word()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Header),
    }
}
fn optional(v: u32) -> Option<u32> {
    (v != u32::MAX).then_some(v)
}
fn span(r: &mut Reader<'_>) -> Result<RecordSpan> {
    Ok(RecordSpan {
        start: r.word()?,
        count: r.word()?,
    })
}
fn handle(r: &mut Reader<'_>) -> Result<Handle> {
    Ok(Handle {
        slot: r.word()?,
        generation: r.word()?,
    })
}
pub fn decode_islands<'a>(
    bytes: &[u8],
    c: IslandContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedIslands<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut rd = Reader { bytes, position: 0 };
    let identity = c.bodies.record().identity;
    read_identity(&mut rd, MAGIC, identity)?;
    context(identity, c)?;
    let active_epoch = rd.word()?;
    let awake = optional(rd.word()?);
    let sleep_scan_stamp = rd.word()?;
    let pending_split = optional(rd.word()?);
    let bootstrapped = boolean(&mut rd)?;
    let pending_removals = rd.word()?;
    let joint_links = rd.word()?;
    let joint_locations = rd.word()?;
    if !bootstrapped || joint_links != 0 || joint_locations != 0 {
        return Err(Error::UnsupportedIsland);
    }
    if pending_removals != 0 {
        return Err(Error::PendingMutation);
    }
    let active_live = rd.word()?;
    let persistent_live = rd.word()?;
    let mut n = [0; 8];
    for v in &mut n {
        *v = rd.word()? as usize;
    }
    if bytes.len() != length(n)? {
        return Err(Error::Length);
    }
    if active_live as usize > n[0]
        || n[1] != n[0] - active_live as usize
        || persistent_live as usize > n[3]
        || n[4] != n[3] - persistent_live as usize
        || n[2] > c.bodies.record().bodies.len()
        || n[5] > n[2]
        || n[6] > c.manifolds.record().pairs.len()
        || n[7] > c.manifolds.record().pairs.len()
    {
        return Err(Error::Length);
    }
    let sizes = [
        size_of::<Option<RecordSpan>>(),
        size_of::<u32>(),
        size_of::<Handle>(),
        size_of::<Option<PersistentIslandRow>>(),
        size_of::<u32>(),
        size_of::<Handle>(),
        size_of::<IslandContact>(),
        size_of::<Option<IslandLocation>>(),
    ];
    let allocation = n
        .into_iter()
        .zip(sizes)
        .try_fold(0usize, |sum, (n, size)| {
            n.checked_mul(size)
                .and_then(|v| sum.checked_add(v))
                .ok_or(Error::Limit)
        })?;
    budget.charge(allocation)?;
    let mut r = IslandTable {
        identity,
        active_epoch,
        awake,
        sleep_scan_stamp,
        pending_split,
        bootstrapped,
        pending_removals,
        joint_links,
        joint_locations,
        active_live,
        persistent_live,
        active: vector(n[0])?,
        active_free: vector(n[1])?,
        active_bodies: vector(n[2])?,
        persistent: vector(n[3])?,
        persistent_free: vector(n[4])?,
        persistent_bodies: vector(n[5])?,
        contacts: vector(n[6])?,
        locations: vector(n[7])?,
    };
    for _ in 0..n[0] {
        let present = boolean(&mut rd)?;
        let s = span(&mut rd)?;
        if !present && s != RecordSpan::default() {
            return Err(Error::Header);
        }
        r.active.push(present.then_some(s));
    }
    for _ in 0..n[1] {
        r.active_free.push(rd.word()?);
    }
    for _ in 0..n[2] {
        r.active_bodies.push(handle(&mut rd)?);
    }
    for _ in 0..n[3] {
        let present = boolean(&mut rd)?;
        let p = PersistentIslandRow {
            bodies: span(&mut rd)?,
            contacts: span(&mut rd)?,
            constraint_remove_count: rd.word()?,
            split_denied_until: rd.word()?,
            sleeping: boolean(&mut rd)?,
        };
        if !present
            && (p.bodies != RecordSpan::default()
                || p.contacts != RecordSpan::default()
                || p.constraint_remove_count != 0
                || p.split_denied_until != 0
                || p.sleeping)
        {
            return Err(Error::Header);
        }
        r.persistent.push(present.then_some(p));
    }
    for _ in 0..n[4] {
        r.persistent_free.push(rd.word()?);
    }
    for _ in 0..n[5] {
        r.persistent_bodies.push(handle(&mut rd)?);
    }
    for _ in 0..n[6] {
        r.contacts.push(IslandContact {
            edge: rd.word()?,
            bodies: [handle(&mut rd)?, handle(&mut rd)?],
        });
    }
    for _ in 0..n[7] {
        let island = rd.word()?;
        let index = rd.word()?;
        if (island == u32::MAX) != (index == u32::MAX) {
            return Err(Error::Header);
        }
        r.locations
            .push(optional(island).map(|island| IslandLocation { island, index }));
    }
    validate_islands(r, c, budget)
}
pub fn encode_islands(v: &ValidatedIslands<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let n = counts(r);
    let len = length(n)?;
    budget.charge(len)?;
    let mut out = vector(len)?;
    write_identity(&mut out, MAGIC, r.identity);
    let mut put = |v: u32| out.extend_from_slice(&v.to_le_bytes());
    for w in [
        r.active_epoch,
        r.awake.unwrap_or(u32::MAX),
        r.sleep_scan_stamp,
        r.pending_split.unwrap_or(u32::MAX),
        u32::from(r.bootstrapped),
        r.pending_removals,
        r.joint_links,
        r.joint_locations,
        r.active_live,
        r.persistent_live,
    ] {
        put(w);
    }
    for count in n {
        put(count as u32);
    }
    for s in &r.active {
        put(u32::from(s.is_some()));
        let s = s.unwrap_or_default();
        put(s.start);
        put(s.count);
    }
    for f in &r.active_free {
        put(*f);
    }
    for h in &r.active_bodies {
        put(h.slot);
        put(h.generation);
    }
    for p in &r.persistent {
        put(u32::from(p.is_some()));
        if let Some(p) = p {
            for w in [
                p.bodies.start,
                p.bodies.count,
                p.contacts.start,
                p.contacts.count,
                p.constraint_remove_count,
                p.split_denied_until,
                u32::from(p.sleeping),
            ] {
                put(w);
            }
        } else {
            for _ in 0..7 {
                put(0);
            }
        }
    }
    for f in &r.persistent_free {
        put(*f);
    }
    for h in &r.persistent_bodies {
        put(h.slot);
        put(h.generation);
    }
    for link in &r.contacts {
        put(link.edge);
        for h in link.bodies {
            put(h.slot);
            put(h.generation);
        }
    }
    for loc in &r.locations {
        put(loc.map(|l| l.island).unwrap_or(u32::MAX));
        put(loc.map(|l| l.index).unwrap_or(u32::MAX));
    }
    Ok(out)
}
