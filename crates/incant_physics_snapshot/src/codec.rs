use crate::{
    Budget, BvhRecord, Dimension, Error, Identity, LeafMap, MAX_BYTES, Node, Result, ValidatedBvh,
    Wide, records::lengths, validate, vector,
};

const MAGIC: &[u8; 8] = b"ICBVH001";
const HEADER: usize = 80;
const VERSION: u16 = 1;
const F32: u8 = 1;
const NONE: u32 = u32::MAX;

fn encoded_len(nodes: usize, slots: usize, free: usize) -> Result<usize> {
    nodes
        .checked_mul(68)
        .and_then(|n| {
            slots
                .checked_add(free)
                .and_then(|s| s.checked_mul(4))
                .and_then(|s| n.checked_add(s))
        })
        .and_then(|n| n.checked_add(HEADER))
        .filter(|n| *n <= MAX_BYTES)
        .ok_or(Error::Limit)
}
pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) position: usize,
}
impl Reader<'_> {
    pub(crate) fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.position.checked_add(N).ok_or(Error::Length)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(Error::Length)?
            .try_into()
            .map_err(|_| Error::Length)?;
        self.position = end;
        Ok(value)
    }
    pub(crate) fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn node(&mut self) -> Result<Node> {
        Ok(Node {
            mins: [self.word()?, self.word()?, self.word()?],
            children: self.word()?,
            maxs: [self.word()?, self.word()?, self.word()?],
            data: self.word()?,
        })
    }
}

/// No serde and no live solver object. Complete header, identity, section lengths
/// and aggregate record allocation are admitted before allocating any vector.
pub fn decode(bytes: &[u8], expected: Identity, budget: &mut Budget) -> Result<ValidatedBvh> {
    if bytes.len() < HEADER || bytes.len() > MAX_BYTES {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    if &read.take::<8>()? != MAGIC {
        return Err(Error::Header);
    }
    if u16::from_le_bytes(read.take()?) != VERSION {
        return Err(Error::Version);
    }
    let dimension = match read.take::<1>()?[0] {
        2 => Dimension::Planar,
        3 => Dimension::Spatial,
        _ => return Err(Error::Identity),
    };
    if read.take::<1>()?[0] != F32 {
        return Err(Error::Identity);
    }
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    let identity = Identity {
        dimension,
        build: read.take()?,
    };
    if identity != expected {
        return Err(Error::Identity);
    }
    let n = read.word()? as usize;
    let parents = read.word()? as usize;
    let slots = read.word()? as usize;
    let occupied = read.word()?;
    let free = read.word()? as usize;
    let rebuild_frame = read.word()?;
    let rebuild_start = read.word()?;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    lengths(n, parents, slots, occupied, free)?;
    if bytes.len() != encoded_len(n, slots, free)? {
        return Err(Error::Length);
    }
    let allocation = n
        .checked_mul(size_of::<Wide>() + size_of::<u32>())
        .and_then(|v| {
            slots
                .checked_mul(size_of::<Option<u32>>())
                .and_then(|s| v.checked_add(s))
        })
        .and_then(|v| {
            free.checked_mul(size_of::<u32>())
                .and_then(|f| v.checked_add(f))
        })
        .ok_or(Error::Limit)?;
    budget.charge(allocation)?;
    let mut record = BvhRecord {
        identity,
        nodes: vector(n)?,
        parents: vector(n)?,
        leaves: LeafMap {
            occupied,
            slots: vector(slots)?,
        },
        free: vector(free)?,
        rebuild_frame,
        rebuild_start,
    };
    for _ in 0..n {
        record.nodes.push(Wide {
            left: read.node()?,
            right: read.node()?,
        });
    }
    for _ in 0..n {
        record.parents.push(read.word()?);
    }
    for _ in 0..slots {
        let value = read.word()?;
        record.leaves.slots.push((value != NONE).then_some(value));
    }
    for _ in 0..free {
        record.free.push(read.word()?);
    }
    validate(record, budget)
}

/// Canonical little-endian encoding of a previously validated immutable record.
/// A future enclosing world format must supply its own integrity/source binding.
pub fn encode(value: &ValidatedBvh, budget: &mut Budget) -> Result<Vec<u8>> {
    let record = value.record();
    let len = encoded_len(
        record.nodes.len(),
        record.leaves.slots.len(),
        record.free.len(),
    )?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&[record.identity.dimension as u8, F32]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&record.identity.build);
    for value in [
        record.nodes.len() as u32,
        record.parents.len() as u32,
        record.leaves.slots.len() as u32,
        record.leaves.occupied,
        record.free.len() as u32,
        record.rebuild_frame,
        record.rebuild_start,
        0,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for wide in &record.nodes {
        for node in [wide.left, wide.right] {
            for word in node
                .mins
                .into_iter()
                .chain([node.children])
                .chain(node.maxs)
                .chain([node.data])
            {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
    }
    for &word in &record.parents {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for word in &record.leaves.slots {
        bytes.extend_from_slice(&word.unwrap_or(NONE).to_le_bytes());
    }
    for &word in &record.free {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Ok(bytes)
}
