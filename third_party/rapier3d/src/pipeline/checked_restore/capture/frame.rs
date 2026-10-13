//! Only an exclusive owner issues this in-memory frame. There is no importer,
//! caller authenticity flag, Deserialize implementation or public raw setter.
use super::super::{WorkBudget, world};
use super::{CaptureError, CheckpointOwner, compiled_identity};
use alloc::vec::Vec;
use incant_physics_snapshot::{Budget, Identity, construction::AllocationBudget};
/// Opaque completed-state capture bound to the actual linked implementation.
/// `encoded` is inspectable data, not a capability to admit arbitrary bytes.
/// Thread-pool ownership is explicitly retained in memory and not serialized.
pub struct NativeFrame {
    pub(super) identity: Identity,
    pub(super) bytes: Vec<u8>,
    pub(super) context: world::CaptureContext,
}
impl NativeFrame {
    /// Actual compiled identity of the exclusive source owner.
    pub fn identity(&self) -> Identity {
        self.identity
    }
    /// Immutable captured envelope. No corresponding public from-bytes exists.
    pub fn encoded(&self) -> &[u8] {
        &self.bytes
    }
    /// Stage a complete candidate under cumulative decode, work and allocation
    /// budgets. This never replaces a caller's live owner or outer runtime.
    pub fn reconstruct(
        &self,
        decode: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<CheckpointOwner, CaptureError> {
        let actual = compiled_identity()?;
        if actual != self.identity {
            return Err(CaptureError::Build);
        }
        let candidate = world::restore_captured(
            &self.bytes,
            actual,
            &self.context,
            decode,
            work,
            allocations,
        )?;
        Ok(CheckpointOwner::restored(candidate, actual))
    }
}
