//! Operation-boundary fences for generation and island removal arithmetic.
//! Immutable preflight supports an all-scenes pass before outer maps change.
//! The ordered permit additionally binds one scene's execution to that plan.
use super::super::WorkBudget;
use super::{CaptureError, CheckpointOwner};
use crate::{
    dynamics::{RigidBody, RigidBodyHandle},
    geometry::{Collider, ColliderHandle},
};
use alloc::sync::Arc;
use incant_physics_snapshot::MAX_ITEMS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Unique sorted native operations. A body owns deletion of its attachments.
pub enum StructuralRemoval {
    /// Removes this body and all of its attached colliders.
    Body(RigidBodyHandle),
    /// Removes exactly this collider and preserves its optional parent body.
    Collider(ColliderHandle),
}
impl StructuralRemoval {
    /// Canonical plan order, independent of the native map implementation.
    pub fn key(self) -> (u8, u32, u32) {
        match self {
            Self::Body(h) => {
                let (i, g) = h.into_raw_parts();
                (0, i, g)
            }
            Self::Collider(h) => {
                let (i, g) = h.into_raw_parts();
                (1, i, g)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Peak insertion counts, admitted independently of removal ordering.
pub struct StructuralInsertions {
    /// Number of new body slots.
    pub bodies: usize,
    /// Number of new collider slots.
    pub colliders: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Typed refusal before a planned mutation. No exhaustion is disguised as None.
pub enum StructuralError {
    /// Underlying owner lifecycle or unsupported family.
    Owner(CaptureError),
    /// Cumulative logical work budget exhausted.
    Work,
    /// Operation count exceeds the format bounds.
    Limit,
    /// Plan is not strictly sorted and unique.
    Order,
    /// A required native handle is absent.
    Missing,
    /// An explicit collider removal overlaps a removed body attachment.
    Overlap,
    /// Body generation or native slot-growth bound exhausted.
    BodyGenerationOrCapacity,
    /// Collider generation or native slot-growth bound exhausted.
    ColliderGenerationOrCapacity,
    /// Persistent island removal sums could overflow.
    IslandCounter,
    /// Wrong owner, stale plan, unexpected operation or incomplete execution.
    Plan,
}
type Result<T> = core::result::Result<T, StructuralError>;
impl CheckpointOwner {
    /// Check a complete sorted, unique removal plan and maximum insertion counts
    /// without altering this owner, its phase, or any native/outer journal.
    /// All-scene callers must finish every preflight before applying any edit.
    /// The budget is cumulative even when a later scene refuses.
    pub fn preflight_structure<'p>(
        &self,
        removals: &'p [StructuralRemoval],
        insertions: StructuralInsertions,
        work: &mut WorkBudget,
    ) -> Result<StructuralPlan<'p>> {
        let w = self.get().map_err(StructuralError::Owner)?;
        if removals.len() > MAX_ITEMS
            || insertions.bodies > MAX_ITEMS
            || insertions.colliders > MAX_ITEMS
        {
            return Err(StructuralError::Limit);
        }
        // This covers every plan comparison and binary-search comparison before
        // examining entries. Attached collider count is charged before walking it.
        let search = usize::BITS - removals.len().leading_zeros();
        work.charge(1 + (removals.len() as u64) * u64::from(search + 2))
            .map_err(|_| StructuralError::Work)?;
        if removals.windows(2).any(|r| r[0].key() >= r[1].key()) {
            return Err(StructuralError::Order);
        }
        w.impulse_joints
            .checkpoint_empty_gate()
            .map_err(|e| StructuralError::Owner(CaptureError::Family(e)))?;
        w.multibody_joints
            .checkpoint_empty_gate()
            .map_err(|e| StructuralError::Owner(CaptureError::Family(e)))?;
        w.soft_bodies
            .checkpoint_empty_gate()
            .map_err(|e| StructuralError::Owner(CaptureError::Family(e)))?;
        let mut bodies = 0usize;
        let mut colliders = 0usize;
        for removal in removals {
            match *removal {
                StructuralRemoval::Body(h) => {
                    let b = w.bodies.get(h).ok_or(StructuralError::Missing)?;
                    bodies += 1;
                    work.charge(b.colliders().len() as u64)
                        .map_err(|_| StructuralError::Work)?;
                    colliders = colliders
                        .checked_add(b.colliders().len())
                        .ok_or(StructuralError::Limit)?;
                    for c in b.colliders() {
                        if !w.colliders.contains(*c) {
                            return Err(StructuralError::Missing);
                        }
                    }
                }
                StructuralRemoval::Collider(h) => {
                    let c = w.colliders.get(h).ok_or(StructuralError::Missing)?;
                    if let Some(parent) = c.parent() {
                        let key = StructuralRemoval::Body(parent).key();
                        if removals.binary_search_by_key(&key, |r| r.key()).is_ok() {
                            return Err(StructuralError::Overlap);
                        }
                    }
                    colliders = colliders.checked_add(1).ok_or(StructuralError::Limit)?;
                }
            }
        }
        if !w
            .bodies
            .bodies
            .checkpoint_structure_headroom(bodies, insertions.bodies)
        {
            return Err(StructuralError::BodyGenerationOrCapacity);
        }
        if !w
            .colliders
            .colliders
            .checkpoint_structure_headroom(colliders, insertions.colliders)
        {
            return Err(StructuralError::ColliderGenerationOrCapacity);
        }
        island_headroom(w, bodies as u64, work)?;
        let operations = removals
            .len()
            .checked_add(insertions.bodies)
            .and_then(|n| n.checked_add(insertions.colliders))
            .ok_or(StructuralError::Limit)?;
        self.mutation_revision
            .checked_add(operations as u64)
            .ok_or(StructuralError::Owner(CaptureError::MutationExhausted))?;
        Ok(StructuralPlan {
            dispatcher: self.dispatcher.clone(),
            revision: self.mutation_revision,
            removals,
            insertions,
        })
    }
    /// Bind exclusive access to the checked plan. No cloning or detached permit
    /// can authorize another world. Dropping a partly consumed permit does not
    /// roll back already requested ordinary edits.
    pub fn structural_permit<'a, 'p>(
        &'a mut self,
        plan: StructuralPlan<'p>,
    ) -> Result<StructuralPermit<'a, 'p>> {
        self.get().map_err(StructuralError::Owner)?;
        if !Arc::ptr_eq(&self.dispatcher, &plan.dispatcher)
            || self.mutation_revision != plan.revision
        {
            return Err(StructuralError::Plan);
        }
        Ok(StructuralPermit {
            owner: self,
            removals: plan.removals,
            next: 0,
            insertions: plan.insertions,
        })
    }
    /// None denotes an absent handle only; exhaustion is a typed error before
    /// the first native mutation. Prefer one whole-batch preflight for outer edits.
    pub fn remove_body_checked(
        &mut self,
        h: RigidBodyHandle,
        work: &mut WorkBudget,
    ) -> Result<Option<RigidBody>> {
        if !self
            .get()
            .map_err(StructuralError::Owner)?
            .bodies
            .contains(h)
        {
            return Ok(None);
        }
        let removals = [StructuralRemoval::Body(h)];
        let plan = self.preflight_structure(&removals, StructuralInsertions::default(), work)?;
        self.structural_permit(plan)?.remove_body(h).map(Some)
    }
    /// Checked single removal; whole-batch users should consume one prepared plan.
    pub fn remove_collider_checked(
        &mut self,
        h: ColliderHandle,
        work: &mut WorkBudget,
    ) -> Result<Option<Collider>> {
        if !self
            .get()
            .map_err(StructuralError::Owner)?
            .colliders
            .contains(h)
        {
            return Ok(None);
        }
        let removals = [StructuralRemoval::Collider(h)];
        let plan = self.preflight_structure(&removals, StructuralInsertions::default(), work)?;
        self.structural_permit(plan)?.remove_collider(h).map(Some)
    }
}
/// Cumulative conservative sum also covers native eager island merges. Checking
/// each island separately would miss `big.count += small.count` overflow.
pub(super) fn island_headroom(
    w: &crate::pipeline::PhysicsWorld,
    extra: u64,
    work: &mut WorkBudget,
) -> Result<()> {
    // VecMap iteration walks retained slots, not just occupied entries.
    let capacity = w.islands.persistent.islands.capacity();
    if capacity > MAX_ITEMS {
        return Err(StructuralError::IslandCounter);
    }
    work.charge(capacity as u64 + 1)
        .map_err(|_| StructuralError::Work)?;
    let sum = w
        .islands
        .persistent
        .islands
        .iter()
        .try_fold(extra, |sum, (_, i)| {
            sum.checked_add(u64::from(i.constraint_remove_count))
                .ok_or(StructuralError::IslandCounter)
        })?;
    if sum > u64::from(u32::MAX) {
        return Err(StructuralError::IslandCounter);
    }
    Ok(())
}
/// Detached immutable preflight, bound to one owner and exact mutation revision.
/// It may be retained while other scenes are preflighted. No native state is
/// borrowed, and the retained Arc prevents allocator-address reuse from forging
/// owner identity. It is deliberately non-Clone and has no public fields.
pub struct StructuralPlan<'p> {
    dispatcher: super::owner::Dispatcher,
    revision: u64,
    removals: &'p [StructuralRemoval],
    insertions: StructuralInsertions,
}
/// Exclusive execution of one admitted plan, without new work-budget charges.
/// All removals precede insertions, so a newly attached collider cannot change a
/// yet-to-be-removed body's generation debit. Unexpected operations refuse.
pub struct StructuralPermit<'a, 'p> {
    owner: &'a mut CheckpointOwner,
    removals: &'p [StructuralRemoval],
    next: usize,
    insertions: StructuralInsertions,
}
impl StructuralPermit<'_, '_> {
    fn removal(&mut self, value: StructuralRemoval) -> Result<&mut crate::pipeline::PhysicsWorld> {
        if self.removals.get(self.next) != Some(&value) {
            return Err(StructuralError::Plan);
        }
        self.next += 1;
        self.owner.access_mut().map_err(StructuralError::Owner)
    }
    /// Consume the next planned body removal.
    pub fn remove_body(&mut self, h: RigidBodyHandle) -> Result<RigidBody> {
        self.removal(StructuralRemoval::Body(h))?
            .remove_body(h)
            .ok_or(StructuralError::Missing)
    }
    /// Consume the next planned collider removal.
    pub fn remove_collider(&mut self, h: ColliderHandle) -> Result<Collider> {
        self.removal(StructuralRemoval::Collider(h))?
            .remove_collider(h)
            .ok_or(StructuralError::Missing)
    }
    /// Consume one insertion after all planned removals.
    pub fn insert_body(&mut self, body: impl Into<RigidBody>) -> Result<RigidBodyHandle> {
        if self.next != self.removals.len() || self.insertions.bodies == 0 {
            return Err(StructuralError::Plan);
        }
        self.insertions.bodies -= 1;
        Ok(self
            .owner
            .access_mut()
            .map_err(StructuralError::Owner)?
            .insert_body(body))
    }
    /// Consume one collider insertion, checking its parent still exists.
    pub fn insert_collider(
        &mut self,
        collider: impl Into<Collider>,
        parent: Option<RigidBodyHandle>,
    ) -> Result<ColliderHandle> {
        if self.next != self.removals.len() || self.insertions.colliders == 0 {
            return Err(StructuralError::Plan);
        }
        if parent.is_some_and(|p| !self.owner.get().is_ok_and(|w| w.bodies.contains(p))) {
            return Err(StructuralError::Missing);
        }
        self.insertions.colliders -= 1;
        Ok(self
            .owner
            .access_mut()
            .map_err(StructuralError::Owner)?
            .insert_collider(collider, parent))
    }
    /// Require every planned operation to have been consumed.
    pub fn finish(self) -> Result<()> {
        if self.next != self.removals.len() || self.insertions != StructuralInsertions::default() {
            return Err(StructuralError::Plan);
        }
        Ok(())
    }
}
#[cfg(all(test, feature = "serde-serialize"))]
mod tests;
