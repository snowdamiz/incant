use crate::{
    Budget, CoarenaRecord, CoarenaRow, Error, MAX_BYTES, Result, ValidatedArena, ValidatedCoarena,
    arena_codec::{frame_length, read_identity, write_identity},
    coarena::coarena_length,
    codec::Reader,
    validate_coarena, vector,
};
const MAGIC: &[u8; 8] = b"ICCOA001";
const HEADER: usize = 52;
const ROW: usize = 8;

pub fn decode_coarena<'a>(
    bytes: &[u8],
    arena: &'a ValidatedArena,
    budget: &mut Budget,
) -> Result<ValidatedCoarena<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    read_identity(&mut read, MAGIC, arena.record().identity)?;
    let count = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    coarena_length(count, arena)?;
    if bytes.len() != frame_length(HEADER, ROW, count)? {
        return Err(Error::Length);
    }
    budget.charge(
        count
            .checked_mul(size_of::<CoarenaRow>())
            .ok_or(Error::Limit)?,
    )?;
    let mut rows = vector(count)?;
    for _ in 0..count {
        rows.push(CoarenaRow {
            generation: read.word()?,
            row: read.word()?,
        });
    }
    validate_coarena(
        CoarenaRecord {
            identity: arena.record().identity,
            rows,
        },
        arena,
        budget,
    )
}

pub fn encode_coarena(value: &ValidatedCoarena<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let record = value.record();
    let len = frame_length(HEADER, ROW, record.rows.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, record.identity);
    bytes.extend_from_slice(&(record.rows.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for row in &record.rows {
        bytes.extend_from_slice(&row.generation.to_le_bytes());
        bytes.extend_from_slice(&row.row.to_le_bytes());
    }
    Ok(bytes)
}
