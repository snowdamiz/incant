//! Private island selection through the exact admitted whole-frame witness.
use super::{NumericFrame, allocation::*};
use crate::dynamics::IslandManager;
pub(super) fn plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
    IslandManager::checkpoint_plan(
        frame.frame.broad().context().narrow.context().rigid,
        frame.identity(),
    )
}
pub(super) fn construct(
    frame: &NumericFrame<'_>,
    permit: &mut AllocationPermit,
) -> ConstructionResult<IslandManager> {
    IslandManager::from_checkpoint(
        frame.frame.broad().context().narrow.context().rigid,
        frame.identity(),
        permit,
    )
}
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
#[path = "island_tests.rs"]
mod tests;
