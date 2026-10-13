use super::*;
use crate::{Budget, Error, Result, arena_codec::*, codec::Reader, vector};
const MAGIC: &[u8; 8] = b"ICPIP001";
const SIZE: usize = 112;

/// Fixed-size hostile decode uses stack storage and invokes no native code.
pub fn decode_pipeline_state<'a>(
    bytes: &[u8],
    c: PipelineContext<'a>,
) -> Result<ValidatedPipelineState<'a>> {
    if bytes.len() != SIZE {
        return Err(Error::Length);
    }
    let identity = c.integration.record().identity;
    validate::context(identity, c)?;
    let mut r = Reader { bytes, position: 0 };
    read_identity(&mut r, MAGIC, identity)?;
    let gravity = [r.word()?, r.word()?, r.word()?];
    let post_solve_ccd_active = match r.word()? {
        0 => None,
        1 => Some(false),
        2 => Some(true),
        _ => return Err(Error::Pipeline),
    };
    let phase = match r.word()? {
        0 => PipelinePhase::Unverified,
        1 => PipelinePhase::CompletedPositive,
        2 => PipelinePhase::CollisionOnly,
        _ => return Err(Error::PipelinePhase),
    };
    let flags = r.word()?;
    if flags & !3 != 0 || r.word()? != 0 {
        return Err(Error::Header);
    }
    let mut quarantine = [0; 5];
    for count in &mut quarantine {
        *count = u64::from_le_bytes(r.take()?);
    }
    validate_pipeline_state(
        PipelineState {
            identity,
            gravity,
            post_solve_ccd_active,
            phase,
            deferred_bvh_receiver: flags & 1 != 0,
            deferred_bvh_inline: flags & 2 != 0,
            quarantine,
        },
        c,
    )
}
pub fn encode_pipeline_state(
    v: &ValidatedPipelineState<'_>,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    budget.charge(SIZE)?;
    let mut bytes = vector(SIZE)?;
    let r = v.record();
    write_identity(&mut bytes, MAGIC, r.identity);
    let verdict = match r.post_solve_ccd_active {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    };
    for word in r.gravity.into_iter().chain([
        verdict,
        r.phase as u32,
        u32::from(r.deferred_bvh_receiver) | (u32::from(r.deferred_bvh_inline) << 1),
        0,
    ]) {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for count in r.quarantine {
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    Ok(bytes)
}
