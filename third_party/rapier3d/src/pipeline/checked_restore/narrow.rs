use super::{NumericFrame, allocation::*};
use crate::geometry::NarrowPhase;
pub(super) fn plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
    NarrowPhase::checkpoint_plan(frame)
}
pub(super) fn construct(
    frame: &NumericFrame<'_>,
    p: &mut AllocationPermit,
) -> ConstructionResult<NarrowPhase> {
    NarrowPhase::from_checkpoint(frame, p)
}
