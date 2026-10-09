use crate::{
    AssetError, MAX_SOURCE_BYTES, Result, SourceSet, Texture, TextureUsage, import_image, invalid,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// glTF texture indices stay stable in material JSON. One source can be used for
/// both color and numeric maps, requiring distinct color space and mip filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTexture {
    pub color: Option<usize>,
    pub linear: Option<usize>,
    pub normal: Option<usize>,
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
    pub wrap_s: u32,
    pub wrap_t: u32,
}
pub(crate) fn import_textures(
    doc: &gltf::Document,
    buffers: &[Vec<u8>],
    sources: &mut SourceSet,
    source: &Path,
) -> Result<(Vec<ModelTexture>, Vec<Texture>)> {
    if doc.images().len() > 256 || doc.textures().len() > 1024 {
        return Err(AssetError::Limit("model textures"));
    }
    let mut usage = vec![BTreeSet::new(); doc.textures().len()];
    let mut mark = |index: usize, coord: u32, kind: TextureUsage| -> Result<()> {
        if coord != 0 {
            return Err(AssetError::Unsupported(
                "texture coordinate set beyond UV0".into(),
            ));
        }
        usage[index].insert(kind);
        Ok(())
    };
    for material in doc.materials() {
        let pbr = material.pbr_metallic_roughness();
        for info in [pbr.base_color_texture(), material.emissive_texture()]
            .into_iter()
            .flatten()
        {
            mark(
                info.texture().index(),
                info.tex_coord(),
                TextureUsage::Color,
            )?;
        }
        if let Some(info) = pbr.metallic_roughness_texture() {
            mark(
                info.texture().index(),
                info.tex_coord(),
                TextureUsage::Linear,
            )?;
        }
        if let Some(info) = material.occlusion_texture() {
            mark(
                info.texture().index(),
                info.tex_coord(),
                TextureUsage::Linear,
            )?;
        }
        if let Some(info) = material.normal_texture() {
            mark(
                info.texture().index(),
                info.tex_coord(),
                TextureUsage::Normal,
            )?;
        }
    }
    let mut encoded = Vec::new();
    let mut source_bytes = 0;
    for image in doc.images() {
        let data = match image.source() {
            gltf::image::Source::Uri { uri, .. } => sources.uri(source, uri)?,
            gltf::image::Source::View { view, .. } => buffers[view.buffer().index()]
                [view.offset()..view.offset() + view.length()]
                .to_vec(),
        };
        if !matches!(
            image::guess_format(&data),
            Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)
        ) {
            return Err(invalid("glTF images require PNG or JPEG"));
        }
        source_bytes += data.len();
        if source_bytes > MAX_SOURCE_BYTES {
            return Err(AssetError::Limit("model image source bytes"));
        }
        encoded.push(data);
    }
    let mut variants = BTreeMap::new();
    let mut images = Vec::new();
    let mut textures = Vec::new();
    let mut total_bytes = 0;
    for texture in doc.textures() {
        let sampler = texture.sampler();
        let mut record = ModelTexture {
            color: None,
            linear: None,
            normal: None,
            mag_filter: sampler.mag_filter().map(|v| v.as_gl_enum()),
            min_filter: sampler.min_filter().map(|v| v.as_gl_enum()),
            wrap_s: sampler.wrap_s().as_gl_enum(),
            wrap_t: sampler.wrap_t().as_gl_enum(),
        };
        // Retain unused textures for subsequent material edits.
        if usage[texture.index()].is_empty() {
            usage[texture.index()].insert(TextureUsage::Linear);
        }
        for &kind in &usage[texture.index()] {
            let key = (texture.source().index(), kind);
            let id = if let Some(&id) = variants.get(&key) {
                id
            } else {
                let image = import_image(&encoded[key.0], kind)?;
                total_bytes += image.levels.iter().map(Vec::len).sum::<usize>();
                if total_bytes > MAX_SOURCE_BYTES {
                    return Err(AssetError::Limit("decoded model textures"));
                }
                let id = images.len();
                images.push(image);
                variants.insert(key, id);
                id
            };
            match kind {
                TextureUsage::Color => record.color = Some(id),
                TextureUsage::Linear => record.linear = Some(id),
                TextureUsage::Normal => record.normal = Some(id),
            }
        }
        textures.push(record);
    }
    Ok((textures, images))
}
pub(crate) fn validate_textures(textures: &[ModelTexture], count: usize) -> Result<()> {
    for t in textures {
        if [t.color, t.linear, t.normal]
            .into_iter()
            .flatten()
            .any(|i| i >= count)
            || [t.color, t.linear, t.normal].iter().all(Option::is_none)
            || t.mag_filter.is_some_and(|v| ![9728, 9729].contains(&v))
            || t.min_filter
                .is_some_and(|v| ![9728, 9729, 9984, 9985, 9986, 9987].contains(&v))
            || ![33071, 33648, 10497].contains(&t.wrap_s)
            || ![33071, 33648, 10497].contains(&t.wrap_t)
        {
            return Err(invalid("invalid model texture binding"));
        }
    }
    Ok(())
}
