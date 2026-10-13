use crate::{
    ArenaRecord, ArenaSlot, Budget, Dimension, Error, Identity, MAX_BYTES, Result, ValidatedArena,
    arena::arena_lengths, codec::Reader, validate_arena, vector,
};
const MAGIC: &[u8; 8] = b"ICARN001";
const HEADER: usize = 64;
const SLOT: usize = 12;
const NONE: u32 = u32::MAX;

pub(crate) fn frame_length(header: usize, stride: usize, count: usize) -> Result<usize> {
    count
        .checked_mul(stride)
        .and_then(|v| v.checked_add(header))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or(Error::Limit)
}
pub(crate) fn read_identity(
    read: &mut Reader<'_>,
    magic: &[u8; 8],
    expected: Identity,
) -> Result<()> {
    if &read.take::<8>()? != magic {
        return Err(Error::Header);
    }
    if u16::from_le_bytes(read.take()?) != 1 {
        return Err(Error::Version);
    }
    let dimension = match read.take::<1>()?[0] {
        2 => Dimension::Planar,
        3 => Dimension::Spatial,
        _ => return Err(Error::Identity),
    };
    if read.take::<1>()?[0] != 1 {
        return Err(Error::Identity);
    }
    let identity = Identity {
        dimension,
        build: read.take()?,
    };
    if identity != expected {
        return Err(Error::Identity);
    }
    Ok(())
}
pub(crate) fn write_identity(bytes: &mut Vec<u8>, magic: &[u8; 8], identity: Identity) {
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&[identity.dimension as u8, 1]);
    bytes.extend_from_slice(&identity.build);
}

pub fn decode_arena(
    bytes: &[u8],
    expected: Identity,
    budget: &mut Budget,
) -> Result<ValidatedArena> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    read_identity(&mut read, MAGIC, expected)?;
    let generation = read.word()?;
    let live = read.word()?;
    let count = read.word()? as usize;
    let head = read.word()?;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    arena_lengths(count, live, generation)?;
    if (head != NONE && head as usize >= count) || bytes.len() != frame_length(HEADER, SLOT, count)?
    {
        return Err(Error::Length);
    }
    budget.charge(
        count
            .checked_mul(size_of::<ArenaSlot>())
            .ok_or(Error::Limit)?,
    )?;
    let mut slots = vector(count)?;
    for _ in 0..count {
        let tag = read.word()?;
        let a = read.word()?;
        let b = read.word()?;
        slots.push(match tag {
            0 if b == 0 => ArenaSlot::Free {
                next: (a != NONE).then_some(a),
            },
            1 => ArenaSlot::Occupied {
                generation: a,
                row: b,
            },
            _ => return Err(Error::Node),
        });
    }
    validate_arena(
        ArenaRecord {
            identity: expected,
            generation,
            live,
            free_head: (head != NONE).then_some(head),
            slots,
        },
        budget,
    )
}

pub fn encode_arena(value: &ValidatedArena, budget: &mut Budget) -> Result<Vec<u8>> {
    let record = value.record();
    let len = frame_length(HEADER, SLOT, record.slots.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, record.identity);
    for word in [
        record.generation,
        record.live,
        record.slots.len() as u32,
        record.free_head.unwrap_or(NONE),
        0,
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for slot in &record.slots {
        let words = match *slot {
            ArenaSlot::Free { next } => [0, next.unwrap_or(NONE), 0],
            ArenaSlot::Occupied { generation, row } => [1, generation, row],
        };
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    Ok(bytes)
}
