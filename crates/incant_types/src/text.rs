//! Numeric text declarations shared by authoring and native cooked data.
use crate::{Id, StableId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_TEXT_FONTS: usize = 32;
pub const MAX_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_TEXT_NODES: usize = 1024;
pub const MAX_SCENE_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextWrap {
    None,
    Word,
    WordOrGlyph,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextLayout {
    #[schemars(range(min = 1, max = 512))]
    pub font_size: f32,
    #[schemars(range(min = 1, max = 4096))]
    pub line_height: f32,
    #[schemars(range(min = 1, max = 65536))]
    pub width: f32,
    pub wrap: TextWrap,
}
impl TextLayout {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.font_size.is_finite()
            || !(1.0..=512.0).contains(&self.font_size)
            || !self.line_height.is_finite()
            || !(1.0..=4096.0).contains(&self.line_height)
            || !self.width.is_finite()
            || !(1.0..=65536.0).contains(&self.width)
        {
            return Err("invalid text layout metrics");
        }
        Ok(())
    }
}

/// Literal text with an explicit ordered font chain. Localization can supply
/// future updates; this declaration does not guess locale keys or host fonts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Text {
    #[schemars(length(max = 16384))]
    pub content: String,
    #[schemars(length(min = 1, max = 32))]
    pub fonts: Vec<Id>,
    #[schemars(range(min = 1, max = 512))]
    pub font_size: f32,
    #[schemars(range(min = 1, max = 4096))]
    pub line_height: f32,
    #[schemars(range(min = 1, max = 65536))]
    pub width: f32,
    pub wrap: TextWrap,
}
impl Text {
    pub fn layout(&self) -> TextLayout {
        TextLayout {
            font_size: self.font_size,
            line_height: self.line_height,
            width: self.width,
            wrap: self.wrap,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        self.layout().validate()?;
        if self.content.len() > MAX_TEXT_BYTES
            || self.fonts.is_empty()
            || self.fonts.len() > MAX_TEXT_FONTS
            || self
                .fonts
                .iter()
                .enumerate()
                .any(|(i, id)| self.fonts[..i].contains(id))
        {
            return Err("invalid text content or font chain");
        }
        Ok(())
    }
}

/// Immutable startup font recipe binding; the actual reader verifies its data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontAssetRef {
    pub id: StableId,
    pub fingerprint: [u8; 32],
}
