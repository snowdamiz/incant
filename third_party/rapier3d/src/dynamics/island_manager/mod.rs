// Incant modification: default-off trusted sleep-scan checkpoint adapter; see INCANT_PATCH.md.
pub use manager::IslandManager;

pub(crate) use island::Island;
pub(crate) use persistent::{INVALID_ISLAND, ImpulseJointIslandEvent, PersistentIslands};
pub(crate) use substep_groups::SolveGroup;

mod global_split;
mod island;
mod local_split;
mod manager;
mod persistent;
mod sleep;
mod substep_groups;

#[cfg(feature = "incant-checkpoint")]
mod checkpoint;
#[cfg(feature = "incant-checkpoint")]
pub use checkpoint::{SleepScanCheckpoint, SleepScanCheckpointError};
