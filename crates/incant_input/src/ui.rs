//! Immutable, bounded notifications from the trusted composed UI router. These
//! are outcomes, not physical events or permission to address another UI owner.
use crate::InputError;
use schemars::JsonSchema;
use serde::Serialize;
pub const MAX_UI_NOTIFICATIONS: usize = 256;
pub const MAX_UI_NOTIFICATION_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiNotification {
    Activated { entity_id: String },
    ValueChanged { entity_id: String, value: String },
    Submitted { entity_id: String, value: String },
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, JsonSchema)]
pub struct UiSnapshot {
    pub focused: Option<String>,
    pub text_input: bool,
    pub events: Vec<UiNotification>,
    pub discarded_events: u16,
}
impl UiSnapshot {
    pub fn is_empty(&self) -> bool {
        self.focused.is_none()
            && !self.text_input
            && self.events.is_empty()
            && self.discarded_events == 0
    }
    pub fn validate(&self) -> Result<(), InputError> {
        if self.events.len() > MAX_UI_NOTIFICATIONS
            || usize::from(self.discarded_events) > MAX_UI_NOTIFICATIONS
            || self.text_input && self.focused.is_none()
        {
            return Err(InputError::Ui);
        }
        if self.focused.as_deref().is_some_and(|id| !valid_id(id)) {
            return Err(InputError::Ui);
        }
        let mut bytes = 0usize;
        for event in &self.events {
            let (id, value) = match event {
                UiNotification::Activated { entity_id } => (entity_id.as_str(), ""),
                UiNotification::ValueChanged { entity_id, value }
                | UiNotification::Submitted { entity_id, value } => {
                    (entity_id.as_str(), value.as_str())
                }
            };
            if !valid_id(id) || value.len() > 16 * 1024 {
                return Err(InputError::Ui);
            }
            bytes = bytes
                .checked_add(value.len())
                .filter(|n| *n <= MAX_UI_NOTIFICATION_BYTES)
                .ok_or(InputError::Ui)?;
        }
        Ok(())
    }
}
fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 26
        && (b'0'..=b'7').contains(&b[0])
        && b.iter()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c))
}
