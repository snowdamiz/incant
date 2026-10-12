//! Trusted-timeline UI commands, distinct from asynchronous platform receipts.
//! Replay resolves a target against the current retained owner at its scheduled
//! tick. Commit/preedit require focus already established by a preceding tick.
use crate::{InputError, UiSnapshot};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecordedUiEvent {
    Focus {
        entity_id: String,
    },
    Activate,
    Traverse {
        reverse: bool,
    },
    Commit {
        entity_id: String,
        text: String,
    },
    Preedit {
        entity_id: String,
        text: String,
        cursor: Option<[usize; 2]>,
    },
    Cancel {
        entity_id: String,
    },
    Pointer {
        position: [f32; 2],
        pressed: bool,
    },
}
impl RecordedUiEvent {
    pub(crate) fn validate(&self) -> Result<usize, InputError> {
        let (id, text) = match self {
            Self::Focus { entity_id } | Self::Cancel { entity_id } => (Some(entity_id), ""),
            Self::Commit { entity_id, text }
            | Self::Preedit {
                entity_id, text, ..
            } => (Some(entity_id), text.as_str()),
            Self::Activate | Self::Traverse { .. } | Self::Pointer { .. } => (None, ""),
        };
        if let Some(id) = id {
            UiSnapshot {
                focused: Some(id.clone()),
                ..Default::default()
            }
            .validate()?;
        }
        if text.len() > 16 * 1024 {
            return Err(InputError::Ui);
        }
        if let Self::Preedit {
            cursor: Some([start, end]),
            ..
        } = self
            && (start > end
                || *end > text.len()
                || !text.is_char_boundary(*start)
                || !text.is_char_boundary(*end))
        {
            return Err(InputError::Ui);
        }
        if let Self::Pointer { position, .. } = self
            && position
                .iter()
                .any(|n| !n.is_finite() || n.abs() > 1_000_000.)
        {
            return Err(InputError::Ui);
        }
        Ok(text.len())
    }
}
