//! Game UI appearance: display-referred sRGB colours with straight alpha, and
//! metrics in logical pixels. One restrained dark surface family over the game,
//! light text, and a single accent reserved for focus and selection. No glow,
//! gradients or decorative edges; the field outline marks an editable region.
//! Roots and containers are layout only and draw nothing.

/// Primary text on any surface or directly over the scene.
pub const TEXT: [u8; 4] = [236, 238, 242, 255];
/// Disabled text keeps its hue and loses contrast through alpha.
pub const TEXT_DISABLED_ALPHA: u8 = 112;
/// Button surface: neutral, nearly opaque so labels stay legible over bright scenes.
pub const BUTTON: [u8; 4] = [48, 52, 60, 240];
/// Pressed while the pointer press is captured: one step darker, still neutral.
pub const BUTTON_PRESSED: [u8; 4] = [30, 33, 39, 246];
/// Field surface sits below buttons so editable regions read as inset.
pub const FIELD: [u8; 4] = [20, 22, 27, 236];
pub const FIELD_BORDER: [u8; 4] = [88, 94, 106, 255];
/// Disabled surfaces and outlines are drawn at this fraction of their alpha.
pub const DISABLED_SURFACE: f32 = 0.55;
/// Focus ring, caret-independent selection tint and nothing else.
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
/// glyph font size; thickness is at least one logical pixel.
pub const UNDERLINE_OFFSET: f32 = 0.12;
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
