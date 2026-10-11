use crate::{AssetError, Dependency, Result, Texture, TextureUsage, decode_ktx2, invalid, sha256};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
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
    /// Required for Color v3. Legacy manifests and non-Color images omit it.
    /// This is a corruption digest, not authentication of source data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unweighted_content_sha256: Option<String>,
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
    /// Validate manifest identity fields and reproduce the existing cache key.
    /// Source dependency names/hashes are metadata; this never reads those sources.
    pub fn expected_key(&self) -> Result<String> {
        if !valid_key(&self.dependency.sha256) {
            return Err(invalid("invalid texture dependency hash"));
        }
        let modern = self.version == 3 && self.usage == TextureUsage::Color;
        if !modern && self.unweighted_content_sha256.is_some() {
            return Err(invalid("legacy texture cannot declare color role variants"));
        }
        let value = if (self.version == 1 || modern)
            && self.atlas.is_none()
            && self.dependencies.is_empty()
        {
            let recipe = if modern {
                "incant-ktx2-v2-color-roles-v1"
            } else {
                "incant-ktx2-v1"
            };
            serde_json::to_vec(&(recipe, &self.dependency, self.usage))?
        } else if (self.version == 2 || modern)
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
                if modern {
                    "incant-atlas-v2-color-roles-v1"
                } else {
                    "incant-atlas-v1"
                },
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
    /// Alpha-independent color mips. Level zero shares the primary allocation
    /// after the reader checks exact layout and bytes. Absent only in legacy or
    /// non-Color forms; sprites always use `texture`.
    pub color_unweighted: Option<Texture>,
    pub cache_hit: bool,
}
impl CookedTexture {
    /// Source-free selection for a consumer that ignores image alpha. Preserve
    /// the historical chain for pinned old manifests; never hide missing v3 data.
    pub fn radiance_texture(&self) -> Result<&Texture> {
        if self.metadata.version == 3 {
            if self.metadata.usage != TextureUsage::Color {
                return Err(invalid("color role manifest has non-color usage"));
            }
            self.color_unweighted
                .as_ref()
                .ok_or_else(|| invalid("missing independent color mip chain"))
        } else {
            Ok(&self.texture)
        }
    }
}
fn valid_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Shared cache filename construction; the extension is an engine-owned suffix.
/// Does not read or write files.
pub fn path(cache: &Path, key: &str, extension: &str) -> Result<PathBuf> {
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
pub fn load_texture(cache: &Path, key: &str) -> Result<CookedTexture> {
    let metadata: TextureMetadata =
        serde_json::from_slice(&read(&path(cache, key, "json")?, 2 * 1024 * 1024)?)?;
    if metadata.fingerprint != key || metadata.expected_key()? != key {
        return Err(invalid("texture manifest version/key mismatch"));
    }
    let independent_hash = if metadata.version == 3 {
        Some(
            metadata
                .unweighted_content_sha256
                .as_deref()
                .filter(|hash| valid_key(hash))
                .ok_or_else(|| invalid("missing or invalid independent color digest"))?,
        )
    } else {
        None
    };
    let ktx = read(
        &path(cache, &metadata.content_sha256, "ktx2")?,
        crate::MAX_SOURCE_BYTES,
    )?;
    if sha256(&ktx) != metadata.content_sha256 {
        return Err(invalid("texture cache checksum mismatch"));
    }
    // Bound both serialized chains before decoding the second. The same digest
    // still counts twice logically: sharing does not relax the resource limit.
    let independent_ktx = independent_hash
        .map(|hash| {
            let bytes = read(
                &path(cache, hash, "ktx2")?,
                crate::MAX_SOURCE_BYTES - ktx.len(),
            )?;
            if sha256(&bytes) != hash {
                return Err(invalid("independent color cache checksum mismatch"));
            }
            Ok(bytes)
        })
        .transpose()?;
    let texture = decode_ktx2(&ktx)?;
    let color_unweighted = independent_ktx
        .map(|bytes| {
            let mut independent = decode_ktx2(&bytes)?;
            if !matches!(
                texture.format,
                crate::TextureFormat::Rgba8Srgb | crate::TextureFormat::Rgba32Float
            ) {
                return Err(invalid("color role texture has non-color format"));
            }
            if texture
                .payload_bytes()
                .checked_add(independent.payload_bytes())
                .is_none_or(|bytes| bytes > crate::MAX_SOURCE_BYTES)
            {
                return Err(AssetError::Limit("combined color mip bytes"));
            }
            independent.share_base_from(&texture)?;
            Ok(independent)
        })
        .transpose()?;
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
        color_unweighted,
        cache_hit: true,
    })
}
