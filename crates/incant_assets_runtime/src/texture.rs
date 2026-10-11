use crate::{AssetError, Result, invalid};
use serde::{Deserialize, Serialize};

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
    pub levels: Vec<Vec<u8>>,
}
impl Texture {
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
