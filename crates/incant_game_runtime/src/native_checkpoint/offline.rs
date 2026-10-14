use super::*;
use crate::{Core, NoHost, OfflineGameSession, offline::OfflineCore};
use incant_audio_runtime::ClipBinding;

impl OfflineGameSession {
    /// Capture only after an accepted complete tick, including rendered audio.
    /// Physics budgets exclude bounded script, UI and audio checkpoint storage.
    pub fn checkpoint(
        &mut self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<GameCheckpoint> {
        self.inner.core.checkpoint(
            Some(self.inner.sample_rate),
            payload,
            planar_work,
            spatial_work,
        )
    }

    /// Restore a separate candidate using the captured output sample clock.
    /// Audio catalogs are validated before host preparation or guest startup.
    /// No live game, physical input queue or device resource is replaced here.
    pub fn from_checkpoint(
        pending: PendingGameCheckpoint,
        clips: Vec<ClipBinding>,
    ) -> Result<Self> {
        let sample_rate = pending.offline_rate()?;
        crate::offline::validate_rate(sample_rate)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::from_pending_offline_checkpoint(
                    pending.native,
                    |state| {
                        crate::require_nonspatial_world(state.world())?;
                        Ok(NoHost)
                    },
                    clips,
                    sample_rate,
                )?
                .with_checkpoint_suspension(pending.suspended)?,
                sample_rate,
            },
        })
    }
}
