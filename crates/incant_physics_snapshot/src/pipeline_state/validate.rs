use super::*;
use crate::{Dimension, Error, Result};

pub(super) fn context(id: Identity, c: PipelineContext<'_>) -> Result<()> {
    if id != c.integration.record().identity
        || id != c.ccd.record().identity
        || !std::ptr::eq(c.integration, c.ccd.context().integration)
    {
        return Err(Error::Identity);
    }
    Ok(())
}
pub fn validate_pipeline_state(
    record: PipelineState,
    c: PipelineContext<'_>,
) -> Result<ValidatedPipelineState<'_>> {
    context(record.identity, c)?;
    // This checks consistency of the declared scalar boundary, not provenance.
    // Current configuration can be changed after a native step; the enclosing
    // owner must authenticate a completed step under this exact configuration.
    if record.phase != PipelinePhase::CompletedPositive
        || f32::from_bits(c.integration.record().dt) <= 0.
    {
        return Err(Error::PipelinePhase);
    }
    if record.deferred_bvh_receiver || record.deferred_bvh_inline {
        return Err(Error::PipelineDeferred);
    }
    if record.quarantine != [0; 5] {
        return Err(Error::PipelineQuarantine);
    }
    if (record.identity.dimension == Dimension::Planar && record.gravity[2] != 0)
        || record
            .gravity
            .iter()
            .any(|b| !f32::from_bits(*b).is_finite())
    {
        return Err(Error::Pipeline);
    }
    // Do not infer the verdict from cache presence, current awake bodies or CCD
    // settings: actual native worlds retain all three distinct historical states.
    Ok(ValidatedPipelineState { record, context: c })
}
