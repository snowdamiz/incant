//! Runtime game text shaping using only explicitly supplied font data.
#![forbid(unsafe_code)]

mod fonts;
mod layout;

pub use fonts::{FontSource, MAX_FONT_BYTES, MAX_FONTS, MAX_TOTAL_FONT_BYTES};
pub use layout::{Glyph, Line, MAX_TEXT_BYTES, ShapedText, TextEngine, TextStyle, Wrap};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TextError {
    #[error("text resource limit: {0}")]
    Limit(&'static str),
    #[error("invalid text input: {0}")]
    Invalid(&'static str),
    #[error("font {0} is invalid or unsupported; supply one TrueType/OpenType face")]
    Font(ulid::Ulid),
    #[error("text shaper returned invalid geometry")]
    Geometry,
}
