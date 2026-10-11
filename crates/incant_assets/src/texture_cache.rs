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
    let key = sha256(&serde_json::to_vec(&(
        "incant-ktx2-v1",
        &dependency,
        usage,
    ))?);
    if let Ok(cached) = load_texture(cache, &key) {
        return Ok(cached);
    }
    let texture = import_image(&bytes, usage)?;
    let metadata = TextureMetadata {
        version: 1,
        fingerprint: key,
        dependency,
        usage,
        content_sha256: String::new(),
        dependencies: Vec::new(),
        atlas: None,
    };
    store(cache, metadata, texture)
}
pub(crate) fn store(
    cache: &Path,
    mut metadata: TextureMetadata,
    texture: Texture,
) -> Result<CookedTexture> {
    let ktx = encode_ktx2(&texture)?;
    metadata.content_sha256 = sha256(&ktx);
    fs::create_dir_all(cache)?;
    write(&path(cache, &metadata.content_sha256, "ktx2")?, &ktx)?;
    write(
        &path(cache, &metadata.fingerprint, "json")?,
        &serde_json::to_vec(&metadata)?,
    )?;
    Ok(CookedTexture {
        metadata,
        texture,
        cache_hit: false,
    })
}
