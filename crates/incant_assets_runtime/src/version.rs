//! Sealed immutable cooked versions shared by authoring previews and native players.
//! All construction goes through bounded readers; no constructor accepts mutable
//! caller-built model/texture metadata. These are data integrity checks, not trust
//! in executable content or authentication of a source-recipe fingerprint.
use crate::{CookedAudio, CookedModel, CookedTexture, Result, TextureUsage, invalid};
use incant_types::{VisualAssetKind, VisualAssetRef};
use serde::Serialize;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

/// Process-local identity of one successfully loaded immutable version. It is
/// neither a recipe fingerprint nor a persisted/content identifier. Separate
/// loads cannot alias even if their caller labels and recipe keys are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetVersionId(u64);
static NEXT_VERSION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum RuntimeAssetData {
    Model(CookedModel),
    Texture(CookedTexture),
    Audio(Arc<CookedAudio>),
    #[cfg(feature = "font")]
    Font(crate::CookedFont),
}
impl RuntimeAssetData {
    /// Decoded vertex/index/mip payload, excluding Rust metadata and allocator
    /// overhead. Audio counts its manifest; PCM stays in a validated file and
    /// mixer/decoder buffers have separate bounds. Staged replacements and
    /// retained versions add to the process peak, outside this published-set cap.
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Model(model) => {
                model
                    .meshes
                    .iter()
                    .map(|mesh| {
                        mesh.vertices.len() * size_of::<crate::Vertex>()
                            + mesh.indices.len() * size_of::<u32>()
                    })
                    .sum::<usize>()
                    + model
                        .images
                        .iter()
                        .map(crate::Texture::payload_bytes)
                        .sum::<usize>()
            }
            Self::Texture(texture) => std::iter::once(&texture.texture)
                .chain(texture.color_unweighted.as_ref())
                .map(crate::Texture::payload_bytes)
                .sum(),
            Self::Audio(audio) => audio.resident_bytes(),
            #[cfg(feature = "font")]
            Self::Font(font) => font.bytes().len(),
        }
    }

    fn resident_payload_bytes(&self) -> usize {
        match self {
            Self::Model(model) => {
                self.payload_bytes()
                    - model
                        .images
                        .iter()
                        .map(crate::Texture::payload_bytes)
                        .sum::<usize>()
                    + crate::texture::resident_bytes(&model.images)
            }
            Self::Texture(texture) => crate::texture::resident_bytes(
                std::iter::once(&texture.texture).chain(texture.color_unweighted.as_ref()),
            ),
            Self::Audio(audio) => audio.resident_bytes(),
            #[cfg(feature = "font")]
            Self::Font(font) => font.bytes().len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeAssetInfo {
    pub id: String,
    pub kind: String,
    pub fingerprint: String,
    /// Monotonically increasing within a store, including remove/re-add cycles.
    pub generation: u64,
    pub payload_bytes: usize,
}

#[derive(Debug)]
pub struct RuntimeAsset {
    version: AssetVersionId,
    info: RuntimeAssetInfo,
    usage: Option<TextureUsage>,
    data: RuntimeAssetData,
    resident_payload_bytes: usize,
}
impl RuntimeAsset {
    pub fn version_id(&self) -> AssetVersionId {
        self.version
    }
    pub fn info(&self) -> &RuntimeAssetInfo {
        &self.info
    }
    pub fn data(&self) -> &RuntimeAssetData {
        &self.data
    }
    pub fn usage(&self) -> Option<TextureUsage> {
        self.usage
    }
    /// Unique retained CPU payload, excluding allocation metadata. The separate
    /// info.payload_bytes remains the conservative logical budget; neither is a
    /// measurement of GPU allocations or serialized disk bytes.
    pub fn resident_payload_bytes(&self) -> usize {
        self.resident_payload_bytes
    }
}

impl RuntimeAsset {
    fn create(
        id: &str,
        generation: u64,
        fingerprint: &str,
        data: RuntimeAssetData,
    ) -> Result<Self> {
        let animation_bytes = match &data {
            RuntimeAssetData::Model(model) => model
                .animation
                .as_ref()
                .map(crate::animation::AnimationData::retained_bytes)
                .transpose()?
                .unwrap_or(0),
            _ => 0,
        };
        let (kind, usage) = match &data {
            RuntimeAssetData::Model(_) => ("model", None),
            RuntimeAssetData::Texture(texture) => ("texture", Some(texture.metadata.usage)),
            RuntimeAssetData::Audio(_) => ("audio", None),
            #[cfg(feature = "font")]
            RuntimeAssetData::Font(_) => ("font", None),
        };
        let version = NEXT_VERSION
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| invalid("loaded asset version identity exhausted"))?;
        Ok(Self {
            version: AssetVersionId(version),
            info: RuntimeAssetInfo {
                id: id.into(),
                kind: kind.into(),
                fingerprint: fingerprint.into(),
                generation,
                payload_bytes: data
                    .payload_bytes()
                    .checked_add(animation_bytes)
                    .ok_or(crate::AssetError::Limit("model animation payload"))?,
            },
            usage,
            resident_payload_bytes: data
                .resident_payload_bytes()
                .checked_add(animation_bytes)
                .ok_or(crate::AssetError::Limit("model animation payload"))?,
            data,
        })
    }
    fn check_id(id: &str) -> Result<()> {
        if id.is_empty() || id.len() > 256 {
            return Err(invalid("asset label exceeds 1..256 bytes"));
        }
        Ok(())
    }
    pub fn load_model(id: &str, generation: u64, cache: &Path, fingerprint: &str) -> Result<Self> {
        Self::check_id(id)?;
        let model = crate::load_model(cache, fingerprint)?;
        Self::create(id, generation, fingerprint, RuntimeAssetData::Model(model))
    }
    pub fn load_texture(
        id: &str,
        generation: u64,
        cache: &Path,
        fingerprint: &str,
        expected_usage: Option<TextureUsage>,
    ) -> Result<Self> {
        Self::check_id(id)?;
        let texture = crate::load_texture(cache, fingerprint)?;
        if expected_usage.is_some_and(|usage| usage != texture.metadata.usage) {
            return Err(invalid("texture settings do not match the cooked payload"));
        }
        Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Texture(texture),
        )
    }
    pub fn load_audio(id: &str, generation: u64, cache: &Path, fingerprint: &str) -> Result<Self> {
        Self::check_id(id)?;
        Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Audio(Arc::new(crate::load_audio(cache, fingerprint)?)),
        )
    }
    #[cfg(feature = "font")]
    pub fn load_font(id: &str, generation: u64, cache: &Path, fingerprint: &str) -> Result<Self> {
        Self::check_id(id)?;
        Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Font(crate::load_font(cache, fingerprint)?),
        )
    }
    /// Startup-only adapter from a binary package declaration. The caller retains
    /// the binary ID as its lookup key; this informational label is formatted once.
    /// Cache points at the model or texture cache directory selected by the caller.
    pub fn load_visual(reference: &VisualAssetRef, generation: u64, cache: &Path) -> Result<Self> {
        use std::fmt::Write;
        let mut id = String::with_capacity(32);
        let mut fingerprint = String::with_capacity(64);
        for byte in reference.id.0 {
            write!(&mut id, "{byte:02x}").unwrap();
        }
        for byte in reference.fingerprint {
            write!(&mut fingerprint, "{byte:02x}").unwrap();
        }
        match reference.kind {
            VisualAssetKind::Model => Self::load_model(&id, generation, cache, &fingerprint),
            VisualAssetKind::Texture => {
                Self::load_texture(&id, generation, cache, &fingerprint, None)
            }
        }
    }
}
