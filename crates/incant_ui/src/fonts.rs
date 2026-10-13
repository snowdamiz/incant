//! Shared packaged-font measurement. A failed candidate may change private
//! shaping scratch, but cannot mutate an accepted immutable ShapedText.
use crate::{MAX_UI_EXTENT, MeasureSpace, StableId, TextMeasure, UiError, Widget};
use incant_text::{ShapedText, SharedFontSource, TextEngine, TextStyle, Wrap};
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_UI_FONT_CHAINS: usize = 32;
pub struct UiFonts {
    sources: BTreeMap<StableId, SharedFontSource>,
    engines: BTreeMap<Vec<StableId>, TextEngine>,
    bytes: usize,
}
impl UiFonts {
    /// Sources must come from the checked package catalog. This additional
    /// boundary refuses duplicate IDs, invalid faces and excessive raw payload;
    /// bytes are shared with the world-text binding, never rediscovered by name.
    pub fn new(sources: Vec<SharedFontSource>, widgets: &[Widget]) -> Result<Self, UiError> {
        if sources.len() > incant_text::MAX_FONTS || widgets.len() > crate::MAX_UI_NODES {
            return Err(UiError::Limit("font or widget count"));
        }
        let mut bytes = 0usize;
        let mut fonts = BTreeMap::new();
        for source in sources {
            let id = StableId(source.id.to_bytes());
            if fonts.contains_key(&id) {
                return Err(UiError::Invalid("duplicate font asset"));
            }
            bytes = bytes
                .checked_add(source.bytes.len())
                .filter(|n| *n <= incant_text::MAX_TOTAL_FONT_BYTES)
                .ok_or(UiError::Limit("font bytes"))?;
            incant_text::validate_font(&source.bytes)?;
            fonts.insert(id, source);
        }
        let mut engines = BTreeMap::new();
        for widget in widgets {
            widget.node.validate().map_err(UiError::Invalid)?;
            let Some(text) = widget.node.kind.text() else {
                continue;
            };
            if engines.contains_key(&text.fonts) {
                continue;
            }
            if engines.len() == MAX_UI_FONT_CHAINS {
                return Err(UiError::Limit("font chains"));
            }
            let chain = text
                .fonts
                .iter()
                .map(|id| {
                    fonts
                        .get(id)
                        .cloned()
                        .ok_or(UiError::Invalid("missing packaged font"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            engines.insert(text.fonts.clone(), TextEngine::new_shared(chain)?);
        }
        Ok(Self {
            sources: fonts,
            engines,
            bytes,
        })
    }
    pub(crate) fn raster(
        &self,
        widget: &Widget,
        layout: &ShapedText,
        atlas: &mut incant_text::GlyphAtlas,
    ) -> Result<Vec<incant_text::GlyphHandle>, UiError> {
        let text = widget
            .node
            .kind
            .text()
            .ok_or(UiError::Invalid("widget has no text"))?;
        let engine = self
            .engines
            .get(&text.fonts)
            .ok_or(UiError::Invalid("unbound font chain"))?;
        (0..layout.glyphs().len())
            .map(|i| engine.cache_glyph(layout, i, atlas).map_err(Into::into))
            .collect()
    }
    /// One prepared halo per shaped glyph, at the radius `rule` gives that
    /// glyph's font size: dilated once into the atlas here, never per frame.
    pub(crate) fn raster_halos(
        &self,
        widget: &Widget,
        layout: &ShapedText,
        atlas: &mut incant_text::GlyphAtlas,
        rule: incant_text::HaloRule,
    ) -> Result<Vec<incant_text::GlyphHandle>, UiError> {
        let text = widget
            .node
            .kind
            .text()
            .ok_or(UiError::Invalid("widget has no text"))?;
        let engine = self
            .engines
            .get(&text.fonts)
            .ok_or(UiError::Invalid("unbound font chain"))?;
        layout
            .glyphs()
            .iter()
            .enumerate()
            .map(|(i, glyph)| {
                let radius = rule.radius(glyph.font_size)?;
                Ok(engine.cache_halo(layout, i, atlas, radius)?)
            })
            .collect()
    }
    pub fn raw_bytes(&self) -> usize {
        self.bytes
    }
    /// Borrow shared catalog bytes for a checked topology rebuild. No reread or
    /// duplicate raw font allocation is necessary.
    pub fn sources(&self) -> impl ExactSizeIterator<Item = &SharedFontSource> {
        self.sources.values()
    }

    /// Shape the final content-box width. A zero-width clipped box still uses
    /// the shaper's minimum one-pixel width; its visible extent remains zero.
    pub fn shape(
        &mut self,
        widget: &Widget,
        display: &str,
        width: f32,
    ) -> Result<Arc<ShapedText>, UiError> {
        let text = widget
            .node
            .kind
            .text()
            .ok_or(UiError::Invalid("widget has no text"))?;
        if !width.is_finite() || !(0.0..=MAX_UI_EXTENT).contains(&width) {
            return Err(UiError::Invalid("text content width"));
        }
        let style = TextStyle {
            font_size: text.font_size,
            line_height: text.line_height,
            width: width.max(1.0),
            wrap: match text.wrap {
                incant_types::TextWrap::None => Wrap::None,
                incant_types::TextWrap::Word => Wrap::Word,
                incant_types::TextWrap::WordOrGlyph => Wrap::WordOrGlyph,
            },
        };
        Ok(self
            .engines
            .get_mut(&text.fonts)
            .ok_or(UiError::Invalid("unbound font chain"))?
            .shape(display, style)?)
    }
}
impl TextMeasure for UiFonts {
    fn measure(
        &mut self,
        widget: &Widget,
        display: &str,
        known: [Option<f32>; 2],
        available: [MeasureSpace; 2],
    ) -> Result<[f32; 2], UiError> {
        let text = widget
            .node
            .kind
            .text()
            .ok_or(UiError::Invalid("measurement without text"))?;
        let (width, wrap) = if let Some(width) = known[0] {
            (width, text.wrap)
        } else {
            match available[0] {
                MeasureSpace::Definite(width) => {
                    if !width.is_finite() {
                        return Err(UiError::Invalid("measured available width"));
                    }
                    (width.max(0.0), text.wrap)
                }
                // Width one lets the existing Unicode break rules measure the
                // longest unbreakable segment (or glyph for WordOrGlyph).
                MeasureSpace::MinContent => (1.0, text.wrap),
                MeasureSpace::MaxContent => (MAX_UI_EXTENT, incant_types::TextWrap::None),
            }
        };
        if !width.is_finite() || !(0.0..=MAX_UI_EXTENT).contains(&width) {
            return Err(UiError::Invalid("measured available width"));
        }
        let shaped = self
            .engines
            .get_mut(&text.fonts)
            .ok_or(UiError::Invalid("unbound font chain"))?
            .shape(
                display,
                TextStyle {
                    font_size: text.font_size,
                    line_height: text.line_height,
                    width: width.max(1.0),
                    wrap: match wrap {
                        incant_types::TextWrap::None => Wrap::None,
                        incant_types::TextWrap::Word => Wrap::Word,
                        incant_types::TextWrap::WordOrGlyph => Wrap::WordOrGlyph,
                    },
                },
            )?;
        Ok([
            known[0].unwrap_or(shaped.size()[0]),
            known[1].unwrap_or(shaped.size()[1]),
        ])
    }
}
