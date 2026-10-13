use super::validate::{HEADER, context_lengths, frame_length};
use super::*;
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    contact_points_codec::{read_point, read_solver, write_point, write_solver},
    vector,
};
const MAGIC: &[u8; 8] = b"ICMAN001";
fn words<const N: usize>(r: &mut Reader<'_>) -> Result<[u32; N]> {
    let mut v = [0; N];
    for x in &mut v {
        *x = r.word()?;
    }
    Ok(v)
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
fn optional_handle(r: &mut Reader<'_>) -> Result<Option<Handle>> {
    let tag = r.word()?;
    let h = handle(r)?;
    match tag {
        0 if h.slot == 0 && h.generation == 0 => Ok(None),
        1 => Ok(Some(h)),
        _ => Err(Error::Header),
    }
}
fn optional_pose(r: &mut Reader<'_>) -> Result<Option<LocalPose>> {
    let tag = r.word()?;
    let p = LocalPose {
        translation: words(r)?,
        rotation: words(r)?,
    };
    match tag {
        0 if p.translation == [0; 3] && p.rotation == [0; 4] => Ok(None),
        1 => Ok(Some(p)),
        _ => Err(Error::Header),
    }
}
fn read_manifold(r: &mut Reader<'_>) -> Result<ContactManifoldRecord> {
    let points = span(r)?;
    let solver = span(r)?;
    let local_normals = [words(r)?, words(r)?];
    let subshapes = words(r)?;
    let tag = r.word()?;
    let poses = [optional_pose(r)?, optional_pose(r)?];
    let subshape_poses = match tag {
        0 if poses == [None; 2] => None,
        1 => Some(poses),
        _ => return Err(Error::Header),
    };
    Ok(ContactManifoldRecord {
        points,
        solver,
        local_normals,
        subshapes,
        subshape_poses,
        bodies: [optional_handle(r)?, optional_handle(r)?],
        solver_flags: r.word()?,
        solver_color: r.word()?,
        solver_body_ids: words(r)?,
        graph_position: r.word()?,
        normal: words(r)?,
        relative_dominance: r.word()? as i32,
        user_data: r.word()?,
        friction: r.word()?,
        restitution: r.word()?,
    })
}
pub fn decode_manifolds<'a>(
    bytes: &[u8],
    context: ManifoldContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedManifolds<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let identity = context.graph.record().identity;
    let mut r = Reader { bytes, position: 0 };
    read_identity(&mut r, MAGIC, identity)?;
    let counts = words::<5>(&mut r)?.map(|v| v as usize);
    if r.word()? != 0 {
        return Err(Error::Header);
    }
    context_lengths(identity, counts, context)?;
    if bytes.len() != frame_length(counts)? {
        return Err(Error::Length);
    }
    let sizes = [
        size_of::<Handle>(),
        size_of::<ContactPairManifolds>(),
        size_of::<ContactManifoldRecord>(),
        size_of::<TrackedContactPoint>(),
        size_of::<SolverContactPoint>(),
    ];
    let allocation = counts
        .into_iter()
        .zip(sizes)
        .try_fold(0usize, |v, (n, s)| {
            n.checked_mul(s)
                .and_then(|n| v.checked_add(n))
                .ok_or(Error::Limit)
        })?;
    budget.charge(allocation)?;
    let [n, p, m, pt, s] = counts;
    let mut table = ManifoldTable {
        identity,
        nodes: vector(n)?,
        pairs: vector(p)?,
        manifolds: vector(m)?,
        points: vector(pt)?,
        solver: vector(s)?,
    };
    for _ in 0..n {
        table.nodes.push(handle(&mut r)?);
    }
    for _ in 0..p {
        table.pairs.push(ContactPairManifolds {
            colliders: [handle(&mut r)?, handle(&mut r)?],
            plain: span(&mut r)?,
            clusters: span(&mut r)?,
        });
    }
    for _ in 0..m {
        table.manifolds.push(read_manifold(&mut r)?);
    }
    for _ in 0..pt {
        table.points.push(read_point(&mut r)?);
    }
    for _ in 0..s {
        table.solver.push(read_solver(&mut r)?);
    }
    validate_manifolds(table, context, budget)
}
fn write_span(s: RecordSpan, put: &mut impl FnMut(u32)) {
    put(s.start);
    put(s.count);
}
fn write_handle(h: Handle, put: &mut impl FnMut(u32)) {
    put(h.slot);
    put(h.generation);
}
fn write_optional_handle(h: Option<Handle>, put: &mut impl FnMut(u32)) {
    put(u32::from(h.is_some()));
    write_handle(
        h.unwrap_or(Handle {
            slot: 0,
            generation: 0,
        }),
        put,
    );
}
fn write_optional_pose(p: Option<LocalPose>, put: &mut impl FnMut(u32)) {
    put(u32::from(p.is_some()));
    let p = p.unwrap_or(LocalPose {
        translation: [0; 3],
        rotation: [0; 4],
    });
    for v in p.translation.into_iter().chain(p.rotation) {
        put(v);
    }
}
fn write_manifold(m: &ContactManifoldRecord, put: &mut impl FnMut(u32)) {
    write_span(m.points, put);
    write_span(m.solver, put);
    for v in m.local_normals.into_iter().flatten().chain(m.subshapes) {
        put(v);
    }
    put(u32::from(m.subshape_poses.is_some()));
    for p in m.subshape_poses.unwrap_or([None; 2]) {
        write_optional_pose(p, put);
    }
    for h in m.bodies {
        write_optional_handle(h, put);
    }
    for v in [m.solver_flags, m.solver_color]
        .into_iter()
        .chain(m.solver_body_ids)
        .chain([m.graph_position])
        .chain(m.normal)
        .chain([
            m.relative_dominance as u32,
            m.user_data,
            m.friction,
            m.restitution,
        ])
    {
        put(v);
    }
}
pub fn encode_manifolds(value: &ValidatedManifolds<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = value.record();
    let counts = [
        r.nodes.len(),
        r.pairs.len(),
        r.manifolds.len(),
        r.points.len(),
        r.solver.len(),
    ];
    let length = frame_length(counts)?;
    budget.charge(length)?;
    let mut bytes = vector(length)?;
    write_identity(&mut bytes, MAGIC, r.identity);
    let mut put = |v: u32| bytes.extend_from_slice(&v.to_le_bytes());
    for n in counts {
        put(n as u32);
    }
    put(0);
    for h in &r.nodes {
        write_handle(*h, &mut put);
    }
    for p in &r.pairs {
        for h in p.colliders {
            write_handle(h, &mut put);
        }
        write_span(p.plain, &mut put);
        write_span(p.clusters, &mut put);
    }
    for m in &r.manifolds {
        write_manifold(m, &mut put);
    }
    for p in &r.points {
        write_point(p, &mut put);
    }
    for s in &r.solver {
        write_solver(s, &mut put);
    }
    Ok(bytes)
}
