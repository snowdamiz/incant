use super::validate::{HEADER, context, length};
use super::*;
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    vector,
};
const MAGIC: &[u8; 8] = b"ICPAIR01";
fn words<const N: usize>(r: &mut Reader<'_>) -> Result<[u32; N]> {
    let mut a = [0; N];
    for v in &mut a {
        *v = r.word()?;
    }
    Ok(a)
}
fn boolean(r: &mut Reader<'_>) -> Result<bool> {
    match r.word()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Header),
    }
}
fn read_recycle(r: &mut Reader<'_>) -> Result<Option<RecycleState>> {
    let present = boolean(r)?;
    let s = RecycleState {
        relative_pose: LocalPose {
            translation: words(r)?,
            rotation: words(r)?,
        },
        rotations: [words(r)?, words(r)?],
        max_extent: r.word()?,
        max_drift: r.word()?,
    };
    if present {
        return Ok(Some(s));
    }
    if s.relative_pose.translation != [0; 3]
        || s.relative_pose.rotation != [0; 4]
        || s.rotations != [[0; 4]; 2]
        || s.max_extent != 0
        || s.max_drift != 0
    {
        return Err(Error::Header);
    }
    Ok(None)
}
pub fn decode_pair_state<'a>(
    bytes: &[u8],
    m: &'a ValidatedManifolds<'a>,
    budget: &mut Budget,
) -> Result<ValidatedPairState<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    let identity = m.record().identity;
    read_identity(&mut read, MAGIC, identity)?;
    let p = read.word()? as usize;
    let d = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    context(identity, p, d, m)?;
    if bytes.len() != length(p, d)? {
        return Err(Error::Length);
    }
    budget.charge(
        p.checked_mul(size_of::<PairState>())
            .and_then(|v| {
                d.checked_mul(size_of::<SubDetector>())
                    .and_then(|d| v.checked_add(d))
            })
            .ok_or(Error::Limit)?,
    )?;
    let mut r = PairStateTable {
        identity,
        pairs: vector(p)?,
        detectors: vector(d)?,
    };
    for _ in 0..p {
        let event_status = read.word()?;
        let solver_color = read.word()?;
        let solver_color_bodies = words(&mut read)?;
        let recycle = read_recycle(&mut read)?;
        let workspace = match read.word()? {
            0 => WorkspaceKind::None,
            1 => WorkspaceKind::CompositeShape,
            2 => WorkspaceKind::CompositeComposite,
            _ => return Err(Error::Workspace),
        };
        let timestamp = boolean(&mut read)?;
        let detectors = RecordSpan {
            start: read.word()?,
            count: read.word()?,
        };
        r.pairs.push(PairState {
            event_status,
            solver_color,
            solver_color_bodies,
            recycle,
            workspace,
            timestamp,
            detectors,
        });
    }
    for _ in 0..d {
        r.detectors.push(SubDetector {
            key: words(&mut read)?,
            manifold: read.word()?,
            timestamp: boolean(&mut read)?,
        });
    }
    validate_pair_state(r, m, budget)
}
pub fn encode_pair_state(v: &ValidatedPairState<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let n = length(r.pairs.len(), r.detectors.len())?;
    budget.charge(n)?;
    let mut out = vector(n)?;
    write_identity(&mut out, MAGIC, r.identity);
    let mut put = |v: u32| out.extend_from_slice(&v.to_le_bytes());
    put(r.pairs.len() as u32);
    put(r.detectors.len() as u32);
    put(0);
    for p in &r.pairs {
        put(p.event_status);
        put(p.solver_color);
        for b in p.solver_color_bodies {
            put(b);
        }
        put(u32::from(p.recycle.is_some()));
        if let Some(s) = p.recycle {
            for b in s
                .relative_pose
                .translation
                .into_iter()
                .chain(s.relative_pose.rotation)
                .chain(s.rotations.into_iter().flatten())
                .chain([s.max_extent, s.max_drift])
            {
                put(b);
            }
        } else {
            for _ in 0..17 {
                put(0);
            }
        }
        put(p.workspace as u32);
        put(u32::from(p.timestamp));
        put(p.detectors.start);
        put(p.detectors.count);
    }
    for d in &r.detectors {
        put(d.key[0]);
        put(d.key[1]);
        put(d.manifold);
        put(u32::from(d.timestamp));
    }
    Ok(out)
}
