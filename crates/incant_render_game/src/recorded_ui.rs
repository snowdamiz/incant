//! Fixed-tick recorded UI on the existing composed game. Unlike asynchronous
//! platform receipts, timeline records resolve against the current UI owner.
use crate::{NativeRenderHost, RenderHostError, RenderInputConsumer};
use incant_game_runtime::{GameError, HostedOfflineGameSession};
use incant_input::{InputEvent, RecordedUiEvent};
use incant_ui::{UiEvent, UiRuntime};
use std::convert::Infallible;

#[derive(Debug, thiserror::Error)]
pub enum RecordedUiError<E = Infallible> {
    #[error("{0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Input(#[from] incant_input::InputError),
    #[error(transparent)]
    Game(#[from] GameError<RenderHostError<E>>),
}

/// Check admission before a run starts, even if its first UI event is later.
pub fn validate_ui_recording<C: RenderInputConsumer>(
    game: &HostedOfflineGameSession<NativeRenderHost<C>>,
    uses_ui: bool,
) -> Result<(), RecordedUiError<C::Error>> {
    if uses_ui && game.host()?.ui().is_none() {
        return Err(RecordedUiError::Invalid(
            "UI recording requires an explicitly configured CPU UI host",
        ));
    }
    Ok(())
}

fn events<E>(
    owner: &UiRuntime,
    records: &[RecordedUiEvent],
) -> Result<Vec<UiEvent>, RecordedUiError<E>> {
    let id = |text: &str| {
        text.parse::<ulid::Ulid>()
            .map(|v| incant_runtime::StableId(v.to_bytes()))
            .map_err(|_| RecordedUiError::Invalid("invalid recorded UI entity"))
    };
    let target = |text: &str| {
        let token = owner.focus().ok_or(RecordedUiError::Invalid(
            "recorded UI text requires established focus",
        ))?;
        if token.widget() != id(text)? {
            return Err(RecordedUiError::Invalid(
                "recorded UI text target differs from current focus",
            ));
        }
        Ok(token)
    };
    records
        .iter()
        .map(|event| {
            Ok(match event {
                RecordedUiEvent::Focus { entity_id } => UiEvent::Focus(id(entity_id)?),
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
                    target: owner.pointer_token().ok_or(RecordedUiError::Invalid(
                        "recorded pointer has no displayed layout",
                    ))?,
                },
            })
        })
        .collect()
}

/// Reuses ordinary player replay semantics: focus must precede text by a tick;
/// validate it before resize, then resolve pointer receipts against the resized
/// layout. Resize is a separate presentation transaction; no simulation advances
/// until the complete input packet is accepted. Guest failure is terminal.
pub fn tick_recorded_ui<C: RenderInputConsumer>(
    game: &mut HostedOfflineGameSession<NativeRenderHost<C>>,
    physical: &[InputEvent],
    records: &[RecordedUiEvent],
    viewport: Option<[f32; 2]>,
    output: &mut [f32],
) -> Result<(), RecordedUiError<C::Error>> {
    if records.is_empty() && viewport.is_none() {
        return Ok(game.tick(physical, output)?);
    }
    // Public callers may supply typed records directly, without InputReplay.
    // Charge all limits before cloning any text or changing presentation.
    if records.len() > incant_input::MAX_UI_NOTIFICATIONS
        || viewport.is_some_and(|v| {
            v.iter()
                .any(|n| !n.is_finite() || !(1.0..=65_536.0).contains(n))
        })
    {
        return Err(RecordedUiError::Input(incant_input::InputError::Ui));
    }
    let mut bytes = 0usize;
    for event in records {
        bytes = bytes
            .checked_add(event.validate()?)
            .filter(|n| *n <= incant_input::MAX_UI_NOTIFICATION_BYTES)
            .ok_or(incant_input::InputError::Ui)?;
    }
    let owner = game.host()?.ui().ok_or(RecordedUiError::Invalid(
        "input recording requires a retained UI owner",
    ))?;
    let mut resolved = events(owner, records)?;
    if let Some(viewport) = viewport {
        game.resize_ui(viewport)?;
        resolved = events(
            game.host()?
                .ui()
                .ok_or(RecordedUiError::Invalid("UI owner invariant"))?,
            records,
        )?;
    }
    Ok(game.tick_with_ui(physical, &resolved, output)?)
}
