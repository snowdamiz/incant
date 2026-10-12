//! Read-only presentation state for drawing one prepared candidate or the
//! accepted owner. It borrows existing focus, pointer-capture and text-editing
//! state; nothing here changes focus, text, layout or routing semantics.
use crate::{LayoutFrame, UiError, UiPending, UiRuntime, Widget, focus::State};
use incant_text::editing::TextBuffer;

/// Aligned with `layout.nodes`, the canonical paint order of this exact frame.
pub struct UiVisual<'a> {
    widgets: &'a [Widget],
    state: &'a State,
    pub layout: &'a LayoutFrame,
}
#[derive(Clone, Copy)]
pub struct WidgetVisual<'a> {
    pub widget: &'a Widget,
    /// The current focus owner (cleared while the window is unfocused).
    pub focused: bool,
    /// A pointer press captured by this widget and not yet released.
    pub pressed: bool,
    /// TextInput editing state: display text, selection and composition.
    pub text: Option<&'a TextBuffer>,
}
impl<'a> UiVisual<'a> {
    pub fn len(&self) -> usize {
        self.layout.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.layout.nodes.is_empty()
    }
    /// `position` indexes `layout.nodes`; out-of-range positions return None.
    pub fn node(&self, position: usize) -> Option<WidgetVisual<'a>> {
        if position >= self.layout.nodes.len() {
            return None;
        }
        let i = self.layout.widget_index(position);
        Some(WidgetVisual {
            widget: &self.widgets[i],
            focused: self.state.focus == Some(i),
            pressed: self.state.pointer == Some(i),
            text: self.state.buffers[i].as_deref(),
        })
    }
}
impl UiPending {
    /// The candidate's own widgets/state, including a structural replacement.
    pub fn visual<'a>(&'a self, runtime: &'a UiRuntime) -> Result<UiVisual<'a>, UiError> {
        let (widgets, state) = self.prepared(runtime)?;
        Ok(UiVisual {
            widgets,
            state,
            layout: self.layout(),
        })
    }
}
impl UiRuntime {
    /// The accepted frame; None before the first committed layout.
    pub fn visual(&self) -> Option<UiVisual<'_>> {
        self.layout().map(|layout| UiVisual {
            widgets: self.widgets(),
            state: &self.state,
            layout,
        })
    }
}
