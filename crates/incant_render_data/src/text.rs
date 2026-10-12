//! Retained CPU text input. Layout coordinates are logical pixels, with no
//! implied widget/viewport/appearance policy. GPU text drawing is not implemented.
use glam::DMat4;
use incant_text::{GlyphAtlas, GlyphHandle, ShapedText};
use incant_types::StableId;
use std::sync::Arc;

#[derive(Clone)]
pub struct TextNode {
    pub id: StableId,
    pub world: DMat4,
    pub layout: Arc<ShapedText>,
    /// One checked handle per shaped glyph, including empty/missing outlines.
    pub glyphs: Arc<[GlyphHandle]>,
}
#[derive(Clone)]
pub struct TextInput {
    pub nodes: Vec<TextNode>,
    pub atlas: Arc<GlyphAtlas>,
}
