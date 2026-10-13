use super::WorldBoundary;
use crate::{Error, Result, ValidatedBroadOwner, ValidatedPipelineState};

/// Joins already-validated section identities and their exact shared witnesses.
/// Equal same-shape records from another owner do not authenticate provenance.
pub fn join_world_boundary<'a>(
    broad: &'a ValidatedBroadOwner<'a>,
    pipeline: &'a ValidatedPipelineState<'a>,
) -> Result<WorldBoundary<'a>> {
    let colliders = broad.context().narrow.context().rigid.collider_state();
    let pc = pipeline.context();
    if broad.record().identity != pipeline.record().identity
        || !std::ptr::eq(colliders, pc.ccd.context().colliders)
        || !std::ptr::eq(pc.integration, pc.ccd.context().integration)
    {
        return Err(Error::Identity);
    }
    Ok(WorldBoundary { broad, pipeline })
}
