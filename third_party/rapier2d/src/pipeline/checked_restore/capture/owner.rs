use super::super::WorkBudget;
use super::{CaptureError, compiled_identity};
use crate::{
    geometry::{ContactData, ContactManifoldData},
    pipeline::PhysicsWorld,
};
use alloc::sync::Arc;
use incant_physics_snapshot::{Identity, MAX_ITEMS};
use parry::query::PersistentQueryDispatcher;
pub(super) type Dispatcher = Arc<dyn PersistentQueryDispatcher<ContactManifoldData, ContactData>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Dirty,
    Completed,
    ImportedValidated,
    Poisoned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameOrigin {
    NativeCompleted,
    ImportedValidated,
    ImportedCompleted,
}
/// Owns native state and its completed-step authority. No `From<PhysicsWorld>`,
/// raw replacement, mutable Deref or caller-supplied authenticity flag exists.
/// Ordinary mutations use `access_mut`, invalidating authority before borrowing.
pub struct CheckpointOwner {
    world: PhysicsWorld,
    pub(super) dispatcher: Dispatcher,
    pub(super) mutation_revision: u64,
    identity: Result<Identity, CaptureError>,
    phase: Phase,
    origin: FrameOrigin,
}
/// Exclusive short-lived proof issued only by the owner after its actual step.
/// Empty counters, imported bytes and phase tags cannot construct this value.
pub struct CompletedStep<'a> {
    owner: &'a mut CheckpointOwner,
    identity: Identity,
}
impl CheckpointOwner {
    /// Create fresh default-native state with the actual compiled identity.
    pub fn new() -> Result<Self, CaptureError> {
        let owner = Self::for_simulation();
        owner.identity?;
        Ok(owner)
    }
    /// Always create ordinary native simulation, even when checkpoint capture
    /// is unavailable for this selected build. Capture admission stays fallible.
    pub fn for_simulation() -> Self {
        let world = PhysicsWorld::new();
        let dispatcher = world.narrow_phase.checkpoint_dispatcher();
        Self {
            world,
            dispatcher,
            mutation_revision: 0,
            identity: compiled_identity(),
            phase: Phase::Dirty,
            origin: FrameOrigin::NativeCompleted,
        }
    }
    pub(crate) fn restored(world: PhysicsWorld, identity: Identity, origin: FrameOrigin) -> Self {
        let dispatcher = world.narrow_phase.checkpoint_dispatcher();
        Self {
            world,
            dispatcher,
            mutation_revision: 0,
            identity: Ok(identity),
            phase: if origin == FrameOrigin::ImportedValidated {
                Phase::ImportedValidated
            } else {
                Phase::Completed
            },
            origin,
        }
    }
    /// Capture the owner's completed state or its independently validated imported
    /// state. The latter retains imported provenance and cannot issue CompletedStep.
    /// Neither path trusts a caller phase claim.
    pub fn checkpoint(
        &mut self,
        budget: &mut incant_physics_snapshot::Budget,
        work: &mut WorkBudget,
    ) -> Result<super::NativeFrame, CaptureError> {
        self.healthy()?;
        if !matches!(self.phase, Phase::Completed | Phase::ImportedValidated) {
            return Err(CaptureError::NotCompleted);
        }
        let identity = self.validate_boundary(work)?;
        let w = &mut self.world;
        let inner = super::aggregate::encode(w, identity, budget, work)?;
        let bytes =
            super::super::world::capture_envelope(&inner, w.impulse_joints.assembly_epoch, budget)?;
        let context = super::super::world::capture_context(w);
        Ok(super::NativeFrame {
            identity,
            bytes,
            context,
            origin: self.origin,
        })
    }
    /// Actual library identity, never taken from a caller's record.
    pub fn identity(&self) -> Result<Identity, CaptureError> {
        self.identity
    }
    /// Healthy immutable lookup does not invalidate completed-step authority.
    pub fn get(&self) -> Result<&PhysicsWorld, CaptureError> {
        self.healthy()?;
        Ok(&self.world)
    }
    /// Check one cumulative allowance for every forthcoming mutable borrow or
    /// step in an outer transaction, including structural permit operations.
    /// This does not reserve or advance the private revision. Call before any
    /// mutation; the caller must not spend separately checked overlapping
    /// allowances. Imported bytes never supply the revision.
    pub fn preflight_revision_edits(&self, count: u64) -> Result<(), CaptureError> {
        self.healthy()?;
        self.mutation_revision
            .checked_add(count)
            .ok_or(CaptureError::MutationExhausted)?;
        Ok(())
    }
    /// Invalidate before exposing any mutation, including a native owner swap.
    /// The original dispatcher Arc is retained so pointer reuse cannot forge it.
    pub fn access_mut(&mut self) -> Result<&mut PhysicsWorld, CaptureError> {
        self.healthy()?;
        self.mutation_revision = self
            .mutation_revision
            .checked_add(1)
            .ok_or(CaptureError::MutationExhausted)?;
        self.phase = Phase::Dirty;
        Ok(&mut self.world)
    }
    /// Advance with the default hooks/event handler. An unwind leaves poison;
    /// normal zero/nonfinite time is not reinterpreted as capture completion.
    /// Unsupported capture state does not alter ordinary simulation outcomes.
    pub fn step(&mut self) -> Result<(), CaptureError> {
        self.advance(PhysicsWorld::step)
    }
    fn advance(&mut self, advance: impl FnOnce(&mut PhysicsWorld)) -> Result<(), CaptureError> {
        self.healthy()?;
        if self.origin != FrameOrigin::NativeCompleted {
            super::super::persisted::pre_step(&self.world).map_err(CaptureError::ImportedDomain)?;
        }
        self.mutation_revision = self
            .mutation_revision
            .checked_add(1)
            .ok_or(CaptureError::MutationExhausted)?;
        self.phase = Phase::Poisoned;
        let dt = self.world.integration_parameters.dt;
        advance(&mut self.world);
        self.phase = if dt.is_finite() && dt > 0. {
            Phase::Completed
        } else {
            Phase::Dirty
        };
        if self.phase == Phase::Completed && self.origin != FrameOrigin::NativeCompleted {
            self.origin = FrameOrigin::ImportedCompleted;
        }
        Ok(())
    }
    fn healthy(&self) -> Result<(), CaptureError> {
        if self.phase == Phase::Poisoned {
            Err(CaptureError::Poisoned)
        } else {
            Ok(())
        }
    }
    /// Obtain exclusive capture access after checking bounded native journals,
    /// deferred ownership and unsupported history. Failure never clears reports.
    pub fn completed_step(
        &mut self,
        work: &mut WorkBudget,
    ) -> Result<CompletedStep<'_>, CaptureError> {
        self.healthy()?;
        if self.phase != Phase::Completed {
            return Err(CaptureError::NotCompleted);
        }
        let identity = self.validate_boundary(work)?;
        Ok(CompletedStep {
            owner: self,
            identity,
        })
    }
    fn validate_boundary(&mut self, work: &mut WorkBudget) -> Result<Identity, CaptureError> {
        let identity = self.identity?;
        if !Arc::ptr_eq(
            &self.dispatcher,
            &self.world.narrow_phase.checkpoint_dispatcher(),
        ) {
            return Err(CaptureError::Dispatcher);
        }
        let w = &mut self.world;
        let bodies = w.bodies.bodies.checkpoint_slot_count();
        let colliders = w.colliders.colliders.checkpoint_slot_count();
        if bodies > MAX_ITEMS || colliders > MAX_ITEMS {
            return Err(CaptureError::Limit);
        }
        work.charge((bodies as u64) + (colliders as u64) + 1)?;
        w.impulse_joints
            .checkpoint_empty_gate()
            .map_err(CaptureError::Family)?;
        w.multibody_joints
            .checkpoint_empty_gate()
            .map_err(CaptureError::Family)?;
        w.soft_bodies
            .checkpoint_empty_gate()
            .map_err(CaptureError::Family)?;
        let o = w
            .physics_pipeline
            .checkpoint_observation()
            .map_err(|_| CaptureError::DeferredPoison)?;
        if o.deferred_bvh_receiver || o.deferred_bvh_inline {
            return Err(CaptureError::Deferred);
        }
        if [
            o.quarantine_bodies,
            o.quarantine_colliders,
            o.quarantine_soft_bodies,
            o.quarantine_body_workspace,
            o.quarantine_collider_workspace,
        ] != [0; 5]
        {
            return Err(CaptureError::Quarantine);
        }
        if !w.bodies.modified_bodies.is_empty()
            || !w.colliders.modified_colliders.is_empty()
            || !w.colliders.removed_colliders.is_empty()
            || w.colliders.iter().any(|(_, c)| !c.changes.is_empty())
        {
            return Err(CaptureError::PendingMutation);
        }
        Ok(identity)
    }
}
impl CompletedStep<'_> {
    /// Identity bound to this exclusive native owner.
    pub fn identity(&self) -> Identity {
        self.identity
    }
    /// Immutable native lookups while the same completed-step borrow is held.
    pub fn get(&self) -> &PhysicsWorld {
        &self.owner.world
    }
}
#[cfg(test)]
mod tests;
