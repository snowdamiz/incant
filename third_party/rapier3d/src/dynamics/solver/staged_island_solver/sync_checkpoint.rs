use super::*;
use crate::pipeline::checked_restore::allocation::*;
impl StageSync {
    pub(crate) fn checkpoint_plan() -> ConstructionResult<AllocationPlan> {
        AllocationPlan::array::<PaddedCursor>(1)
    }
    pub(crate) fn checkpoint_new(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        let mut cursors = p.vector(1)?;
        cursors.push(PaddedCursor(AtomicCursor::new(0)));
        Ok(Self {
            num_workers: 1,
            published: AtomicUsize::new(0),
            advance_ticket: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            cursors,
        })
    }
}
