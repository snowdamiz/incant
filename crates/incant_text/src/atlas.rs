//! Retained, append-only CPU alpha atlas. No eviction/repacking behind a live
//! handle; reset explicitly invalidates every handle. Renderer upload is future.
use crate::{ShapedText, TextEngine, TextError, raster::Raster};
use std::{collections::BTreeMap, sync::Arc};
use ulid::Ulid;
pub const MAX_ATLAS_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ATLAS_GLYPHS: usize = 8192;

#[derive(Clone, Copy, Debug)]
pub struct AtlasConfig {
    pub width: u32,
    pub height: u32,
    pub max_glyphs: usize,
}
/// Exact raster identity: packaged asset + content hash, glyph, and unquantized
/// positive size bits. Integer-origin coverage only; placement stays with layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphKey {
    pub font: Ulid,
    pub content_sha256: [u8; 32],
    pub glyph: u16,
    pub font_size_bits: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasEntry {
    pub key: GlyphKey,
    /// Inner coverage rectangle [x,y,width,height], excluding 1px zero padding.
    /// Empty outlines (including spaces) consume a key, but no texels.
    pub rect: Option<[u32; 4]>,
    /// Signed top-left bearing relative to the logical baseline, Y down.
    pub origin: [i32; 2],
    /// Glyph zero is retained as explicit .notdef rather than hidden fallback.
    pub missing: bool,
}
#[derive(Clone, Debug)]
pub struct GlyphHandle {
    owner: Arc<()>,
    generation: u64,
    slot: usize,
}
#[derive(Clone, Copy, Default)]
struct Shelf {
    x: u32,
    y: u32,
    height: u32,
}
pub struct GlyphAtlas {
    config: AtlasConfig,
    pixels: Vec<u8>,
    entries: Vec<AtlasEntry>,
    keys: BTreeMap<GlyphKey, usize>,
    owner: Arc<()>,
    generation: u64,
    revision: u64,
    shelf: Shelf,
    raster: Raster,
}
impl GlyphAtlas {
    pub fn new(config: AtlasConfig) -> Result<Self, TextError> {
        let bytes = (config.width as usize)
            .checked_mul(config.height as usize)
            .ok_or(TextError::Limit("atlas dimensions"))?;
        if config.width == 0
            || config.height == 0
            || config.width > 2048
            || config.height > 2048
            || bytes > MAX_ATLAS_BYTES
            || !(1..=MAX_ATLAS_GLYPHS).contains(&config.max_glyphs)
        {
            return Err(TextError::Limit("atlas configuration"));
        }
        Ok(Self {
            config,
            pixels: vec![0; bytes],
            entries: Vec::with_capacity(config.max_glyphs),
            keys: BTreeMap::new(),
            owner: Arc::new(()),
            generation: 0,
            revision: 0,
            shelf: Shelf::default(),
            raster: Raster::default(),
        })
    }
    pub fn size(&self) -> [u32; 2] {
        [self.config.width, self.config.height]
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Changes only after successful insertion/reset, never on cache hit/refusal.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn get(&self, handle: &GlyphHandle) -> Option<&AtlasEntry> {
        if !Arc::ptr_eq(&self.owner, &handle.owner) || handle.generation != self.generation {
            return None;
        }
        self.entries.get(handle.slot)
    }
    /// Reuses the pixel allocation; all old handles become invalid atomically.
    pub fn reset(&mut self) -> Result<(), TextError> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(TextError::Limit("atlas generation"))?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TextError::Limit("atlas revision"))?;
        self.pixels.fill(0);
        self.entries.clear();
        self.keys.clear();
        self.shelf = Shelf::default();
        self.generation = generation;
        self.revision = revision;
        Ok(())
    }
    fn handle(&self, slot: usize) -> GlyphHandle {
        GlyphHandle {
            owner: Arc::clone(&self.owner),
            generation: self.generation,
            slot,
        }
    }
    fn insert(&mut self, key: GlyphKey, bytes: &[u8]) -> Result<GlyphHandle, TextError> {
        if let Some(&slot) = self.keys.get(&key) {
            return Ok(self.handle(slot));
        }
        if self.entries.len() == self.config.max_glyphs {
            return Err(TextError::Limit("atlas glyph count"));
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TextError::Limit("atlas revision"))?;
        let image = self
            .raster
            .image(bytes, key.glyph, f32::from_bits(key.font_size_bits))?;
        let mut shelf = self.shelf;
        let rect = if image.width == 0 || image.height == 0 {
            None
        } else {
            let (width, height) = (image.width + 2, image.height + 2);
            if width > self.config.width || height > self.config.height {
                return Err(TextError::Limit("atlas glyph dimensions"));
            }
            if shelf.x + width > self.config.width {
                shelf.x = 0;
                shelf.y += shelf.height;
                shelf.height = 0;
            }
            if shelf.y + height > self.config.height {
                return Err(TextError::Limit("atlas full"));
            }
            let rect = [shelf.x + 1, shelf.y + 1, image.width, image.height];
            shelf.x += width;
            shelf.height = shelf.height.max(height);
            Some(rect)
        };
        // No fallible validation below: only private memory, no external callback.
        let entry = AtlasEntry {
            key,
            rect,
            origin: image.origin,
            missing: key.glyph == 0,
        };
        let slot = self.entries.len();
        self.keys.insert(key, slot);
        self.entries.push(entry);
        if let Some([x, y, width, height]) = rect {
            for row in 0..height as usize {
                let dst = (y as usize + row) * self.config.width as usize + x as usize;
                let src = row * width as usize;
                self.pixels[dst..dst + width as usize]
                    .copy_from_slice(&image.pixels[src..src + width as usize]);
            }
        }
        self.shelf = shelf;
        self.revision = revision;
        Ok(self.handle(slot))
    }
}
impl TextEngine {
    /// Rasterize one already-shaped glyph using this exact immutable font chain.
    /// A shape from another/reloaded engine is rejected even if asset IDs match.
    /// Cache hits neither parse fonts nor allocate. Misses have bounded outline/
    /// mask scratch; alpha-only, no hinting, subpixel phase or variation axes.
    pub fn cache_glyph(
        &self,
        text: &ShapedText,
        index: usize,
        atlas: &mut GlyphAtlas,
    ) -> Result<GlyphHandle, TextError> {
        if !Arc::ptr_eq(&text.font_identity, &self.fonts.identity) {
            return Err(TextError::Invalid(
                "shaped text belongs to another font engine",
            ));
        }
        let glyph = text
            .glyphs()
            .get(index)
            .ok_or(TextError::Invalid("glyph index"))?;
        if !glyph.font_size.is_finite() || !(1.0..=512.0).contains(&glyph.font_size) {
            return Err(TextError::Invalid("raster size"));
        }
        let font = self
            .fonts
            .sources
            .get(&glyph.font)
            .ok_or(TextError::Invalid("font asset"))?;
        atlas.insert(
            GlyphKey {
                font: glyph.font,
                content_sha256: font.digest,
                glyph: glyph.id,
                font_size_bits: glyph.font_size.to_bits(),
            },
            &font.bytes,
        )
    }
}
