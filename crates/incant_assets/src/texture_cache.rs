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
    /// Additional sources of an atlas. Ordinary v1 image metadata is unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atlas: Option<incant_2d::atlas::AtlasLayout>,
}
impl TextureMetadata {
    pub fn all_dependencies(&self) -> Vec<Dependency> {
        std::iter::once(self.dependency.clone())
            .chain(self.dependencies.iter().cloned())
            .collect()
    }
    pub(crate) fn expected_key(&self) -> Result<String> {
        if !valid_key(&self.dependency.sha256) {
            return Err(invalid("invalid texture dependency hash"));
        }
        let value = if self.version == 1 && self.atlas.is_none() && self.dependencies.is_empty() {
            serde_json::to_vec(&("incant-ktx2-v1", &self.dependency, self.usage))?
        } else if self.version == 2
            && self.usage == TextureUsage::Color
            && !self.dependencies.is_empty()
            && self.dependencies.len() <= 256
        {
            let atlas = self
                .atlas
                .as_ref()
                .ok_or_else(|| invalid("atlas metadata missing"))?;
            atlas.validate()?;
            let mut seen = std::collections::BTreeSet::from([self.dependency.path.as_str()]);
            for dependency in &self.dependencies {
                if !valid_key(&dependency.sha256) || !seen.insert(&dependency.path) {
                    return Err(invalid("invalid atlas dependencies"));
                }
            }
            serde_json::to_vec(&(
                "incant-atlas-v1",
                &self.dependency,
                &self.dependencies,
                self.usage,
                atlas,
            ))?
        } else {
            return Err(invalid("texture manifest version/key mismatch"));
        };
        Ok(sha256(&value))
    }
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
        serde_json::from_slice(&read(&path(cache, key, "json")?, 2 * 1024 * 1024)?)?;
    if metadata.fingerprint != key || metadata.expected_key()? != key {
        return Err(invalid("texture manifest version/key mismatch"));
    }
    let ktx = read(
        &path(cache, &metadata.content_sha256, "ktx2")?,
        crate::MAX_SOURCE_BYTES,
    )?;
    if sha256(&ktx) != metadata.content_sha256 {
        return Err(invalid("texture cache checksum mismatch"));
    }
    let texture = decode_ktx2(&ktx)?;
    if metadata
        .atlas
        .as_ref()
        .is_some_and(|atlas| atlas.size != [texture.width, texture.height])
    {
        return Err(invalid("atlas layout does not match the cooked image"));
    }
    Ok(CookedTexture {
        metadata,
        texture,
        cache_hit: true,
    })
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
