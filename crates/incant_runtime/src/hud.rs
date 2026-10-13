//! Ordered gameplay controls for existing text declarations. Fonts, layout,
//! roles and hierarchy remain the validated cooked/native configuration.
use std::sync::Arc;

pub const MAX_HUD_COMMANDS: usize = 256;
pub const MAX_HUD_COMMAND_BYTES: usize = crate::MAX_SCENE_TEXT_BYTES;

#[derive(Debug, Clone, PartialEq)]
pub enum UiControl {
    /// Replace a label/button's literal text, or a text input's declared value.
    /// A changed input declaration resets its buffer during host reconciliation.
    Text(Arc<str>),
    /// Disables this subtree for input without changing its layout.
    Disabled(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextControl {
    Content(Arc<str>),
}

pub(crate) fn bytes(command: &crate::StructuralCommand) -> Option<usize> {
    use crate::StructuralCommand as C;
    match command {
        C::ControlUi {
            control: UiControl::Text(text),
            ..
        }
        | C::ControlText {
            control: TextControl::Content(text),
            ..
        } => Some(text.len()),
        C::ControlUi {
            control: UiControl::Disabled(_),
            ..
        } => Some(0),
        _ => None,
    }
}
