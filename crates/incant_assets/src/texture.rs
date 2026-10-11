use crate::{AssetError, MAX_SOURCE_BYTES, Result, invalid};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use incant_assets_runtime::texture::MAX_DIMENSION;
pub use incant_assets_runtime::{Texture, TextureFormat, TextureUsage};
use std::io::Cursor;

/// Color space and coverage filtering are separate decisions. An opaque base
/// color or emissive image ignores its own alpha; a blended base or sprite does
/// not. This cook-only role never changes level-zero source channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImageRole {
    ColorCoverage,
    ColorUnweighted,
    Linear,
    Normal,
}
impl ImageRole {
    pub(crate) fn usage(self) -> TextureUsage {
        match self {
            Self::ColorCoverage | Self::ColorUnweighted => TextureUsage::Color,
            Self::Linear => TextureUsage::Linear,
            Self::Normal => TextureUsage::Normal,
        }
    }
}
impl From<TextureUsage> for ImageRole {
    fn from(usage: TextureUsage) -> Self {
        match usage {
            TextureUsage::Color => Self::ColorCoverage,
            TextureUsage::Linear => Self::Linear,
            TextureUsage::Normal => Self::Normal,
        }
    }
}

fn dimensions(w: u32, h: u32) -> Result<()> {
    incant_assets_runtime::texture::dimensions(w, h).map_err(Into::into)
}
pub(crate) fn decode_srgb(v: f32) -> f32 {
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
    import_image_role(bytes, usage.into())
}

pub(crate) fn import_image_role(bytes: &[u8], role: ImageRole) -> Result<Texture> {
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
    let srgb_input = role.usage() == TextureUsage::Color && format != ImageFormat::OpenExr;
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
    from_linear_role(width, height, pixels, target, role)
}

pub(crate) fn from_linear_role(
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
    target: TextureFormat,
    role: ImageRole,
) -> Result<Texture> {
    from_linear_pixels(width, height, &pixels, target, role)
}

pub(crate) fn color_pair_limit(texture: &Texture) -> Result<()> {
    if texture.payload_bytes() > MAX_SOURCE_BYTES / 2 {
        return Err(AssetError::Limit("combined color mip bytes"));
    }
    Ok(())
}

pub(crate) fn color_variants(
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
    target: TextureFormat,
) -> Result<(Texture, Texture)> {
    let coverage = from_linear_pixels(width, height, &pixels, target, ImageRole::ColorCoverage)?;
    color_pair_limit(&coverage)?;
    let mut independent =
        from_linear_pixels(width, height, &pixels, target, ImageRole::ColorUnweighted)?;
    independent.share_base_from(&coverage)?;
    Ok((coverage, independent))
}

fn from_linear_pixels(
    width: u32,
    height: u32,
    pixels: &[[f32; 4]],
    target: TextureFormat,
    role: ImageRole,
) -> Result<Texture> {
    dimensions(width, height)?;
    if pixels.len() != width as usize * height as usize {
        return Err(invalid("linear texture pixel count mismatch"));
    }
    let mut levels = Vec::new();
    let mut pixels = std::borrow::Cow::Borrowed(pixels);
    let (mut w, mut h) = (width, height);
    loop {
        let mut bytes = Vec::with_capacity(w as usize * h as usize * target.bytes_per_pixel());
        for p in pixels.iter() {
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
        levels.push(bytes.into());
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        pixels = std::borrow::Cow::Owned(downsample(&pixels, w, h, nw, nh, role));
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
// Coverage colors are averaged premultiplied in linear light, then stored
// straight-alpha. Independent colors preserve RGB regardless of source alpha.
fn downsample(
    src: &[[f32; 4]],
    w: u32,
    h: u32,
    nw: u32,
    nh: u32,
    role: ImageRole,
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
                            match role {
                                ImageRole::ColorCoverage => v *= f64::from(p[3]),
                                ImageRole::Normal => v = v * 2. - 1.,
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
            match role {
                ImageRole::ColorCoverage => {
                    let a = sum[3];
                    for s in &mut sum[..3] {
                        *s = if a > 0. { *s / a } else { 0. };
                    }
                }
                ImageRole::Normal => {
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
                ImageRole::ColorUnweighted | ImageRole::Linear => {}
            }
            out.push(sum.map(|v| v as f32));
        }
    }
    out
}

#[cfg(test)]
mod role_tests {
    use super::*;
    #[test]
    fn opaque_rgb_uses_every_texel_even_with_mixed_or_zero_alpha() {
        for alpha in [0., 1.] {
            let pixels = vec![[1., 0., 0., 0.], [0., 0., 1., alpha]];
            let (coverage, independent) =
                color_variants(2, 1, pixels, TextureFormat::Rgba8Srgb).unwrap();
            assert_eq!(
                &*independent.levels[1],
                &[188, 0, 188, (alpha * 127.5_f32).round() as u8]
            );
            assert_eq!(
                &*coverage.levels[1],
                if alpha == 0. {
                    &[0, 0, 0, 0]
                } else {
                    &[0, 0, 255, 128]
                }
            );
            assert!(std::sync::Arc::ptr_eq(
                &coverage.levels[0],
                &independent.levels[0]
            ));
        }
    }
    #[test]
    fn independent_hdr_odd_filter_preserves_area_weights_and_source_alpha() {
        let pixels = vec![[6., 0., 0., 0.], [0., 3., 0., 0.5], [0., 0., 9., 1.]];
        let (coverage, independent) =
            color_variants(3, 1, pixels, TextureFormat::Rgba32Float).unwrap();
        let values: Vec<f32> = independent.levels[1]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| f32::from_le_bytes(*word))
            .collect();
        assert_eq!(values, [2., 1., 3., 0.5]);
        assert_eq!(coverage.levels[0], independent.levels[0]);
        assert_ne!(coverage.levels[1], independent.levels[1]);
    }
}
