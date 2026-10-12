//! Cold text preparation, sharing raw fonts and publishing only complete layouts.
use crate::NativeRenderError;
use incant_assets_runtime::FontCatalog;
use incant_render_data::{TextInput, TextNode};
use incant_runtime::{FontAssetRef, SpatialView, StableId, TextView, TextWrap};
use incant_text::{AtlasConfig, GlyphAtlas, SharedFontSource, TextEngine, TextStyle, Wrap};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub const MAX_TEXT_CHAINS: usize = 32;
pub const MAX_TEXT_GLYPHS: usize = 65_536;
pub const MAX_TEXT_LINES: usize = 16_384;

/// Font paths are explicit startup inputs, never inferred from another cache.
/// Each atlas alpha plane is at most 4 MiB; maps/handles/scratch are additional.
/// Rebuild retains the accepted atlas while
/// preparing one candidate; fonts share their single admitted 64 MiB catalog.
#[derive(Clone)]
pub struct TextSettings {
    pub font_cache: PathBuf,
    pub atlas: AtlasConfig,
}
impl TextSettings {
    pub fn new(font_cache: PathBuf) -> Self {
        Self {
            font_cache,
            atlas: AtlasConfig {
                width: 2048,
                height: 2048,
                max_glyphs: incant_text::MAX_ATLAS_GLYPHS,
            },
        }
    }
}
pub(crate) struct Binding {
    refs: Vec<FontAssetRef>,
    fonts: BTreeMap<StableId, SharedFontSource>,
    atlas: AtlasConfig,
    pub bytes: usize,
}
pub(crate) struct Prepared {
    pub input: Option<TextInput>,
    pub slots: Vec<Option<usize>>,
    pub undo: Vec<glam::DMat4>,
}
impl Binding {
    pub fn sources(&self) -> impl Iterator<Item = &SharedFontSource> {
        self.fonts.values()
    }
    pub fn load(view: &TextView<'_>, settings: TextSettings) -> Result<Self, NativeRenderError> {
        settings.atlas.validate()?;
        let catalog = FontCatalog::load(&settings.font_cache, view.fonts())?;
        let bytes = catalog.bytes();
        let fonts = catalog
            .into_fonts()
            .map(|(id, font)| (id, font.into_source(ulid::Ulid::from_bytes(id.0)).into()))
            .collect();
        Ok(Self {
            refs: view.fonts().to_vec(),
            fonts,
            atlas: settings.atlas,
            bytes,
        })
    }
    pub fn validate(&self, text: &TextView<'_>) -> Result<(), NativeRenderError> {
        if self.refs != text.fonts() {
            return Err(NativeRenderError::TextBinding);
        }
        Ok(())
    }
    pub fn prepare(
        &self,
        text: &TextView<'_>,
        spatial: &SpatialView<'_>,
    ) -> Result<Prepared, NativeRenderError> {
        self.validate(text)?;
        let mut engines = BTreeMap::<Vec<StableId>, TextEngine>::new();
        let mut layouts = Vec::with_capacity(text.nodes().len());
        let mut glyphs = 0usize;
        let mut lines = 0usize;
        // Check all aggregate layout counts before creating the candidate atlas
        // or performing any raster work. One bounded shape can be staged beyond
        // an aggregate ceiling; shaper caches/scratch have their own limits.
        for (id, node) in text.nodes() {
            if !engines.contains_key(node.fonts.as_ref()) {
                if engines.len() == MAX_TEXT_CHAINS {
                    return Err(NativeRenderError::TextLimit("font chains"));
                }
                let fonts = node
                    .fonts
                    .iter()
                    .map(|id| {
                        self.fonts
                            .get(id)
                            .cloned()
                            .ok_or(NativeRenderError::TextBinding)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                engines.insert(node.fonts.to_vec(), TextEngine::new_shared(fonts)?);
            }
            let engine = engines.get_mut(node.fonts.as_ref()).unwrap();
            let layout = engine.shape(
                &node.content,
                TextStyle {
                    font_size: node.layout.font_size,
                    line_height: node.layout.line_height,
                    width: node.layout.width,
                    wrap: match node.layout.wrap {
                        TextWrap::None => Wrap::None,
                        TextWrap::Word => Wrap::Word,
                        TextWrap::WordOrGlyph => Wrap::WordOrGlyph,
                    },
                },
            )?;
            glyphs = glyphs
                .checked_add(layout.glyphs().len())
                .filter(|n| *n <= MAX_TEXT_GLYPHS)
                .ok_or(NativeRenderError::TextLimit("glyph placements"))?;
            lines = lines
                .checked_add(layout.lines().len())
                .filter(|n| *n <= MAX_TEXT_LINES)
                .ok_or(NativeRenderError::TextLimit("layout lines"))?;
            layouts.push((id, node.fonts.as_ref(), layout));
        }
        if layouts.is_empty() {
            return map(None, spatial);
        }
        let mut atlas = GlyphAtlas::new(self.atlas)?;
        let mut nodes = Vec::with_capacity(layouts.len());
        for (id, chain, layout) in layouts {
            let engine = &engines[chain];
            let glyphs = (0..layout.glyphs().len())
                .map(|i| engine.cache_glyph(&layout, i, &mut atlas))
                .collect::<Result<Vec<_>, _>>()?;
            nodes.push(TextNode {
                id,
                world: glam::DMat4::IDENTITY,
                layout,
                glyphs: glyphs.into(),
            });
        }
        map(
            Some(TextInput {
                nodes,
                atlas: Arc::new(atlas),
            }),
            spatial,
        )
    }
}
/// Spatial topology can change without text changes; reuse immutable prepared
/// layouts/atlas and only rebuild the bounded numeric slot map.
pub(crate) fn map(
    mut input: Option<TextInput>,
    spatial: &SpatialView<'_>,
) -> Result<Prepared, NativeRenderError> {
    let mut slots = vec![None; spatial.nodes().len()];
    let mut found = 0;
    if let Some(text) = &mut input {
        for node in spatial.nodes() {
            if let Ok(slot) = text.nodes.binary_search_by_key(&node.id, |n| n.id) {
                text.nodes[slot].world = *node.global;
                slots[node.index] = Some(slot);
                found += 1;
            }
        }
        if found != text.nodes.len() {
            return Err(NativeRenderError::TextBinding);
        }
    }
    Ok(Prepared {
        input,
        slots,
        undo: vec![glam::DMat4::IDENTITY; found],
    })
}
