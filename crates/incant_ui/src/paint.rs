//! Canonical retained screen-space glyph input. Top-left content origins and
//! Y-down logical pixels are explicit; no world matrix, shader or visual style.
use crate::{LayoutFrame, Rect, StableId, UiError, UiFonts, UiPending, UiRuntime};
use incant_text::{AtlasConfig, GlyphAtlas, GlyphHandle, ShapedText};
use std::sync::Arc;
pub const MAX_UI_GLYPHS: usize = 65_536;
pub const MAX_UI_LINES: usize = 16_384;
pub struct UiTextPaint {
    pub id: StableId,
    pub origin: [f32; 2],
    pub clip: Rect,
    pub layout: Arc<ShapedText>,
    pub glyphs: Arc<[GlyphHandle]>,
}
pub struct UiPaint {
    config: AtlasConfig,
    pub layout: Arc<LayoutFrame>,
    pub texts: Vec<UiTextPaint>,
    pub atlas: Arc<GlyphAtlas>,
}
impl UiPaint {
    /// One accepted and one candidate UI atlas (each <=4MiB), <=2*65,536
    /// glyph placements and <=2*16,384 lines may overlap. This is additional to
    /// independently bounded world Text. All combined UI counts are checked
    /// before allocating/rasterizing its candidate plane. Shaping can stage one
    /// bounded layout beyond the aggregate before refusing. Raw fonts are shared.
    pub fn prepare(
        runtime: &UiRuntime,
        pending: &UiPending,
        fonts: &mut UiFonts,
        config: AtlasConfig,
        previous: Option<&Arc<Self>>,
    ) -> Result<Arc<Self>, UiError> {
        config.validate()?;
        let (widgets, state) = pending.prepared(runtime)?;
        if let Some(previous) = previous
            && Arc::ptr_eq(&previous.layout, pending.layout())
            && previous.config.width == config.width
            && previous.config.height == config.height
            && previous.config.max_glyphs == config.max_glyphs
        {
            return Ok(Arc::clone(previous));
        }
        let mut layouts = Vec::new();
        let mut glyphs = 0usize;
        let mut lines = 0usize;
        // Layout preorder is the canonical draw order; font-chain identity is
        // taken only from the validated widget declaration.
        for (position, node) in pending.layout().nodes.iter().enumerate() {
            let index = pending.layout().widget_index(position);
            let widget = &widgets[index];
            let Some(text) = widget.node.kind.text() else {
                continue;
            };
            let display = state.buffers[index]
                .as_ref()
                .map_or(text.content.as_str(), |b| b.display_text());
            let layout = fonts.shape(widget, display, node.content.size[0])?;
            glyphs = glyphs
                .checked_add(layout.glyphs().len())
                .filter(|n| *n <= MAX_UI_GLYPHS)
                .ok_or(UiError::Limit("UI glyph placements"))?;
            lines = lines
                .checked_add(layout.lines().len())
                .filter(|n| *n <= MAX_UI_LINES)
                .ok_or(UiError::Limit("UI layout lines"))?;
            layouts.push((index, node, layout));
        }
        let mut atlas = GlyphAtlas::new(config)?;
        let mut texts = Vec::with_capacity(layouts.len());
        for (index, node, layout) in layouts {
            let glyphs = fonts.raster(&widgets[index], &layout, &mut atlas)?;
            texts.push(UiTextPaint {
                id: node.id,
                origin: node.content.origin,
                clip: node.clip,
                layout,
                glyphs: glyphs.into(),
            });
        }
        Ok(Arc::new(Self {
            config,
            layout: Arc::clone(pending.layout()),
            texts,
            atlas: Arc::new(atlas),
        }))
    }
}
