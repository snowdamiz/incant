//! Select only the exact broad witness joined to this admitted numeric frame.
use super::{NumericFrame, allocation::*};
use crate::geometry::BroadPhaseBvh;
pub(super) fn plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
    BroadPhaseBvh::checkpoint_plan(frame.frame.broad(), frame.identity())
}
pub(super) fn construct(
    frame: &NumericFrame<'_>,
    permit: &mut AllocationPermit,
) -> ConstructionResult<BroadPhaseBvh> {
    BroadPhaseBvh::from_checkpoint(frame.frame.broad(), frame.identity(), permit)
}
