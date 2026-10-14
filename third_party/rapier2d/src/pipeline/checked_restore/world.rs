//! Atomic private closed-shape world assembly. The production exclusive capture
//! owner issues the context after its authentic completed step. No public
//! arbitrary bytes-to-live entry or caller-supplied authority flag exists.
use super::{
    NumericFrame, allocation, allocation::*, body, broad, collider, convert::v, integration,
    islands, narrow,
};
use crate::dynamics::{CCDSolver, ImpulseJointSet, MultibodyJointSet, SoftBodySet};
use crate::pipeline::{CollisionPipeline, PhysicsPipeline, PhysicsWorld};

/// Exact unsupported native family field; no empty-length shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmittedOwnerError {
    /// Unsupported ImpulseArena history.
    ImpulseArena,
    /// Unsupported ImpulseGraph history.
    ImpulseGraph,
    /// Unsupported ImpulseJournal history.
    ImpulseJournal,
    /// Unsupported MultibodyArena history.
    MultibodyArena,
    /// Unsupported MultibodyGraph history.
    MultibodyGraph,
    /// Unsupported MultibodyJournal history.
    MultibodyJournal,
    /// Unsupported MultibodyEpoch history.
    MultibodyEpoch,
    /// Unsupported SoftArena history.
    SoftArena,
    /// Unsupported SoftJournal history.
    SoftJournal,
}
// No public constructor for the assembly context. Its ownership is not inferred
// from an untrusted phase tag, current lengths, serialization equality, or a bool.
mod frame;
use frame::AssemblyFrame;
mod admit;
use admit::AdmittedAssembly;
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
use admit::with_admitted;

pub(super) struct CaptureContext {
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

// The production caller is the sibling opaque capture module. It cannot receive
// arbitrary public bytes: NativeFrame fields and its issuance are private.
pub(super) fn capture_context(_world: &PhysicsWorld) -> CaptureContext {
    CaptureContext {
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
        pool: _world.thread_pool(),
    }
}
pub(super) fn capture_envelope(
    inner: &[u8],
    epoch: u32,
    budget: &mut incant_physics_snapshot::Budget,
) -> incant_physics_snapshot::Result<alloc::vec::Vec<u8>> {
    use incant_physics_snapshot::{Error, MAX_BYTES};
    let len = inner
        .len()
        .checked_add(16)
        .filter(|n| *n <= MAX_BYTES + 16)
        .ok_or(Error::Limit)?;
    let mut bytes = budget.vector(len)?;
    bytes.extend_from_slice(b"ICNATIV1");
    bytes.extend_from_slice(&(inner.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&epoch.to_le_bytes());
    bytes.extend_from_slice(inner);
    Ok(bytes)
}
pub(super) fn restore_captured(
    bytes: &[u8],
    expected: incant_physics_snapshot::Identity,
    context: &CaptureContext,
    decode: &mut incant_physics_snapshot::Budget,
    work: &mut super::WorkBudget,
    allocations: &mut AllocationBudget,
) -> Result<PhysicsWorld, super::capture::CaptureError> {
    use super::capture::CaptureError as E;
    let assembly = AssemblyFrame::decode(bytes).map_err(E::Construction)?;
    admit::with_admitted(&assembly, expected, decode, work, |a| {
        construct(a, context, allocations)
    })
    .map_err(|e| match e {
        admit::AssemblyError::Decode(e) => E::Record(e),
        admit::AssemblyError::Numeric(e) => E::Numeric(e),
        admit::AssemblyError::Construction(e) => E::Construction(e),
    })
}
