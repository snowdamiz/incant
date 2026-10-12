//! Fixed-tick UI replay on the same composed offline game. Recorded commands
//! deliberately target its current owner; no serialized platform token is trusted.
use crate::session::{Player, Result};
use incant_input::{InputEvent, RecordedUiEvent};
impl Player {
    pub fn tick_recorded(
        &mut self,
        physical: &[InputEvent],
        records: &[RecordedUiEvent],
        viewport: Option<[f32; 2]>,
        output: &mut [f32],
    ) -> Result<()> {
        if records.is_empty() && viewport.is_none() {
            return self.tick(physical, output);
        }
        #[cfg(feature = "render-input")]
        if let Self::Spatial(game) = self {
            incant_render_game::tick_recorded_ui(game, physical, records, viewport, output)?;
            return Ok(());
        }
        Err("UI input requires the existing CPU composed host".into())
    }
    pub fn validate_ui_recording(&self, uses_ui: bool) -> Result<()> {
        if !uses_ui {
            return Ok(());
        }
        #[cfg(feature = "render-input")]
        if let Self::Spatial(game) = self {
            incant_render_game::validate_ui_recording(game, uses_ui)?;
            return Ok(());
        }
        Err("UI recording requires an explicitly configured CPU UI host".into())
    }
}
