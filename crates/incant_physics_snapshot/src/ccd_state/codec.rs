use super::*;
use crate::{Budget, Error, Handle, MAX_BYTES, Result, arena_codec::*, codec::Reader, vector};
const MAGIC: &[u8; 8] = b"ICCCD001";
const HEADER: usize = 60;
const STRIDE: usize = 32;

pub fn decode_ccd_state<'a>(
    bytes: &[u8],
    c: CcdContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedCcdState<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let expected = c.integration.record().identity;
    validate::context(expected, c)?;
    let mut r = Reader { bytes, position: 0 };
    read_identity(&mut r, MAGIC, expected)?;
    let tag = r.word()?;
    let prediction = r.word()?;
    let count = r.word()? as usize;
    if r.word()? != 0 || tag > 2 || (tag != 1 && count != 0) || (tag == 0 && prediction != 0) {
        return Err(Error::Header);
    }
    if count > MAX_CCD_FIXED_TARGETS {
        return Err(Error::Limit);
    }
    if bytes.len() != frame_length(HEADER, STRIDE, count)? {
        return Err(Error::Length);
    }
    let cache = match tag {
        0 => CcdCache::Uninitialized,
        1 => {
            budget.charge(
                count
                    .checked_mul(size_of::<CcdTarget>())
                    .ok_or(Error::Limit)?,
            )?;
            let mut targets = vector(count)?;
            for _ in 0..count {
                let collider = Handle {
                    slot: r.word()?,
                    generation: r.word()?,
                };
                let mut mins = [0; 3];
                let mut maxs = [0; 3];
                for value in &mut mins {
                    *value = r.word()?;
                }
                for value in &mut maxs {
                    *value = r.word()?;
                }
                targets.push(CcdTarget {
                    collider,
                    mins,
                    maxs,
                });
            }
            CcdCache::Targets {
                prediction,
                targets,
            }
        }
        2 => CcdCache::FullBvh { prediction },
        _ => unreachable!(),
    };
    validate_ccd_state(
        CcdState {
            identity: expected,
            cache,
        },
        c,
    )
}
pub fn encode_ccd_state(value: &ValidatedCcdState<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let (tag, prediction, targets): (u32, u32, &[CcdTarget]) = match &value.record.cache {
        CcdCache::Uninitialized => (0, 0, &[]),
        CcdCache::Targets {
            prediction,
            targets,
        } => (1, *prediction, targets),
        CcdCache::FullBvh { prediction } => (2, *prediction, &[]),
    };
    let len = frame_length(HEADER, STRIDE, targets.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, value.record.identity);
    for word in [tag, prediction, targets.len() as u32, 0] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for t in targets {
        for word in [t.collider.slot, t.collider.generation]
            .into_iter()
            .chain(t.mins)
            .chain(t.maxs)
        {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    Ok(bytes)
}
