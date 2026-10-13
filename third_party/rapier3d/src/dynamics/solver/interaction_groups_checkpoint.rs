//! The native fixed SIMD bucket box is charged before allocation.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
impl InteractionGroups {
    pub(crate) fn checkpoint_plan() -> ConstructionResult<AllocationPlan> {
        AllocationPlan::array::<[[ContactRef; SIMD_WIDTH]; 128]>(1)
    }
    pub(crate) fn checkpoint_new(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        Ok(Self {
            bucket_len: [0; 128],
            body_masks: Vec::new(),
            to_group: Vec::new(),
            ref_bucket_slots: p.boxed([[ContactRef::PADDING; SIMD_WIDTH]; 128])?,
            simd_ref_interactions: Vec::new(),
            nongrouped_ref_interactions: Vec::new(),
        })
    }
}
