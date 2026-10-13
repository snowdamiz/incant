use super::*;
use crate::pipeline::checked_restore::allocation::*;
impl ContactConstraintsSet {
    pub(crate) fn checkpoint_plan() -> ConstructionResult<AllocationPlan> {
        InteractionGroups::checkpoint_plan()
    }
    pub(crate) fn checkpoint_new(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        Ok(Self {
            generic_jacobians: DVector::zeros(0),
            interaction_groups: InteractionGroups::checkpoint_new(p)?,
            generic_velocity_constraints: vec![],
            simd_velocity_coulomb_constraints: vec![],
            generic_velocity_constraints_builder: vec![],
            simd_velocity_coulomb_constraints_builder: vec![],
            #[cfg(feature = "dim3")]
            simd_velocity_twist_constraints: vec![],
            #[cfg(feature = "dim3")]
            simd_velocity_twist_constraints_builder: vec![],
        })
    }
}
