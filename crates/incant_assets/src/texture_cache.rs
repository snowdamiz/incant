use crate::{
    AssetError, Result, SourceSet, Texture, TextureUsage, encode_ktx2, import_image, invalid,
    sha256,
};
use incant_assets_runtime::texture_cache::path;
pub use incant_assets_runtime::{CookedTexture, TextureMetadata};
use std::{fs, io::Write, path::Path};
fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| AssetError::Io(e.error))?;
    Ok(())
}
pub fn load_texture(cache: &Path, key: &str) -> Result<CookedTexture> {
    incant_assets_runtime::load_texture(cache, key).map_err(Into::into)
}
pub fn cook_texture(
    root: &Path,
    source: &Path,
    cache: &Path,
    usage: TextureUsage,
) -> Result<CookedTexture> {
    if source
        .file_name()
        .and_then(|p| p.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".atlas.json"))
    {
        return crate::atlas::cook(root, source, cache, usage);
    }
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read(source)?;
    let dependency = sources
        .dependencies()
        .into_iter()
        .next()
        .ok_or_else(|| invalid("missing texture source"))?;
    let mut metadata = TextureMetadata {
        version: if usage == TextureUsage::Color { 3 } else { 1 },
        fingerprint: String::new(),
        dependency,
        usage,
        content_sha256: String::new(),
        unweighted_content_sha256: None,
        dependencies: Vec::new(),
        atlas: None,
    };
    metadata.fingerprint = metadata.expected_key()?;
    if let Ok(cached) = load_texture(cache, &metadata.fingerprint) {
        return Ok(cached);
    }
    let texture = import_image(&bytes, usage)?;
    let color_unweighted = if usage == TextureUsage::Color {
        crate::texture::color_pair_limit(&texture)?;
        let mut independent =
            crate::texture::import_image_role(&bytes, crate::texture::ImageRole::ColorUnweighted)?;
        independent.share_base_from(&texture)?;
        Some(independent)
    } else {
        None
    };
    store(cache, metadata, texture, color_unweighted)
}
pub(crate) fn store(
    cache: &Path,
    mut metadata: TextureMetadata,
    texture: Texture,
    mut color_unweighted: Option<Texture>,
) -> Result<CookedTexture> {
    if (metadata.version == 3) != color_unweighted.is_some()
        || (metadata.version == 3 && metadata.usage != TextureUsage::Color)
    {
        return Err(invalid("cooked color roles do not match the manifest"));
    }
    if let Some(independent) = &mut color_unweighted {
        crate::texture::color_pair_limit(&texture)?;
        independent.share_base_from(&texture)?;
    }
    let ktx = encode_ktx2(&texture)?;
    metadata.content_sha256 = sha256(&ktx);
    let independent_ktx = color_unweighted.as_ref().map(encode_ktx2).transpose()?;
    if independent_ktx
        .as_ref()
        .is_some_and(|bytes| bytes.len() > crate::MAX_SOURCE_BYTES.saturating_sub(ktx.len()))
    {
        return Err(AssetError::Limit("combined color KTX2 bytes"));
    }
    metadata.unweighted_content_sha256 = independent_ktx.as_ref().map(|bytes| sha256(bytes));
    if metadata.expected_key()? != metadata.fingerprint {
        return Err(invalid("cooked texture key mismatch"));
    }
    fs::create_dir_all(cache)?;
    write(&path(cache, &metadata.content_sha256, "ktx2")?, &ktx)?;
    if let (Some(hash), Some(bytes)) = (&metadata.unweighted_content_sha256, independent_ktx) {
        write(&path(cache, hash, "ktx2")?, &bytes)?;
    }
    write(
        &path(cache, &metadata.fingerprint, "json")?,
        &serde_json::to_vec(&metadata)?,
    )?;
    Ok(CookedTexture {
        metadata,
        texture,
        color_unweighted,
        cache_hit: false,
    })
}
