use crate::{AssetError, Result, invalid};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub const MAX_TEXELS: usize = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 8192;
pub use incant_types::TextureUsage;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureFormat {
    Rgba8Srgb,
    Rgba8Linear,
    Rgba32Float,
}
impl TextureFormat {
    pub fn bytes_per_pixel(self) -> usize {
        if self == Self::Rgba32Float { 16 } else { 4 }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    /// Tightly packed top-left-origin rows; base level first. Floats are LE.
    /// Immutable levels allow checked role variants to share level zero without
    /// duplicating its resident bytes. The texture descriptor remains editable
    /// until a bounded reader seals it inside a RuntimeAsset.
    pub levels: Vec<Arc<[u8]>>,
}
impl Texture {
    /// Logical mip payload, before accounting for shared allocations. Resource
    /// limits conservatively charge every role's complete chain.
    pub fn payload_bytes(&self) -> usize {
        self.levels.iter().map(|level| level.len()).sum()
    }

    /// Share only an exactly matching base after validating both complete
    /// layouts. Used once during cooking/loading, never during a frame update.
    pub fn share_base_from(&mut self, source: &Self) -> Result<()> {
        self.validate()?;
        source.validate()?;
        if self.width != source.width
            || self.height != source.height
            || self.format != source.format
            || self.levels[0] != source.levels[0]
        {
            return Err(invalid("color role base/layout mismatch"));
        }
        self.levels[0] = Arc::clone(&source.levels[0]);
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        dimensions(self.width, self.height)?;
        if self.levels.len() != mip_count(self.width, self.height) {
            return Err(invalid("incomplete texture mip chain"));
        }
        for (level, bytes) in self.levels.iter().enumerate() {
            let (w, h) = ((self.width >> level).max(1), (self.height >> level).max(1));
            if bytes.len() != w as usize * h as usize * self.format.bytes_per_pixel() {
                return Err(invalid("texture mip byte length mismatch"));
            }
            if self.format == TextureFormat::Rgba32Float
                && bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|f| !f32::from_le_bytes(*f).is_finite())
            {
                return Err(invalid("nonfinite texture data"));
            }
        }
        Ok(())
    }
}

/// Exact retained mip bytes for these textures, excluding Arc/Vec metadata.
/// Identity, not equal contents, determines whether an allocation is shared.
pub(crate) fn resident_bytes<'a>(textures: impl IntoIterator<Item = &'a Texture>) -> usize {
    let mut allocations = HashSet::new();
    textures
        .into_iter()
        .flat_map(|texture| &texture.levels)
        .filter(|level| allocations.insert(Arc::as_ptr(level)))
        .map(|level| level.len())
        .sum()
}
/// Shared decoded-image extent check, also used before authoring image allocation.
pub fn dimensions(w: u32, h: u32) -> Result<()> {
    if w == 0
        || h == 0
        || w > MAX_DIMENSION
        || h > MAX_DIMENSION
        || u64::from(w) * u64::from(h) > MAX_TEXELS as u64
    {
        return Err(AssetError::Limit("texture dimensions"));
    }
    Ok(())
}
pub(crate) fn mip_count(w: u32, h: u32) -> usize {
    (w.max(h).ilog2() + 1) as usize
}

#[cfg(test)]
mod sharing_tests {
    use super::*;

    fn texture(w: u32, h: u32) -> Texture {
        Texture {
            width: w,
            height: h,
            format: TextureFormat::Rgba8Srgb,
            levels: (0..mip_count(w, h))
                .map(|level| {
                    vec![127; (w >> level).max(1) as usize * (h >> level).max(1) as usize * 4]
                        .into()
                })
                .collect(),
        }
    }

    #[test]
    fn resident_counts_allocations_not_equal_bytes_for_square_thin_and_odd_sizes() {
        for (w, h) in [(1, 1), (4, 4), (1, 8), (8, 1), (3, 5), (7, 1)] {
            let a = texture(w, h);
            let mut b = texture(w, h);
            let logical = a.payload_bytes() + b.payload_bytes();
            assert_eq!(resident_bytes([&a, &b]), logical);
            b.share_base_from(&a).unwrap();
            assert!(Arc::ptr_eq(&a.levels[0], &b.levels[0]));
            assert_eq!(resident_bytes([&a, &b]), logical - (w * h * 4) as usize);
            assert_eq!(a.payload_bytes() + b.payload_bytes(), logical);
            assert_eq!(resident_bytes([&a, &a.clone()]), a.payload_bytes());
        }
    }

    #[test]
    fn mismatched_bytes_layout_and_invalid_tail_cannot_be_shared() {
        let source = texture(2, 2);
        let mut changed = texture(2, 2);
        Arc::make_mut(&mut changed.levels[0])[0] = 0;
        let before = changed.clone();
        assert!(changed.share_base_from(&source).is_err());
        assert_eq!(changed, before);
        assert!(texture(4, 1).share_base_from(&source).is_err());
        let mut bad = texture(2, 2);
        bad.levels.pop();
        assert!(bad.share_base_from(&source).is_err());
        let mut format = texture(2, 2);
        format.format = TextureFormat::Rgba8Linear;
        assert!(format.share_base_from(&source).is_err());
    }
}
