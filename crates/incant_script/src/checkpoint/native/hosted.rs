//! Capture retained native owners; their bounded sections never authenticate a world.
use super::*;

impl NativeSession {
    /// Capture the exact committed UI owner with opaque physics and script data.
    /// An unrelated or stale world/tick owner refuses before native capture.
    #[cfg(feature = "ui")]
    pub fn checkpoint_with_ui(
        &mut self,
        ui: &incant_ui::UiRuntime,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<NativeSessionCheckpoint> {
        state::receiver(self)?;
        let ui = ui.checkpoint_for_world(&self.world)?;
        ui.validate_world(&self.world)?;
        self.capture_native(
            self.world.requires_ui_checkpoint().then_some(ui.bytes()),
            None,
            payload,
            planar_work,
            spatial_work,
        )
    }

    /// Capture settled offline audio, optional UI, physics and explicit script
    /// data. Device audio retains the existing typed refusal; no output rewinds.
    /// The audio section must match the world's exact rational sample clock.
    #[cfg(feature = "audio")]
    pub fn checkpoint_with_audio<B: incant_audio_runtime::AudioBackend>(
        &mut self,
        audio: &incant_audio_runtime::SceneAudio<B>,
        #[cfg(feature = "ui")] ui: Option<&incant_ui::UiRuntime>,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<NativeSessionCheckpoint>
    where
        B::Error: std::fmt::Debug,
    {
        state::receiver(self)?;
        let audio = audio.checkpoint_for_world(&self.world)?;
        audio.validate_session(&self.world)?;
        #[cfg(feature = "ui")]
        let ui = ui
            .map(|ui| ui.checkpoint_for_world(&self.world))
            .transpose()?;
        #[cfg(feature = "ui")]
        let ui = ui
            .as_ref()
            .filter(|_| self.world.requires_ui_checkpoint())
            .map(|ui| ui.bytes());
        #[cfg(not(feature = "ui"))]
        let ui = None;
        self.capture_native(ui, Some(audio.bytes()), payload, planar_work, spatial_work)
    }
}
