//! Runtime game text shaping using only explicitly supplied font data.
#![forbid(unsafe_code)]

mod fonts;
mod layout;

pub use fonts::{FontSource, MAX_FONT_BYTES, MAX_FONTS, MAX_TOTAL_FONT_BYTES};
pub use layout::{Glyph, Line, MAX_TEXT_BYTES, ShapedText, TextEngine, TextStyle, Wrap};

/// Borrowed admission check for one packaged sfnt face. No font names, system
/// fonts or decoded-name allocations are involved. Uses the same pinned face
/// parser as cosmic-text's actual rustybuzz loader; outlining/rasterization is
/// outside this shaping admission contract.
pub fn validate_font(bytes: &[u8]) -> Result<(), TextError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(TextError::Limit("font bytes"));
    }
    if !matches!(bytes.get(..4), Some(b"\0\x01\0\0" | b"OTTO" | b"true"))
        || rustybuzz::Face::from_slice(bytes, 0).is_none()
    {
        return Err(TextError::Invalid(
            "unsupported or malformed single-face sfnt",
        ));
    }
    Ok(())
}

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
