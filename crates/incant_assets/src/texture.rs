use crate::{AssetError, MAX_SOURCE_BYTES, Result, invalid};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

pub const MAX_TEXELS: usize = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 8192;
pub use incant_doc::TextureUsage;
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
pub(crate) fn dimensions(w: u32, h: u32) -> Result<()> {
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
fn decode_srgb(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encode_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}

/// Color inputs use sRGB for PNG/JPEG and linear light for EXR. Linear maps
/// retain channel values; normal maps renormalize averaged vectors. Sixteen-bit
/// inputs and EXR keep float precision instead of being truncated to eight bits.
pub fn import_image(bytes: &[u8], usage: TextureUsage) -> Result<Texture> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(AssetError::Limit("image source bytes"));
    }
    let format = image::guess_format(bytes).map_err(|e| invalid(e.to_string()))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::OpenExr
    ) {
        return Err(AssetError::Unsupported(
            "image format (expected PNG, JPEG or EXR)".into(),
        ));
    }
    if format == ImageFormat::OpenExr {
        let metadata = exr::meta::MetaData::read_from_buffered(Cursor::new(bytes), false)
            .map_err(|e| invalid(e.to_string()))?;
        if metadata.requirements.has_multiple_layers || metadata.requirements.has_deep_data {
            return Err(AssetError::Unsupported("multipart or deep EXR".into()));
        }
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_SOURCE_BYTES as u64);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|e| invalid(e.to_string()))?;
    let (width, height) = decoder.dimensions();
    dimensions(width, height)?;
    let high_precision =
        decoder.color_type().bits_per_pixel() / u16::from(decoder.color_type().channel_count()) > 8;
    let decoded = DynamicImage::from_decoder(decoder)
        .map_err(|e| invalid(e.to_string()))?
        .into_rgba32f();
    let srgb_input = usage == TextureUsage::Color && format != ImageFormat::OpenExr;
    let target = if high_precision {
        TextureFormat::Rgba32Float
    } else if srgb_input {
        TextureFormat::Rgba8Srgb
    } else {
        TextureFormat::Rgba8Linear
    };
    let mut pixels: Vec<[f32; 4]> = decoded.pixels().map(|p| p.0).collect();
    for p in &mut pixels {
        if p.iter().any(|v| !v.is_finite()) {
            return Err(invalid("nonfinite image pixels"));
        }
        if !(0.0..=1.0).contains(&p[3]) {
            return Err(invalid("image alpha outside normalized range"));
        }
        if srgb_input {
            for v in &mut p[..3] {
                *v = decode_srgb(*v);
            }
        }
    }
    let mut levels = Vec::new();
    let (mut w, mut h) = (width, height);
    loop {
        let mut bytes = Vec::with_capacity(w as usize * h as usize * target.bytes_per_pixel());
        for p in &pixels {
            for (i, &channel) in p.iter().enumerate() {
                if target == TextureFormat::Rgba32Float {
                    bytes.extend(channel.to_le_bytes());
                } else {
                    let v = if target == TextureFormat::Rgba8Srgb && i < 3 {
                        encode_srgb(channel)
                    } else {
                        channel
                    };
                    bytes.push((v.clamp(0., 1.) * 255.).round() as u8);
                }
            }
        }
        levels.push(bytes);
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        pixels = downsample(&pixels, w, h, nw, nh, usage);
        w = nw;
        h = nh;
    }
    let texture = Texture {
        width,
        height,
        format: target,
        levels,
    };
    texture.validate()?;
    Ok(texture)
}
// Area-weighted box filter includes every source texel for odd dimensions.
// Colors are averaged premultiplied in linear light, then stored straight-alpha.
fn downsample(
    src: &[[f32; 4]],
    w: u32,
    h: u32,
    nw: u32,
    nh: u32,
    usage: TextureUsage,
) -> Vec<[f32; 4]> {
    let mut out = Vec::with_capacity((nw * nh) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let (x0, x1) = (
                f64::from(x) * f64::from(w) / f64::from(nw),
                f64::from(x + 1) * f64::from(w) / f64::from(nw),
            );
            let (y0, y1) = (
                f64::from(y) * f64::from(h) / f64::from(nh),
                f64::from(y + 1) * f64::from(h) / f64::from(nh),
            );
            let mut sum = [0_f64; 4];
            let mut total = 0.;
            for sy in y0.floor() as u32..(y1.ceil() as u32).min(h) {
                for sx in x0.floor() as u32..(x1.ceil() as u32).min(w) {
                    let weight = (x1.min(f64::from(sx + 1)) - x0.max(f64::from(sx)))
                        * (y1.min(f64::from(sy + 1)) - y0.max(f64::from(sy)));
                    let p = src[(sy * w + sx) as usize];
                    total += weight;
                    for i in 0..4 {
                        let mut v = f64::from(p[i]);
                        if i < 3 {
                            match usage {
                                TextureUsage::Color => v *= f64::from(p[3]),
                                TextureUsage::Normal => v = v * 2. - 1.,
                                _ => {}
                            }
                        }
                        sum[i] += v * weight;
                    }
                }
            }
            for s in &mut sum {
                *s /= total;
            }
            match usage {
                TextureUsage::Color => {
                    let a = sum[3];
                    for s in &mut sum[..3] {
                        *s = if a > 0. { *s / a } else { 0. };
                    }
                }
                TextureUsage::Normal => {
                    let len = sum[..3].iter().map(|v| v * v).sum::<f64>().sqrt();
                    for (i, s) in sum[..3].iter_mut().enumerate() {
                        *s = (if len > 1e-10 {
                            *s / len
                        } else if i == 2 {
                            1.
                        } else {
                            0.
                        }) * 0.5
                            + 0.5;
                    }
                }
                TextureUsage::Linear => {}
            }
            out.push(sum.map(|v| v as f32));
        }
    }
    out
}
