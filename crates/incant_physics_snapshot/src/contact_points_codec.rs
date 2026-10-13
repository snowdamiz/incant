use crate::{
    Budget, ContactPointsRecord, ContactWarmstart, Error, Identity, MAX_BYTES, Result,
    SolverContactPoint, TrackedContactPoint, ValidatedContactPoints,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    contact_points::{contact_frame_length, contact_lengths},
    validate_contact_points, vector,
};
const MAGIC: &[u8; 8] = b"ICPTS001";
const HEADER: usize = 56;
fn words<const N: usize>(read: &mut Reader<'_>) -> Result<[u32; N]> {
    let mut result = [0; N];
    for value in &mut result {
        *value = read.word()?;
    }
    Ok(result)
}
pub fn decode_contact_points(
    bytes: &[u8],
    expected: Identity,
    budget: &mut Budget,
) -> Result<ValidatedContactPoints> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    read_identity(&mut read, MAGIC, expected)?;
    let p = read.word()? as usize;
    let s = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    contact_lengths(expected.dimension, p, s)?;
    if bytes.len() != contact_frame_length(p, s)? {
        return Err(Error::Length);
    }
    // Every count, identity and exact byte length is checked before allocation.
    budget.charge(
        p.checked_mul(size_of::<TrackedContactPoint>())
            .and_then(|v| {
                s.checked_mul(size_of::<SolverContactPoint>())
                    .and_then(|s| v.checked_add(s))
            })
            .ok_or(Error::Limit)?,
    )?;
    let mut record = ContactPointsRecord {
        identity: expected,
        points: vector(p)?,
        solver: vector(s)?,
    };
    for _ in 0..p {
        record.points.push(TrackedContactPoint {
            local_p1: words(&mut read)?,
            local_p2: words(&mut read)?,
            distance: read.word()?,
            features: words(&mut read)?,
            data: ContactWarmstart {
                impulse: read.word()?,
                tangent_impulse: words(&mut read)?,
                warmstart_impulse: read.word()?,
                warmstart_tangent_impulse: words(&mut read)?,
                warmstart_twist_impulse: read.word()?,
                warmstart_tangent_world: words(&mut read)?,
                solver_dp1: words(&mut read)?,
                solver_dp2: words(&mut read)?,
            },
        });
    }
    for _ in 0..s {
        record.solver.push(SolverContactPoint {
            anchor1: words(&mut read)?,
            anchor2: words(&mut read)?,
            distance: read.word()?,
            tangent_velocity: words(&mut read)?,
            contact_id: read.word()?,
            padding: read.word()?,
        });
    }
    validate_contact_points(record, budget)
}
pub fn encode_contact_points(
    value: &ValidatedContactPoints,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    let r = value.record();
    let len = contact_frame_length(r.points.len(), r.solver.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, r.identity);
    let mut put = |v: u32| bytes.extend_from_slice(&v.to_le_bytes());
    for v in [r.points.len() as u32, r.solver.len() as u32, 0] {
        put(v);
    }
    for p in &r.points {
        let w = p.data;
        for v in p
            .local_p1
            .into_iter()
            .chain(p.local_p2)
            .chain([p.distance])
            .chain(p.features)
            .chain([w.impulse])
            .chain(w.tangent_impulse)
            .chain([w.warmstart_impulse])
            .chain(w.warmstart_tangent_impulse)
            .chain([w.warmstart_twist_impulse])
            .chain(w.warmstart_tangent_world)
            .chain(w.solver_dp1)
            .chain(w.solver_dp2)
        {
            put(v);
        }
    }
    for s in &r.solver {
        for v in s
            .anchor1
            .into_iter()
            .chain(s.anchor2)
            .chain([s.distance])
            .chain(s.tangent_velocity)
            .chain([s.contact_id, s.padding])
        {
            put(v);
        }
    }
    Ok(bytes)
}
