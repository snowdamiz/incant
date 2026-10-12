//! Game UI appearance: display-referred sRGB colours with straight alpha, and
//! metrics in logical pixels. One restrained dark surface family over the game,
//! light text, and a single accent reserved for focus and selection. No glow,
//! gradients or decorative edges; the field outline marks an editable region.
//! Roots and containers are layout only and draw nothing.

/// Primary text on any surface or directly over the scene.
pub const TEXT: [u8; 4] = [236, 238, 242, 255];
/// Disabled text keeps its hue and loses contrast through alpha.
pub const TEXT_DISABLED_ALPHA: u8 = 105;
/// Labels sit directly on the scene, so their glyphs get a hard one-pixel
/// shadow: invisible over dark scenes, legible over bright ones.
pub const LABEL_SHADOW: [u8; 4] = [0, 0, 0, 150];
pub const LABEL_SHADOW_OFFSET: [f32; 2] = [0., 1.];
/// Surfaces are opaque: the scene never shows through a control.
pub const BUTTON: [u8; 4] = [46, 50, 58, 255];
/// Pressed while the pointer press is captured: one step darker.
pub const BUTTON_PRESSED: [u8; 4] = [31, 34, 40, 255];
/// Disabled: flatter, closer to the backdrop, with muted text.
pub const BUTTON_DISABLED: [u8; 4] = [35, 38, 44, 255];
/// Field surface sits below buttons so editable regions read as inset.
pub const FIELD: [u8; 4] = [21, 23, 28, 255];
/// At least 3:1 against both the field and a dark backdrop (WCAG non-text).
pub const FIELD_BORDER: [u8; 4] = [100, 106, 119, 255];
pub const FIELD_BORDER_DISABLED: [u8; 4] = [54, 58, 66, 255];
/// Focus ring, selection tint and nothing else.
pub const ACCENT: [u8; 4] = [76, 146, 255, 255];
pub const SELECTION: [u8; 4] = [76, 146, 255, 110];
/// Composition underline uses the text colour.
pub const PREEDIT_ALPHA: u8 = 220;

pub const RADIUS: f32 = 4.0;
pub const BORDER_WIDTH: f32 = 1.0;
/// Drawn inside the border box, so it never overlaps neighbours or ancestors.
pub const FOCUS_WIDTH: f32 = 2.0;
pub const CARET_WIDTH: f32 = 1.0;
/// Underline offset below the baseline and thickness, as fractions of the
/// glyph font size; it clears CJK descent. Thickness is at least one pixel.
pub const UNDERLINE_OFFSET: f32 = 0.2;
pub const UNDERLINE_THICKNESS: f32 = 0.06;
/// The input method's converted clause is underlined at this multiple.
pub const CLAUSE_THICKNESS: f32 = 2.0;

pub(crate) fn alpha(color: [u8; 4], factor: f32) -> [u8; 4] {
    [
        color[0],
        color[1],
        color[2],
        (f32::from(color[3]) * factor).round() as u8,
    ]
}
