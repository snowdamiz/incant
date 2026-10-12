//! Fixed-tick UI replay on the same composed offline game. Recorded commands
//! deliberately target its current owner; no serialized platform token is trusted.
use crate::session::{Player, Result};
use incant_input::{InputEvent, RecordedUiEvent};
#[cfg(feature = "render-input")]
fn events(
    game: &incant_ui::UiRuntime,
    records: &[RecordedUiEvent],
) -> Result<Vec<incant_ui::UiEvent>> {
    use incant_ui::UiEvent;
    let target = |id: &str| -> Result<_> {
        let token = game
            .focus()
            .ok_or("recorded UI text requires established focus")?;
        if ulid::Ulid::from_bytes(token.widget().0).to_string() != id {
            return Err("recorded UI text target differs from current focus".into());
        }
        Ok(token)
    };
    records
        .iter()
        .map(|event| {
            Ok(match event {
                RecordedUiEvent::Focus { entity_id } => UiEvent::Focus(incant_runtime::StableId(
                    entity_id.parse::<ulid::Ulid>()?.to_bytes(),
                )),
                RecordedUiEvent::Activate => UiEvent::Activate,
                RecordedUiEvent::Traverse { reverse } => UiEvent::Traverse { reverse: *reverse },
                RecordedUiEvent::Commit { entity_id, text } => UiEvent::Commit {
                    target: target(entity_id)?,
                    text: text.clone(),
                },
                RecordedUiEvent::Preedit {
                    entity_id,
                    text,
                    cursor,
                } => UiEvent::Preedit {
                    target: target(entity_id)?,
                    text: text.clone(),
                    cursor: cursor.map(|[a, b]| a..b),
                },
                RecordedUiEvent::Cancel { entity_id } => UiEvent::Cancel {
                    target: target(entity_id)?,
                },
                RecordedUiEvent::Pointer { position, pressed } => UiEvent::Pointer {
                    position: *position,
                    pressed: *pressed,
                    target: game
                        .pointer_token()
                        .ok_or("recorded pointer has no displayed layout")?,
                },
            })
        })
        .collect()
}
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
            let owner = game
                .host()?
                .ui()
                .ok_or("input recording requires a retained UI owner")?;
            // Validate focus-dependent records before changing presentation.
            // Timeline commands address the displayed layout after this
            // explicit resize; platform receipts have a different stale policy.
            let mut resolved = events(owner, records)?;
            if let Some(viewport) = viewport {
                game.resize_ui(viewport)?;
                resolved = events(game.host()?.ui().ok_or("UI owner invariant")?, records)?;
            }
            game.tick_with_ui(physical, &resolved, output)?;
            return Ok(());
        }
        Err("UI input requires the existing CPU composed host; GPU/window/static routes are unsupported".into())
    }
    pub fn validate_ui_recording(&self, uses_ui: bool) -> Result<()> {
        if !uses_ui {
            return Ok(());
        }
        #[cfg(feature = "render-input")]
        if let Self::Spatial(game) = self
            && game.host()?.ui().is_some()
        {
            return Ok(());
        }
        Err("UI recording requires an explicitly configured CPU UI host".into())
    }
}
