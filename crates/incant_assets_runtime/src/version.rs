//! Sealed immutable cooked versions shared by authoring previews and native players.
//! All construction goes through bounded readers; no constructor accepts mutable
//! caller-built model/texture metadata. These are data integrity checks, not trust
//! in executable content or authentication of a source-recipe fingerprint.
use crate::{CookedAudio, CookedModel, CookedTexture, Result, TextureUsage, invalid};
use incant_types::{VisualAssetKind, VisualAssetRef};
use serde::Serialize;
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub enum RuntimeAssetData {
    Model(CookedModel),
    Texture(CookedTexture),
    Audio(Arc<CookedAudio>),
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
                        .flat_map(|image| &image.levels)
                        .map(Vec::len)
                        .sum::<usize>()
            }
            Self::Texture(texture) => texture.texture.levels.iter().map(Vec::len).sum(),
            Self::Audio(audio) => audio.resident_bytes(),
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
    info: RuntimeAssetInfo,
    usage: Option<TextureUsage>,
    data: RuntimeAssetData,
}
impl RuntimeAsset {
    pub fn info(&self) -> &RuntimeAssetInfo {
        &self.info
    }
    pub fn data(&self) -> &RuntimeAssetData {
        &self.data
    }
    pub fn usage(&self) -> Option<TextureUsage> {
        self.usage
    }
}

impl RuntimeAsset {
    fn create(id: &str, generation: u64, fingerprint: &str, data: RuntimeAssetData) -> Self {
        let (kind, usage) = match &data {
            RuntimeAssetData::Model(_) => ("model", None),
            RuntimeAssetData::Texture(texture) => ("texture", Some(texture.metadata.usage)),
            RuntimeAssetData::Audio(_) => ("audio", None),
        };
        Self {
            info: RuntimeAssetInfo {
                id: id.into(),
                kind: kind.into(),
                fingerprint: fingerprint.into(),
                generation,
                payload_bytes: data.payload_bytes(),
            },
            usage,
            data,
        }
    }
    fn check_id(id: &str) -> Result<()> {
        if id.is_empty() || id.len() > 256 {
            return Err(invalid("asset label exceeds 1..256 bytes"));
        }
        Ok(())
    }
    pub fn load_model(id: &str, generation: u64, cache: &Path, fingerprint: &str) -> Result<Self> {
        Self::check_id(id)?;
        Ok(Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Model(crate::load_model(cache, fingerprint)?),
        ))
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
        Ok(Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Texture(texture),
        ))
    }
    pub fn load_audio(id: &str, generation: u64, cache: &Path, fingerprint: &str) -> Result<Self> {
        Self::check_id(id)?;
        Ok(Self::create(
            id,
            generation,
            fingerprint,
            RuntimeAssetData::Audio(Arc::new(crate::load_audio(cache, fingerprint)?)),
        ))
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
