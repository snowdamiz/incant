use super::cube::Cube;
use crate::{ResourceError, Result};
use glam::Vec3;
use incant_assets::{CookedTexture, TextureFormat, TextureUsage};
use std::f32::consts::PI;

pub(super) fn decode(source: &CookedTexture) -> Result<Cube> {
    let texture = &source.texture;
    texture.validate()?;
    if source.metadata.usage == TextureUsage::Normal {
        return Err(ResourceError::EnvironmentSource("normal maps cannot provide radiance").into());
    }
    if texture.width != texture.height * 2 || texture.width > 4096 {
        return Err(ResourceError::EnvironmentSource(
            "expected a 2:1 equirectangular image, at most 4096×2048",
        )
        .into());
    }
    let texel = |level: usize, x: i32, y: i32| {
        let width = (texture.width >> level).max(1);
        let height = (texture.height >> level).max(1);
        let x = x.rem_euclid(width as i32) as usize;
        let y = y.clamp(0, height as i32 - 1) as usize;
        let index = y * width as usize + x;
        let bytes = &texture.levels[level];
        match texture.format {
            TextureFormat::Rgba32Float => Vec3::from_array(std::array::from_fn(|channel| {
                let offset = index * 16 + channel * 4;
                f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
            })),
            format => Vec3::from_array(std::array::from_fn(|channel| {
                let c = bytes[index * 4 + channel] as f32 / 255.;
                if format == TextureFormat::Rgba8Srgb {
                    if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    }
                } else {
                    c
                }
            })),
        }
    };
    // Reject unsupported HDR radiance instead of hiding negatives/overflow in a clamp.
    if texture.format == TextureFormat::Rgba32Float {
        for bytes in &texture.levels {
            for pixel in bytes.as_chunks::<16>().0 {
                if pixel[..12]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|word| f32::from_le_bytes(*word))
                    .any(|value| !(0.0..=65504.0).contains(&value))
                {
                    return Err(ResourceError::EnvironmentSource(
                        "radiance must be finite and within 0..65504",
                    )
                    .into());
                }
            }
        }
    }
    // Use the cooker's color-correct low-pass mip when the source is much larger
    // than the cube. Avoid point-sampling tiny highlights into random bright texels.
    let level = (texture.width / (4 * super::filter::SPECULAR_SIZE))
        .max(1)
        .ilog2() as usize;
    let width = (texture.width >> level).max(1);
    let height = (texture.height >> level).max(1);
    Ok(Cube::from_fn(super::filter::SPECULAR_SIZE, |d| {
        // +X is the center, +Z is three-quarter U, +Y is the north pole.
        // Image rows run from north to south. Alpha has no lighting meaning.
        let x = (d.z.atan2(d.x) / (2. * PI) + 0.5) * width as f32 - 0.5;
        let y = d.y.clamp(-1., 1.).acos() / PI * height as f32 - 0.5;
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        texel(level, ix, iy)
            .lerp(texel(level, ix + 1, iy), x - x.floor())
            .lerp(
                texel(level, ix, iy + 1).lerp(texel(level, ix + 1, iy + 1), x - x.floor()),
                y - y.floor(),
            )
    }))
}
