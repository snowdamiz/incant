//! Screen-space game UI draw input. Logical pixels, top-left origin, Y down.
//! The native UI owner supplies accepted layout, shaped glyphs, retained atlas
//! handles and widget state; the renderer owns appearance and pixel mapping
//! (incant_render `ui` module). No layout, shaping, focus or input logic here.
use incant_text::{GlyphAtlas, GlyphHandle, ShapedText, editing::Selection};
use incant_types::StableId;
use std::{ops::Range, sync::Arc};

/// Logical rectangle as `[left, top, right, bottom]`.
pub type UiRect = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiRole {
    Label,
    Button,
    TextInput,
}

/// TextInput editing state in UTF-8 bytes of the widget's `layout.source()`,
/// which is the buffer's display text (committed text plus any composition).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiEdit {
    /// `TextBuffer::display_selection()`: the ordinary selection, or the input
    /// method's cursor inside an active composition. None hides the caret.
    pub selection: Option<Selection>,
    /// Active composition span, `Preedit::display_range`.
    pub preedit: Option<Range<usize>>,
}

/// One text-bearing widget in canonical paint order. Root and Container nodes
/// have no appearance and are omitted.
#[derive(Clone)]
pub struct UiWidget {
    pub id: StableId,
    pub role: UiRole,
    /// Includes disabled ancestors.
    pub disabled: bool,
    pub focused: bool,
    /// A pointer press captured by this widget and not yet released.
    pub pressed: bool,
    /// Border box, and its visible part after every ancestor clip.
    pub bounds: UiRect,
    pub visible: UiRect,
    /// Content-box origin of the shaped layout, and the effective text clip
    /// (ancestor clips intersected with this widget's own content box).
    pub origin: [f32; 2],
    pub clip: UiRect,
    pub layout: Arc<ShapedText>,
    /// One handle per shaped glyph, bound to `UiInput::atlas`.
    pub glyphs: Arc<[GlyphHandle]>,
    /// Present for TextInput widgets only.
    pub edit: Option<UiEdit>,
}

/// One accepted or candidate UI frame. Identity (the Arc holding this value)
/// is the renderer's retention key; equal content in another Arc is new input.
pub struct UiInput {
    /// Logical viewport `[width, height]` the layout was computed for.
    pub viewport: [f32; 2],
    pub atlas: Arc<GlyphAtlas>,
    pub widgets: Vec<UiWidget>,
}
