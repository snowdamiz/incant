//! Trusted same-owner continuation diagnostics. This is not a hostile decoder.
use super::IslandManager;
use crate::alloc_prelude::*;
pub(super) const MAX_SCAN_SLOTS: usize = 262_144;

/// Exact historical slots, including unused tail slots; do not zero or compact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SleepScanCheckpoint {
    /// Counter of the enclosing persistent-island owner.
    pub stamp: u32,
    /// Exact old counter and eligibility pairs, including retained tails.
    pub slots: Vec<(u32, bool)>,
}
/// Atomic diagnostic capture/install refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepScanCheckpointError {
    /// Owner has a pending scan or island removal journal.
    PendingScan,
    /// The supplied counter does not match the enclosing restored owner.
    Stamp,
    /// The slots do not cover the current island slot space.
    Length,
    /// The explicit bounded diagnostic limit was exceeded.
    Limit,
    /// The bounded copy could not be allocated.
    Allocation,
}
impl core::fmt::Display for SleepScanCheckpointError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid trusted sleep-scan checkpoint: {self:?}")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for SleepScanCheckpointError {}
impl IslandManager {
    /// Copies bounded stamp-sensitive scratch after a completed step. The caller
    /// must bind it to the exact same trusted serialized owner and solver build.
    pub fn checkpoint_sleep_scan(&self) -> Result<SleepScanCheckpoint, SleepScanCheckpointError> {
        self.persistent.checkpoint_sleep_scan()
    }
    /// Installs scratch atomically after restoring that exact trusted owner.
    /// Scalar/size checks do not authenticate the owner or admit untrusted saves.
    pub fn restore_sleep_scan(
        &mut self,
        checkpoint: SleepScanCheckpoint,
    ) -> Result<(), SleepScanCheckpointError> {
        self.persistent.restore_sleep_scan(checkpoint)
    }
}
