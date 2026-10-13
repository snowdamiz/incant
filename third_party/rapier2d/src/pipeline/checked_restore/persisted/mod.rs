//! Checked editable bytes for a documented rigid subset. Imported data does not
//! prove historical completion. Publication of outer owners remains staged by
//! the caller; no native references, dispatcher or thread-pool pointers are read
//! from the envelope. Saved state is never normalized or speculatively stepped.
mod domain;
use super::{
    WorkBudget,
    capture::{CaptureError, CheckpointOwner, compiled_identity},
};
use incant_physics_snapshot::{Budget, construction::AllocationBudget};
/// Specific persisted-domain refusal. These restrictions supplement existing
/// inert structure/numeric checks; they do not apply to ordinary opaque captures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainError {
    /// Shared logical validation/collision work exhausted.
    Work,
    /// Existing derived arithmetic/coherence rejected the state.
    Numeric(super::AdmissionError),
    /// Hard/soft CCD or a retained fixed-target cache needs further admission.
    Ccd,
    /// Gyroscopic correction has a nonfinite intermediate.
    Gyroscopic,
    /// A solver-cluster point cannot yet be tied to a unique saved child.
    Cluster,
    /// A query-visible retained bound disagrees with geometry or child bounds.
    BroadBounds,
    /// Contact cardinality or ownership is outside the consumed native domain.
    Contact,
    /// A packed feature word is not emitted for its concrete closed shape.
    Feature,
    /// Effective dominance differs from the joined body types/groups.
    Dominance,
    /// The retained material differs from the exact current native combination.
    Material,
    /// A retained recycle cache could skip required current-shape work.
    Recycle,
    /// Fast workspace records are not in their defined canonical key order.
    WorkspaceOrder,
    /// Saved arithmetic exceeds the documented finite intermediate envelope.
    Arithmetic,
    /// Native removal/index arithmetic lacks conservative next-step headroom.
    Counter,
}
impl From<super::AdmissionError> for DomainError {
    fn from(e: super::AdmissionError) -> Self {
        Self::Numeric(e)
    }
}
type DomainResult<T> = core::result::Result<T, DomainError>;
/// Native persisted refusal before candidate publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistedError {
    /// Actual compiled compatibility or source capture lifecycle refused.
    Capture(CaptureError),
    /// Bounded inert decoding failed.
    Record(incant_physics_snapshot::Error),
    /// Existing numerical/derived coherence failed.
    Numeric(super::AdmissionError),
    /// Additional editable-data admission failed.
    Domain(DomainError),
    /// Private staged native allocation/construction failed.
    Construction(incant_physics_snapshot::construction::ConstructionError),
}
impl From<CaptureError> for PersistedError {
    fn from(e: CaptureError) -> Self {
        Self::Capture(e)
    }
}
impl From<incant_physics_snapshot::Error> for PersistedError {
    fn from(e: incant_physics_snapshot::Error) -> Self {
        Self::Record(e)
    }
}
impl From<DomainError> for PersistedError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}
type Result<T> = core::result::Result<T, PersistedError>;
impl CheckpointOwner {
    /// Decode, validate and construct an unpublished candidate with the actual
    /// linked native identity and trusted default dispatcher/execution pool.
    /// Its initial readiness is ImportedValidated, never CompletedStep.
    pub fn from_persisted(
        bytes: &[u8],
        payload: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<Self> {
        let identity = compiled_identity()?;
        let world = super::world::restore_persisted(bytes, identity, payload, work, allocations)?;
        Ok(Self::restored(
            world,
            identity,
            super::capture::FrameOrigin::ImportedValidated,
        ))
    }
}
pub(super) fn validate(
    bytes: &[u8],
    identity: incant_physics_snapshot::Identity,
    payload: &mut Budget,
    work: &mut WorkBudget,
) -> Result<()> {
    super::world::validate_persisted(bytes, identity, payload, work)
}
pub(super) fn admit(n: &super::NumericFrame<'_>, work: &mut WorkBudget) -> DomainResult<()> {
    domain::admit(n, work)
}
/// This sticky fence remains enabled after a loaded owner performs a genuine
/// step. A lifecycle transition cannot remove its operation-boundary safeguards.
pub(super) fn pre_step(w: &crate::pipeline::PhysicsWorld) -> DomainResult<()> {
    use incant_physics_snapshot::MAX_ITEMS;
    let bodies = w.bodies.bodies.checkpoint_slot_count();
    let colliders = w.colliders.colliders.checkpoint_slot_count();
    let islands = &w.islands.persistent.islands;
    if bodies > MAX_ITEMS || colliders > MAX_ITEMS || islands.capacity() > MAX_ITEMS {
        return Err(DomainError::Counter);
    }
    if w.bodies
        .iter()
        .any(|(_, b)| b.is_ccd_enabled() || b.is_ccd_active() || b.soft_ccd_prediction() != 0.)
    {
        return Err(DomainError::Ccd);
    }
    // Each modified endpoint can unlink/relink a touching pair independently
    // during user-change processing. The one non-CCD narrow update can unlink
    // it again. Include already queued removals; merges preserve the sum.
    let pending = w.islands.checkpoint_pending_removal_count();
    if pending > MAX_ITEMS {
        return Err(DomainError::Counter);
    }
    let pairs = (colliders as u64)
        .checked_mul(colliders.saturating_sub(1) as u64)
        .ok_or(DomainError::Counter)?
        / 2;
    let extra = pairs
        .checked_mul(3)
        .and_then(|n| n.checked_add(bodies as u64))
        .and_then(|n| n.checked_add(pending as u64))
        .ok_or(DomainError::Counter)?;
    let sum = islands.iter().try_fold(extra, |sum, (_, i)| {
        sum.checked_add(u64::from(i.constraint_remove_count))
            .ok_or(DomainError::Counter)
    })?;
    if sum > u64::from(u32::MAX) {
        return Err(DomainError::Counter);
    }
    Ok(())
}
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
mod tests;
