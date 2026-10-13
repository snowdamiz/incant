use super::*;
impl PendingSessionCheckpoint {
    /// Internal composition only: section parsers bind to this already
    /// reconstructed world's fresh identity before a host or VM can run.
    pub(in crate::checkpoint) fn attach_hosted(
        &mut self,
        ui: Option<&[u8]>,
        audio: Option<&[u8]>,
    ) -> Result<()> {
        if self.world.requires_ui_checkpoint() != ui.is_some() {
            return Err(CheckpointError::Unsupported("hosted UI state is required"));
        }
        if self.world.requires_audio_checkpoint() && audio.is_none() {
            return Err(CheckpointError::Unsupported(
                "retained audio owner is required",
            ));
        }
        #[cfg(feature = "ui")]
        {
            self.ui = ui
                .map(|bytes| incant_ui::PendingUiCheckpoint::new(&self.world, bytes))
                .transpose()?;
        }
        #[cfg(not(feature = "ui"))]
        if ui.is_some() {
            return Err(CheckpointError::Unsupported("hosted UI checkpoint feature"));
        }
        #[cfg(feature = "audio")]
        {
            self.audio = audio
                .map(|bytes| incant_audio_runtime::PendingAudioCheckpoint::new(&self.world, bytes))
                .transpose()?;
            if let Some(audio) = &self.audio {
                audio.validate_session(&self.world)?;
            }
        }
        #[cfg(not(feature = "audio"))]
        if audio.is_some() {
            return Err(CheckpointError::Unsupported(
                "hosted audio checkpoint feature",
            ));
        }
        Ok(())
    }
    pub(in crate::checkpoint) fn retain_restart_requests(&mut self, enabled: bool) {
        self.restart_requests = enabled;
    }
}
