use super::*;
use crate::{
    Core, HostedGameSession, HostedOfflineGameSession, SceneHost, core::WithHost,
    offline::OfflineCore,
};
use incant_audio_runtime::{AudioManagerSettings, ClipBinding, SceneAudio};
use kira::backend::Backend;
use std::fmt::Debug;

fn prepare<H: SceneHost>(
    state: &PendingSessionCheckpoint,
    settings: H::Settings,
) -> Result<WithHost<H>, H::Error> {
    #[cfg(feature = "ui")]
    {
        H::prepare_checkpoint(state.world(), settings, state.ui()).map(WithHost)
    }
    #[cfg(not(feature = "ui"))]
    H::prepare(state.world(), settings)
        .map(WithHost)
        .map_err(GameError::Host)
}

impl<B: Backend, H: SceneHost> HostedGameSession<B, H>
where
    B::Error: Debug,
{
    pub fn checkpoint(
        &mut self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<GameCheckpoint, H::Error> {
        self.core
            .checkpoint(None, payload, planar_work, spatial_work)
    }

    /// Bind a new trusted host to the exact restored world before its guest VM.
    /// Backend-driven audio transport restore remains explicitly unsupported.
    pub fn from_checkpoint(
        pending: PendingGameCheckpoint,
        clips: Vec<ClipBinding>,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        pending.require_backend()?;
        Ok(Self {
            core: Core::from_pending_checkpoint(
                pending.native,
                |state| prepare(state, host_settings),
                |world| SceneAudio::new(world, clips, audio_settings),
            )?
            .with_checkpoint_suspension(pending.suspended)?,
        })
    }
}

impl<H: SceneHost> HostedOfflineGameSession<H> {
    pub fn checkpoint(
        &mut self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<GameCheckpoint, H::Error> {
        self.inner.core.checkpoint(
            Some(self.inner.sample_rate),
            payload,
            planar_work,
            spatial_work,
        )
    }

    /// Recreate audio and the trusted scene/UI host against one fresh world.
    /// Candidate refusal/unwind cannot mutate the previous game; trusted host
    /// preparation remains responsible for its own external side effects.
    pub fn from_checkpoint(
        pending: PendingGameCheckpoint,
        clips: Vec<ClipBinding>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        let sample_rate = pending.offline_rate()?;
        crate::offline::validate_rate(sample_rate)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::from_pending_offline_checkpoint(
                    pending.native,
                    |state| prepare(state, host_settings),
                    clips,
                    sample_rate,
                )?
                .with_checkpoint_suspension(pending.suspended)?,
                sample_rate,
            },
        })
    }
}
