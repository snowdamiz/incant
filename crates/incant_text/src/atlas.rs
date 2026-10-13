//! Retained, append-only CPU alpha atlas. No eviction/repacking behind a live
//! handle; reset explicitly invalidates every handle. Renderer upload is future.
//! An entry is a glyph's own coverage, or its halo: that coverage dilated by a
//! radius (`halo::dilate`) and stored as a separate, larger entry.
use crate::{
    ShapedText, TextEngine, TextError,
    halo::{self, MAX_HALO_RADIUS},
    raster::Raster,
};
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
impl AtlasConfig {
    /// Validate byte/key limits before any pixel allocation or raster work.
    pub fn validate(self) -> Result<usize, TextError> {
        let config = self;
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
        Ok(bytes)
    }
}
/// Exact raster identity: packaged asset + content hash, glyph, unquantized
/// positive size bits, and the halo radius bits (zero for the glyph's own
/// coverage). Integer-origin coverage only; placement stays with layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphKey {
    pub font: Ulid,
    pub content_sha256: [u8; 32],
    pub glyph: u16,
    pub font_size_bits: u32,
    pub halo_radius_bits: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasEntry {
    pub key: GlyphKey,
    /// Inner coverage rectangle [x,y,width,height], excluding 1px zero padding.
    /// Empty outlines (including spaces) consume a key, but no texels. A halo's
    /// rectangle is its glyph's grown by `ceil(radius)` texels on every side.
    pub rect: Option<[u32; 4]>,
    /// Signed top-left bearing relative to the logical baseline, Y down. A
    /// halo's is its glyph's moved up and left by the same margin, also for an
    /// empty outline.
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
    /// Reused halo scratch, bounded by the largest glyph plus its margin.
    halo: Vec<u8>,
}
impl GlyphAtlas {
    pub fn new(config: AtlasConfig) -> Result<Self, TextError> {
        let bytes = config.validate()?;
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
            halo: Vec::new(),
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
        let radius = f32::from_bits(key.halo_radius_bits);
        if key.halo_radius_bits != 0 && !(radius > 0. && radius <= MAX_HALO_RADIUS) {
            return Err(TextError::Invalid("halo radius"));
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TextError::Limit("atlas revision"))?;
        let image = self
            .raster
            .image(bytes, key.glyph, f32::from_bits(key.font_size_bits))?;
        // A halo is the coverage dilated by its radius, `margin` texels larger
        // on every side, with its origin moved up and left by that margin. An
        // empty outline keeps an empty halo at the moved origin.
        let (pixels, width, height, origin) = if key.halo_radius_bits == 0 {
            (image.pixels, image.width, image.height, image.origin)
        } else {
            let margin = halo::margin(radius);
            let origin = image.origin.map(|v| v - margin as i32);
            if image.width == 0 || image.height == 0 {
                (image.pixels, 0, 0, origin)
            } else {
                halo::dilate(
                    image.pixels,
                    image.width,
                    image.height,
                    radius,
                    &mut self.halo,
                );
                let grow = 2 * margin;
                (
                    self.halo.as_slice(),
                    image.width + grow,
                    image.height + grow,
                    origin,
                )
            }
        };
        let mut shelf = self.shelf;
        let rect = if width == 0 || height == 0 {
            None
        } else {
            let (padded_width, padded_height) = (width + 2, height + 2);
            if padded_width > self.config.width || padded_height > self.config.height {
                return Err(TextError::Limit("atlas glyph dimensions"));
            }
            if shelf.x + padded_width > self.config.width {
                shelf.x = 0;
                shelf.y += shelf.height;
                shelf.height = 0;
            }
            if shelf.y + padded_height > self.config.height {
                return Err(TextError::Limit("atlas full"));
            }
            let rect = [shelf.x + 1, shelf.y + 1, width, height];
            shelf.x += padded_width;
            shelf.height = shelf.height.max(padded_height);
            Some(rect)
        };
        // No fallible validation below: only private memory, no external callback.
        let entry = AtlasEntry {
            key,
            rect,
            origin,
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
                    .copy_from_slice(&pixels[src..src + width as usize]);
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
        self.cache(text, index, atlas, 0)
    }
    /// The halo of one already-shaped glyph: its coverage dilated by `radius`
    /// texels (`halo::dilate`), prepared once here and cached under the glyph's
    /// key plus the radius, so drawing it costs one glyph's coverage taps. The
    /// glyph's own coverage entry is unaffected. Refuses a radius outside
    /// `(0, MAX_HALO_RADIUS]`.
    pub fn cache_halo(
        &self,
        text: &ShapedText,
        index: usize,
        atlas: &mut GlyphAtlas,
        radius: f32,
    ) -> Result<GlyphHandle, TextError> {
        if !(radius > 0. && radius <= MAX_HALO_RADIUS) {
            return Err(TextError::Invalid("halo radius"));
        }
        self.cache(text, index, atlas, radius.to_bits())
    }
    fn cache(
        &self,
        text: &ShapedText,
        index: usize,
        atlas: &mut GlyphAtlas,
        halo_radius_bits: u32,
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
                halo_radius_bits,
            },
            &font.bytes,
        )
    }
}
