//! Atlas source snapshots, edge extrusion and ordinary cooked texture publication.
use crate::{
    AssetError, CookedTexture, Result, SourceSet, Texture, TextureFormat, TextureMetadata,
    TextureUsage, import_image, invalid,
};
use incant_2d::atlas::{SpriteAtlas, pack};
use std::{collections::BTreeMap, path::Path};

pub(crate) fn cook(
    root: &Path,
    source: &Path,
    cache: &Path,
    usage: TextureUsage,
) -> Result<CookedTexture> {
    if usage != TextureUsage::Color {
        return Err(invalid("sprite atlases require color interpretation"));
    }
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read(source)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(AssetError::Limit("atlas descriptor bytes"));
    }
    let descriptor: SpriteAtlas = serde_json::from_slice(&bytes)?;
    descriptor.validate()?;
    let dependency = sources.dependencies().remove(0);
    let mut images: BTreeMap<String, Texture> = BTreeMap::new();
    let mut regions = BTreeMap::new();
    let mut sizes = BTreeMap::new();
    let mut texels = 0_u64;
    for (id, frame) in &descriptor.frames {
        if !images.contains_key(&frame.path) {
            let path = source.parent().unwrap_or(Path::new("")).join(&frame.path);
            let texture = import_image(&sources.read(&path)?, TextureUsage::Color)?;
            texels += u64::from(texture.width) * u64::from(texture.height);
            if texels > 4 * 1024 * 1024 {
                return Err(AssetError::Limit("atlas decoded source texels"));
            }
            images.insert(frame.path.clone(), texture);
        }
        let image = &images[&frame.path];
        let region = frame.region.unwrap_or([0, 0, image.width, image.height]);
        let [x, y, w, h] = region;
        if w == 0
            || h == 0
            || u64::from(x) + u64::from(w) > u64::from(image.width)
            || u64::from(y) + u64::from(h) > u64::from(image.height)
        {
            return Err(invalid("atlas source rectangle exceeds image dimensions"));
        }
        sizes.insert(id.clone(), [w, h]);
        regions.insert(id.clone(), region);
    }
    let layout = pack(&descriptor, &sizes)?;
    let mut metadata = TextureMetadata {
        version: 2,
        fingerprint: String::new(),
        dependency: dependency.clone(),
        usage,
        content_sha256: String::new(),
        dependencies: sources
            .dependencies()
            .into_iter()
            .filter(|d| d.path != dependency.path)
            .collect(),
        atlas: Some(layout.clone()),
    };
    metadata.fingerprint = metadata.expected_key()?;
    if let Ok(cached) = crate::load_texture(cache, &metadata.fingerprint) {
        return Ok(cached);
    }
    let [width, height] = layout.size;
    let mut pixels = vec![[0.; 4]; width as usize * height as usize];
    for (id, frame) in &layout.frames {
        let image = &images[&descriptor.frames[id].path];
        let [sx, sy, w, h] = regions[id];
        let [x, y, _, _] = frame.region;
        let p = layout.padding;
        for dy in 0..h + p * 2 {
            for dx in 0..w + p * 2 {
                let source_x = sx + dx.saturating_sub(p).min(w - 1);
                let source_y = sy + dy.saturating_sub(p).min(h - 1);
                pixels[((y - p + dy) * width + x - p + dx) as usize] =
                    linear_texel(image, source_x, source_y)?;
            }
        }
    }
    let format = if images
        .values()
        .any(|t| t.format == TextureFormat::Rgba32Float)
    {
        TextureFormat::Rgba32Float
    } else {
        TextureFormat::Rgba8Srgb
    };
    let texture = crate::texture::from_linear(width, height, pixels, format, usage)?;
    crate::texture_cache::store(cache, metadata, texture)
}
fn linear_texel(image: &Texture, x: u32, y: u32) -> Result<[f32; 4]> {
    let i = (y as usize * image.width as usize + x as usize) * image.format.bytes_per_pixel();
    let mut value = if image.format == TextureFormat::Rgba32Float {
        std::array::from_fn(|c| {
            f32::from_le_bytes(
                image.levels[0][i + c * 4..i + c * 4 + 4]
                    .try_into()
                    .unwrap(),
            )
        })
    } else {
        std::array::from_fn(|c| f32::from(image.levels[0][i + c]) / 255.)
    };
    if value.iter().any(|v| !v.is_finite() || *v < 0.) || value[3] > 1. {
        return Err(invalid(
            "atlas source contains invalid sprite radiance or alpha",
        ));
    }
    if image.format == TextureFormat::Rgba8Srgb {
        for v in &mut value[..3] {
            *v = crate::texture::decode_srgb(*v);
        }
    }
    Ok(value)
}
