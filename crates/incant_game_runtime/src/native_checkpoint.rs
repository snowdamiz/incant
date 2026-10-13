//! Process-local game checkpoints with explicit host preparation before startup.
use crate::{GameError, Result};
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget,
};
use incant_script::{NativeSessionCheckpoint, PendingSessionCheckpoint};
mod hosted;
mod offline;
mod persisted;
pub use persisted::MAX_PERSISTED_GAME_BYTES;

/// Opaque in-memory physics, opted-in script data and admitted UI/audio state.
/// Offline sample rate and game suspension belong to this enclosing owner.
/// Data-file export is separately validated; byte loading creates a pending game.
/// Arbitrary guest heaps and device transport restore remain unsupported.
pub struct GameCheckpoint {
    pub(crate) native: NativeSessionCheckpoint,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) suspended: bool,
}

/// A private fresh world with typed host data, before any guest initialization.
/// Construct a complete game through its `from_checkpoint` factory. The previous
/// game stays untouched until the caller explicitly replaces it with the result.
pub struct PendingGameCheckpoint {
    pub(crate) native: PendingSessionCheckpoint,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) suspended: bool,
}

impl GameCheckpoint {
    /// Reconstruct and validate all captured native data without creating hosts,
    /// evaluating guest code or changing the source game. Physical input resets
    /// neutral on startup; external OS events and old host queues must be dropped.
    pub fn prepare(
        &self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<PendingGameCheckpoint> {
        Ok(PendingGameCheckpoint {
            native: self
                .native
                .prepare(payload, planar_work, spatial_work, allocations)?,
            sample_rate: self.sample_rate,
            suspended: self.suspended,
        })
    }
}

impl PendingGameCheckpoint {
    /// Original offline output clock, or None for a backend-driven game.
    pub fn sample_rate(&self) -> Option<u32> {
        self.sample_rate
    }

    pub(crate) fn offline_rate<E>(&self) -> Result<u32, E> {
        self.sample_rate.ok_or(GameError::Checkpoint(
            incant_script::CheckpointError::Unsupported(
                "offline checkpoint requires a captured sample rate",
            ),
        ))
    }

    pub(crate) fn require_backend<E>(&self) -> Result<(), E> {
        if self.sample_rate.is_some() {
            return Err(GameError::Checkpoint(
                incant_script::CheckpointError::Unsupported(
                    "offline checkpoint requires an offline game",
                ),
            ));
        }
        Ok(())
    }
}

impl<B: kira::backend::Backend> crate::GameSession<B>
where
    B::Error: std::fmt::Debug,
{
    pub fn checkpoint(
        &mut self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<GameCheckpoint> {
        self.core
            .checkpoint(None, payload, planar_work, spatial_work)
    }

    /// Prepare a separate candidate from a backend-driven checkpoint. Existing
    /// audio transport restore policy still applies; offline saves require the
    /// offline factory. Failed preparation leaves the previous game untouched.
    pub fn from_checkpoint(
        pending: PendingGameCheckpoint,
        clips: Vec<incant_audio_runtime::ClipBinding>,
        settings: incant_audio_runtime::AudioManagerSettings<B>,
    ) -> Result<Self> {
        pending.require_backend()?;
        Ok(Self {
            core: crate::Core::from_pending_checkpoint(
                pending.native,
                |state| {
                    crate::require_nonspatial_world(state.world())?;
                    Ok(crate::NoHost)
                },
                |world| incant_audio_runtime::SceneAudio::new(world, clips, settings),
            )?
            .with_checkpoint_suspension(pending.suspended)?,
        })
    }
}
