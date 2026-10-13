//! Fresh scratch: charge the fixed bucket box and fallible cursor allocation.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
impl StagedIslandSolver {
    pub(crate) fn checkpoint_plan() -> ConstructionResult<AllocationPlan> {
        let mut plan = StageSync::checkpoint_plan()?;
        plan.append(ContactConstraintsSet::checkpoint_plan()?)?;
        Ok(plan)
    }
    pub(crate) fn checkpoint_new(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        Ok(Self {
            contact_constraints: ContactConstraintsSet::checkpoint_new(p)?,
            joint_constraints: JointConstraintsSet::new(),
            soft_constraints: SoftConstraintsSet::new(),
            #[cfg(feature = "fem")]
            soft_fem: crate::dynamics::solver::soft_fem::SoftFemSet::new(),
            velocity_solver: VelocitySolver::new(),
            chunk_segments: Vec::new(),
            overflow_chunk_refs: Vec::new(),
            color_ranges: Vec::new(),
            groups: Vec::new(),
            grouped_chunk_refs: Vec::new(),
            overflow_workspace: Vec::new(),
            joint_colors: ParallelInteractionGroups::new(),
            joint_color_ranges: Vec::new(),
            joint_chunk_lanes: Vec::new(),
            joint_chunk_rows: Vec::new(),
            joint_sig_workspace: Vec::new(),
            joint_overflow_workspace: Vec::new(),
            joint_overflow_range: 0..0,
            joint_rows: Vec::new(),
            staged_joints_valid: false,
            prev_staged_active_set_epoch: 0,
            prev_staged_assembly_epoch: 0,
            prev_staged_joint_indices: Vec::new(),
            prev_staged_group_bodies: Vec::new(),
            staged_group_joint_layout: Vec::new(),
            #[cfg(feature = "dim3")]
            any_gyroscopic: AtomicBool::new(false),
            any_ccd_active: AtomicBool::new(false),
            any_bouncy: AtomicBool::new(false),
            post_solve_ccd_active: None,
            sync: StageSync::checkpoint_new(p)?,
        })
    }
}
