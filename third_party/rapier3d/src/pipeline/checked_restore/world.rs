//! Atomic private primitive-world assembly. No production capture capability or
//! public bytes-to-live entry exists. The only context issuer is the test harness,
//! which owns default dispatch, supported empty families and completed steps.
use super::{
    NumericFrame, allocation, allocation::*, body, broad, collider, convert::v, integration,
    islands, narrow,
};
use crate::dynamics::{CCDSolver, ImpulseJointSet, MultibodyJointSet, SoftBodySet};
use crate::pipeline::{CollisionPipeline, PhysicsPipeline, PhysicsWorld};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OmittedOwnerError {
    ImpulseArena,
    ImpulseGraph,
    ImpulseJournal,
    MultibodyArena,
    MultibodyGraph,
    MultibodyJournal,
    MultibodyEpoch,
    SoftArena,
    SoftJournal,
}
// No public or non-test constructor. Its ownership is not inferred from an
// untrusted phase tag, current owner lengths, serialization equality, or a bool.
mod frame;
use frame::AssemblyFrame;
mod admit;
use admit::AdmittedAssembly;
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
use admit::with_admitted;

struct CaptureContext {
    #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
    pool: Option<std::sync::Arc<rayon::ThreadPool>>,
}
fn plan(admitted: &AdmittedAssembly<'_>) -> ConstructionResult<AllocationPlan> {
    let n = admitted.numeric();
    let mut p = AllocationPlan::default();
    for part in [
        body::plan(n)?,
        collider::plan(n)?,
        islands::plan(n)?,
        broad::plan(n)?,
        narrow::plan(n)?,
        CCDSolver::checkpoint_plan(n)?,
        PhysicsPipeline::checkpoint_construction_plan()?,
        ImpulseJointSet::checkpoint_empty_plan()?,
        MultibodyJointSet::checkpoint_empty_plan()?,
        SoftBodySet::checkpoint_empty_plan()?,
    ] {
        p.append(part)?;
    }
    Ok(p)
}
fn construct(
    admitted: &AdmittedAssembly<'_>,
    context: &CaptureContext,
    budget: &mut AllocationBudget,
) -> ConstructionResult<PhysicsWorld> {
    // Admit the complete request before any live owner allocation; a later
    // failure drops all local owners and never replaces the source world.
    let mut p = budget.admit(plan(admitted)?)?;
    let candidate = construct_with_permit(admitted, context, &mut p)?;
    p.finish()?;
    Ok(candidate)
}
fn construct_with_permit(
    admitted: &AdmittedAssembly<'_>,
    _context: &CaptureContext,
    p: &mut AllocationPermit,
) -> ConstructionResult<PhysicsWorld> {
    let n = admitted.numeric();
    let integration_parameters = integration::construct(n)?;
    let bodies = body::construct(n, p)?;
    let colliders = collider::construct(n, p)?;
    let islands = islands::construct(n, p)?;
    let broad_phase = broad::construct(n, p)?;
    let narrow_phase = narrow::construct(n, p)?;
    let ccd_solver = CCDSolver::from_checkpoint(n, p)?;
    let impulse_joints = ImpulseJointSet::checkpoint_empty(admitted.impulse_epoch(), p)?;
    let multibody_joints = MultibodyJointSet::checkpoint_empty(p)?;
    let soft_bodies = SoftBodySet::checkpoint_empty(p)?;
    let physics_pipeline = PhysicsPipeline::from_checkpoint(
        n,
        p,
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
        _context.pool.clone(),
    )?;
    Ok(PhysicsWorld {
        gravity: v(n.boundary().pipeline().record().gravity),
        integration_parameters,
        physics_pipeline,
        collision_pipeline: CollisionPipeline::new(),
        islands,
        broad_phase,
        narrow_phase,
        bodies,
        colliders,
        impulse_joints,
        multibody_joints,
        soft_bodies,
        ccd_solver,
    })
}
// The prepared in-crate CPU harness supplies exact one-way capture code here.
// No installed downstream crate can construct CaptureContext.
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
#[path = "world_tests.rs"]
mod tests;
