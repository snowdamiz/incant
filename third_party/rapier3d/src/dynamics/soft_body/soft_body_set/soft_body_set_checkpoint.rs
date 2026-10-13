//! Empty-owner refusal is field-based, including removed/recycled native histories.
use super::*;
use crate::pipeline::checked_restore::{allocation::*, world::OmittedOwnerError};
impl SoftBodySet {
    // Only the private test capture issuer calls this until runtime ownership is integrated.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn checkpoint_empty_gate(&self) -> Result<(), OmittedOwnerError> {
        if !self.bodies.checkpoint_pristine_empty() {
            return Err(OmittedOwnerError::SoftArena);
        }
        if !self.island_events.is_empty() || !self.attached_to.is_empty() || self.attached_to_stale
        {
            return Err(OmittedOwnerError::SoftJournal);
        }
        Ok(())
    }
    pub(crate) fn checkpoint_empty_plan() -> ConstructionResult<AllocationPlan> {
        Arena::<SoftBody>::checkpoint_empty_plan()
    }
    pub(crate) fn checkpoint_empty(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        Ok(Self {
            bodies: Arena::checkpoint_empty(p)?,
            island_events: Vec::new(),
            attached_to: HashMap::default(),
            attached_to_stale: false,
        })
    }
}
