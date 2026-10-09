use crate::{
    AssetError, Dependency, Result, SourceSet, Texture, TextureUsage, decode_ktx2, encode_ktx2,
    import_image, invalid, sha256,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureMetadata {
    pub version: u32,
    pub fingerprint: String,
    pub dependency: Dependency,
    pub usage: TextureUsage,
    pub content_sha256: String,
}
#[derive(Debug)]
pub struct CookedTexture {
    pub metadata: TextureMetadata,
    pub texture: Texture,
    pub cache_hit: bool,
}
fn valid_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn path(cache: &Path, key: &str, extension: &str) -> Result<PathBuf> {
    if !valid_key(key) {
        return Err(invalid("invalid texture cache key"));
    }
    Ok(cache.join(format!("{key}.{extension}")))
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err(invalid("invalid texture cache entry"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(AssetError::Limit("texture cache bytes"));
    }
    Ok(bytes)
}
fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| AssetError::Io(e.error))?;
    Ok(())
}
pub fn load_texture(cache: &Path, key: &str) -> Result<CookedTexture> {
    let metadata: TextureMetadata =
        serde_json::from_slice(&read(&path(cache, key, "json")?, 16 * 1024)?)?;
    if metadata.version != 1
        || metadata.fingerprint != key
        || !valid_key(&metadata.dependency.sha256)
        || sha256(&serde_json::to_vec(&(
            "incant-ktx2-v1",
            &metadata.dependency,
            metadata.usage,
        ))?) != key
    {
        return Err(invalid("texture manifest version/key mismatch"));
    }
    let ktx = read(
        &path(cache, &metadata.content_sha256, "ktx2")?,
        crate::MAX_SOURCE_BYTES,
    )?;
    if sha256(&ktx) != metadata.content_sha256 {
        return Err(invalid("texture cache checksum mismatch"));
    }
    Ok(CookedTexture {
        metadata,
        texture: decode_ktx2(&ktx)?,
        cache_hit: true,
    })
}
pub fn cook_texture(
    root: &Path,
    source: &Path,
    cache: &Path,
    usage: TextureUsage,
) -> Result<CookedTexture> {
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
    let ktx = encode_ktx2(&texture)?;
    let metadata = TextureMetadata {
        version: 1,
        fingerprint: key.clone(),
        dependency,
        usage,
        content_sha256: sha256(&ktx),
    };
    fs::create_dir_all(cache)?;
    write(&path(cache, &metadata.content_sha256, "ktx2")?, &ktx)?;
    write(&path(cache, &key, "json")?, &serde_json::to_vec(&metadata)?)?;
    Ok(CookedTexture {
        metadata,
        texture,
        cache_hit: false,
    })
}
