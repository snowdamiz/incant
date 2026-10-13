//! Exclusive in-memory checkpoint ownership. Persisted-byte admission and the
//! enclosing runtime's stable IDs/events are deliberately separate.
mod aggregate;
mod config;
mod fields;
mod frame;
mod identity;
mod owner;
pub(crate) mod util;
pub use super::world::OmittedOwnerError;
pub use frame::NativeFrame;
pub use identity::{CompiledMode, compiled_identity, compiled_mode};
pub use owner::{CheckpointOwner, CompletedStep};
/// Typed checkpoint refusal; native stepping is never silently repaired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureError {
    /// Actual selected implementation or linked dependency modes disagree.
    Build,
    /// A prior native step unwound; the owner remains quarantined.
    Poisoned,
    /// The owner has not completed a positive finite step since mutable access.
    NotCompleted,
    /// A native owner was substituted with a different dispatcher identity.
    Dispatcher,
    /// A joint/soft family carries unsupported persistent history.
    Family(OmittedOwnerError),
    /// Native deferred ownership is poisoned.
    DeferredPoison,
    /// Native deferred work still owns state.
    Deferred,
    /// Native quarantine reports/work are nonempty.
    Quarantine,
    /// Native user-change journals remain pending after the step.
    PendingMutation,
    /// A count, allocation or shared budget is outside its admitted bound.
    Limit,
    /// Existing inert validation or encoding rejected the exact records.
    Record(incant_physics_snapshot::Error),
    /// Numeric coherence or logical work preflight refused the frame.
    Numeric(super::AdmissionError),
    /// Exact staged native storage construction failed without publication.
    Construction(incant_physics_snapshot::construction::ConstructionError),
}
impl From<incant_physics_snapshot::Error> for CaptureError {
    fn from(e: incant_physics_snapshot::Error) -> Self {
        Self::Record(e)
    }
}
impl From<super::AdmissionError> for CaptureError {
    fn from(e: super::AdmissionError) -> Self {
        Self::Numeric(e)
    }
}

#[cfg(test)]
mod tests;

// Independent test observer returns inert bytes only: it cannot mint the
// completed-owner capability, an opaque NativeFrame or a restored live world.
#[cfg(test)]
pub(crate) fn observe_for_reference(
    world: &mut crate::pipeline::PhysicsWorld,
    identity: incant_physics_snapshot::Identity,
) -> Result<alloc::vec::Vec<u8>, CaptureError> {
    aggregate::encode(
        world,
        identity,
        &mut incant_physics_snapshot::Budget::new(incant_physics_snapshot::MAX_ALLOCATION).unwrap(),
        &mut super::WorkBudget::new(u64::MAX),
    )
}
