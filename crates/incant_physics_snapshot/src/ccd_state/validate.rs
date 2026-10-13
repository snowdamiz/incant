use super::*;
use crate::{BodyKind, ColliderEnabled, Dimension, Error, PendingJournals, Result};

fn fixed(context: CcdContext<'_>, row: usize) -> bool {
    let colliders = context.colliders;
    if colliders.record().colliders[row].enabled != ColliderEnabled::Enabled {
        return false;
    }
    let bodies = colliders.context().bodies;
    match bodies.attachments().record().colliders[row] {
        None => true,
        Some(parent) => bodies
            .body(parent.body)
            .is_some_and(|body| body.kind == BodyKind::Fixed),
    }
}
pub(super) fn context(id: Identity, c: CcdContext<'_>) -> Result<()> {
    if id != c.colliders.record().identity || id != c.integration.record().identity {
        return Err(Error::Identity);
    }
    if c.colliders.context().journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    Ok(())
}
pub fn validate_ccd_state(record: CcdState, c: CcdContext<'_>) -> Result<ValidatedCcdState<'_>> {
    context(record.identity, c)?;
    let (prediction, targets) = match &record.cache {
        CcdCache::Uninitialized => return Ok(ValidatedCcdState { record, context: c }),
        CcdCache::Targets {
            prediction,
            targets,
        } => (*prediction, Some(targets)),
        CcdCache::FullBvh { prediction } => (*prediction, None),
    };
    let p = f32::from_bits(prediction);
    if !p.is_finite() || p < 0. {
        return Err(Error::Ccd);
    }
    // A completed pipeline invalidates the cache on every body/collider scene
    // change. Parameter-only changes can leave historical prediction intact.
    let fixed_count = (0..c.colliders.record().colliders.len())
        .filter(|row| fixed(c, *row))
        .count();
    let Some(targets) = targets else {
        return if fixed_count > MAX_CCD_FIXED_TARGETS {
            Ok(ValidatedCcdState { record, context: c })
        } else {
            Err(Error::Ccd)
        };
    };
    if targets.len() > MAX_CCD_FIXED_TARGETS || targets.len() != fixed_count {
        return Err(Error::Length);
    }
    let attachments = c.colliders.context().bodies.attachments();
    let mut previous = None;
    for target in targets {
        let row = attachments
            .collider_row(target.collider)
            .ok_or(Error::Generation)? as usize;
        if !fixed(c, row) || previous.is_some_and(|slot| slot >= target.collider.slot) {
            return Err(Error::Ccd);
        }
        previous = Some(target.collider.slot);
        if record.identity.dimension == Dimension::Planar
            && (target.mins[2] != 0 || target.maxs[2] != 0)
        {
            return Err(Error::Bounds);
        }
        for axis in 0..record.identity.dimension as usize {
            let min = f32::from_bits(target.mins[axis]);
            let max = f32::from_bits(target.maxs[axis]);
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err(Error::Bounds);
            }
        }
    }
    Ok(ValidatedCcdState { record, context: c })
}
