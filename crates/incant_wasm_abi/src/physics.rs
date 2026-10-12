//! Canonical, allocation-free physics frames. Every result is checked in full.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Malformed,
}
pub type Result<T> = core::result::Result<T, Error>;
fn malformed() -> Error {
    Error::Malformed
}
fn zero(v: &[f64]) -> bool {
    v.iter().all(|v| *v == 0.)
}
pub fn validate_query(q: &PhysicsQuery) -> Result<()> {
    if q.reserved != 0 || !q.is_finite() {
        return Err(malformed());
    }
    match q.kind {
        0 | 1 => {
            if q.flags & !3 != 0
                || (q.flags & 2 == 0 && q.entity != [0; 16])
                || q.a.iter().chain(&q.b).any(|v| v.abs() > 1_000_000.)
                || zero(&q.b)
                || !(0. ..=1_000_000.).contains(&q.c)
                || q.d != 0.
                || q.e != 0.
                || (q.kind == 1 && (q.a[2] != 0. || q.b[2] != 0.))
            {
                return Err(malformed());
            }
        }
        2 => {
            let distance = |v: f64| (0.001..=1000.).contains(&v);
            if q.flags & !15 != 0
                || q.memberships != 0
                || q.filter != 0
                || q.a.iter().any(|v| v.abs() > 10_000.)
                || !distance(q.b[0])
                || q.b[1..]
                    .iter()
                    .any(|v| !(0. ..=core::f64::consts::FRAC_PI_2).contains(v))
                || (if q.flags & 2 != 0 {
                    !distance(q.c)
                } else {
                    q.c != 0.
                })
                || (if q.flags & 4 != 0 {
                    !distance(q.d) || !distance(q.e)
                } else {
                    q.d != 0. || q.e != 0. || q.flags & 8 != 0
                })
            {
                return Err(malformed());
            }
        }
        _ => return Err(malformed()),
    }
    Ok(())
}
pub fn encode_batch(queries: &[PhysicsQuery], scratch: &mut [u8]) -> Result<usize> {
    if queries.len() > PHYSICS_MAX_BATCH {
        return Err(Error::Bounds);
    }
    let length = PhysicsHeader::SIZE + queries.len() * PhysicsQuery::SIZE;
    if scratch.len() < length {
        return Err(Error::Bounds);
    }
    for q in queries {
        validate_query(q)?;
    }
    scratch[..PhysicsHeader::SIZE].copy_from_slice(
        &PhysicsHeader {
            tag: *PHYSICS_REQUEST_MAGIC,
            count: queries.len() as u32,
            extra: 0,
        }
        .encode(),
    );
    for (q, b) in queries.iter().zip(
        scratch[PhysicsHeader::SIZE..length]
            .as_chunks_mut::<{ PhysicsQuery::SIZE }>()
            .0,
    ) {
        b.copy_from_slice(&q.encode());
    }
    Ok(length)
}
#[derive(Clone, Copy)]
pub struct Batch<'a> {
    rows: &'a [u8],
}
impl<'a> Batch<'a> {
    pub fn len(&self) -> usize {
        self.rows.len() / PhysicsQuery::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = PhysicsQuery> + 'a {
        self.rows
            .as_chunks::<{ PhysicsQuery::SIZE }>()
            .0
            .iter()
            .map(|r| PhysicsQuery::decode(r).expect("validated query"))
    }
}
pub fn decode_batch(bytes: &[u8]) -> Result<Batch<'_>> {
    let h = PhysicsHeader::decode(bytes).ok_or_else(malformed)?;
    if &h.tag != PHYSICS_REQUEST_MAGIC
        || h.extra != 0
        || h.count as usize > PHYSICS_MAX_BATCH
        || bytes.len() != PhysicsHeader::SIZE + h.count as usize * PhysicsQuery::SIZE
    {
        return Err(malformed());
    }
    let batch = Batch {
        rows: &bytes[PhysicsHeader::SIZE..],
    };
    for q in batch.iter() {
        validate_query(&q)?;
    }
    Ok(batch)
}
#[derive(Clone, Copy)]
pub struct Results<'a> {
    rows: &'a [u8],
    collisions: &'a [u8],
}
impl<'a> Results<'a> {
    pub fn len(&self) -> usize {
        self.rows.len() / PhysicsRow::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (PhysicsRow, &'a [[u8; 16]])> + 'a {
        let collisions = self.collisions;
        self.rows
            .as_chunks::<{ PhysicsRow::SIZE }>()
            .0
            .iter()
            .map(move |b| {
                let r = PhysicsRow::decode(b).expect("validated result");
                let ids =
                    &collisions[r.first as usize * 16..(r.first as usize + r.count as usize) * 16];
                (r, ids.as_chunks::<16>().0)
            })
    }
}
pub fn decode_results(bytes: &[u8]) -> Result<Results<'_>> {
    let h = PhysicsHeader::decode(bytes).ok_or_else(malformed)?;
    if &h.tag != PHYSICS_RESULT_MAGIC
        || h.count as usize > PHYSICS_MAX_BATCH
        || h.extra as usize > PHYSICS_MAX_COLLISIONS
    {
        return Err(malformed());
    }
    let end = PhysicsHeader::SIZE + h.count as usize * PhysicsRow::SIZE;
    if bytes.len() != end + h.extra as usize * 16 {
        return Err(malformed());
    }
    let result = Results {
        rows: &bytes[PhysicsHeader::SIZE..end],
        collisions: &bytes[end..],
    };
    let mut next = 0;
    for b in result.rows.as_chunks::<{ PhysicsRow::SIZE }>().0 {
        let r = PhysicsRow::decode(b).ok_or_else(malformed)?;
        if !r.is_finite() || r.reserved != 0 {
            return Err(malformed());
        }
        match r.kind {
            0 | 1 => {
                if r.flags > 1 || r.first != 0 || r.count != 0 {
                    return Err(malformed());
                }
                if r.flags == 0 {
                    if r.entity != [0; 16]
                        || r.distance != 0.
                        || !zero(&r.point)
                        || !zero(&r.normal)
                    {
                        return Err(malformed());
                    }
                } else if !(0. ..=1_000_000.).contains(&r.distance)
                    || r.point.iter().any(|v| v.abs() > 2_000_001.)
                    || r.normal.iter().any(|v| v.abs() > 1.000001)
                    || (r.kind == 1 && (r.point[2] != 0. || r.normal[2] != 0.))
                {
                    return Err(malformed());
                }
            }
            2 => {
                if r.flags & !3 != 0
                    || r.entity != [0; 16]
                    || r.distance != 0.
                    || !zero(&r.normal)
                    || r.point.iter().any(|v| v.abs() > 20_000.)
                    || r.first as usize != next
                    || r.count as usize > h.extra as usize - next
                {
                    return Err(malformed());
                }
                let ids = result.collisions[next * 16..(next + r.count as usize) * 16]
                    .as_chunks::<16>()
                    .0;
                if ids.windows(2).any(|p| p[0] >= p[1]) {
                    return Err(malformed());
                }
                next += r.count as usize;
            }
            _ => return Err(malformed()),
        }
    }
    if next != h.extra as usize {
        return Err(malformed());
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub struct Events<'a> {
    pub scene: [u8; 16],
    pub dimension: u32,
    rows: &'a [u8],
}
impl<'a> Events<'a> {
    pub fn len(&self) -> usize {
        self.rows.len() / PhysicsEvent::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = PhysicsEvent> + 'a {
        self.rows
            .as_chunks::<{ PhysicsEvent::SIZE }>()
            .0
            .iter()
            .map(|b| PhysicsEvent::decode(b).expect("validated event"))
    }
}
pub fn decode_events(bytes: &[u8]) -> Result<Events<'_>> {
    let h = PhysicsEventHeader::decode(bytes).ok_or_else(malformed)?;
    if &h.tag != PHYSICS_EVENTS_MAGIC
        || ![2, 3].contains(&h.dimension)
        || h.count as usize > PHYSICS_MAX_EVENTS
        || bytes.len() != PhysicsEventHeader::SIZE + h.count as usize * PhysicsEvent::SIZE
    {
        return Err(malformed());
    }
    let result = Events {
        scene: h.scene,
        dimension: h.dimension,
        rows: &bytes[PhysicsEventHeader::SIZE..],
    };
    let mut previous = None;
    for event in result.iter() {
        let key = (event.first, event.second, event.entered);
        if event.first >= event.second
            || event.entered > 1
            || event.reserved != 0
            || previous.is_some_and(|p| p >= key)
        {
            return Err(malformed());
        }
        previous = Some(key);
    }
    Ok(result)
}
