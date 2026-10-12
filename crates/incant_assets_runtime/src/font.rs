//! Lossless, bounded packaged sfnt fonts. Runtime reads cooked files only.
use crate::{AssetError, Dependency, Result, invalid, sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub use incant_text::MAX_FONT_BYTES;
pub const FONT_MAGIC: &[u8; 8] = b"INCTFNT\0";
pub const FONT_HEADER_BYTES: usize = 48;
pub const MAX_FONT_MANIFEST_BYTES: usize = 8192;
const COOK_VERSION: &str = "incant-sfnt-v1-cosmic-text-0.14.2-rustybuzz-0.14.1-ttf-parser-0.21.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontMetadata {
    pub version: u32,
    pub fingerprint: String,
    pub dependency: Dependency,
    pub bytes: u32,
    pub content_sha256: String,
}

/// Immutable validated font bytes; consumers move them into the shaper at load
/// time or explicitly copy them when constructing another independent engine.
#[derive(Debug)]
pub struct CookedFont {
    metadata: FontMetadata,
    bytes: Vec<u8>,
    pub cache_hit: bool,
}
impl CookedFont {
    pub fn metadata(&self) -> &FontMetadata {
        &self.metadata
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_source(self, id: ulid::Ulid) -> incant_text::FontSource {
        incant_text::FontSource {
            id,
            bytes: self.bytes,
        }
    }
}

pub fn validate_source(bytes: &[u8]) -> Result<()> {
    incant_text::validate_font(bytes).map_err(|error| match error {
        incant_text::TextError::Limit(_) => AssetError::Limit("font bytes"),
        _ => invalid("font is not a loadable single sfnt face"),
    })
}
fn valid_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_dependency(dependency: &Dependency) -> bool {
    let path = Path::new(&dependency.path);
    !dependency.path.is_empty()
        && dependency.path.len() <= 4096
        && !dependency.path.contains(['\\', ':', '\0'])
        && path.components().all(|c| matches!(c, Component::Normal(_)))
        && path
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
            == dependency.path
        && valid_key(&dependency.sha256)
}
pub fn font_key(dependency: &Dependency) -> Result<String> {
    if !valid_dependency(dependency) {
        return Err(invalid("invalid font source dependency"));
    }
    Ok(sha256(&serde_json::to_vec(&(COOK_VERSION, dependency))?))
}
pub fn font_path(cache: &Path, key: &str, extension: &str) -> Result<PathBuf> {
    if !valid_key(key) || !matches!(extension, "json" | "ifont") {
        return Err(invalid("invalid font cache key or extension"));
    }
    Ok(cache.join(format!("{key}.{extension}")))
}

fn open_file(path: &Path, limit: usize) -> Result<File> {
    let stat = fs::symlink_metadata(path)?;
    if !stat.is_file() || stat.len() > limit as u64 {
        return Err(invalid("invalid font cache file type or length"));
    }
    let file = File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() > limit as u64 {
        return Err(invalid("invalid font cache file type or length"));
    }
    Ok(file)
}

pub fn load_font(cache: &Path, fingerprint: &str) -> Result<CookedFont> {
    let manifest = font_path(cache, fingerprint, "json")?;
    let mut json = Vec::new();
    open_file(&manifest, MAX_FONT_MANIFEST_BYTES)?
        .take(MAX_FONT_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut json)?;
    if json.len() > MAX_FONT_MANIFEST_BYTES {
        return Err(AssetError::Limit("font manifest bytes"));
    }
    let metadata: FontMetadata = serde_json::from_slice(&json)?;
    if metadata.version != 1
        || metadata.fingerprint != fingerprint
        || !(1..=MAX_FONT_BYTES as u32).contains(&metadata.bytes)
        || !valid_key(&metadata.content_sha256)
        || metadata.content_sha256 != metadata.dependency.sha256
        || font_key(&metadata.dependency)? != fingerprint
    {
        return Err(invalid("font manifest version, dimensions or key mismatch"));
    }
    let blob = font_path(cache, &metadata.content_sha256, "ifont")?;
    let mut file = open_file(&blob, MAX_FONT_BYTES + FONT_HEADER_BYTES)?;
    let length = metadata.bytes as usize;
    if file.metadata()?.len() != (FONT_HEADER_BYTES + length) as u64 {
        return Err(invalid("font cache exact length mismatch"));
    }
    let mut header = [0_u8; FONT_HEADER_BYTES];
    file.read_exact(&mut header)?;
    if header[..8] != FONT_MAGIC[..]
        || u32::from_le_bytes(header[8..12].try_into().unwrap()) != 1
        || u32::from_le_bytes(header[12..16].try_into().unwrap()) != metadata.bytes
    {
        return Err(invalid("font cache header mismatch"));
    }
    let mut bytes = Vec::with_capacity(length);
    file.take(length as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() != length {
        return Err(invalid("font cache exact length mismatch"));
    }
    let digest = Sha256::digest(&bytes);
    if header[16..] != digest[..] || format!("{digest:x}") != metadata.content_sha256 {
        return Err(invalid("font cache checksum mismatch"));
    }
    validate_source(&bytes)?;
    Ok(CookedFont {
        metadata,
        bytes,
        cache_hit: true,
    })
}
