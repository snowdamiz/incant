use crate::{AudioError, Result};
use incant_assets_runtime::{CookedAudio, load_audio};
use incant_runtime::StableId;
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub const MAX_SCENE_CLIPS: usize = 128;
pub const MAX_CLIP_METADATA_BYTES: usize = 4 * 1024 * 1024;

/// Startup-only binding from the cooked package's ID/key table. The cache loader
/// verifies the expected fingerprint, manifest, dimensions and every PCM chunk.
/// No API exposes mutable metadata or substitutes a different clip under an ID.
#[derive(Clone)]
pub struct ClipBinding {
    pub(super) id: StableId,
    pub(super) clip: Arc<CookedAudio>,
}
impl ClipBinding {
    pub fn load(id: StableId, cache: &Path, expected_fingerprint: &str) -> Result<Self> {
        Ok(Self {
            id,
            clip: Arc::new(load_audio(cache, expected_fingerprint)?),
        })
    }
}
pub(super) fn collect(bindings: Vec<ClipBinding>) -> Result<BTreeMap<StableId, ClipBinding>> {
    if bindings.len() > MAX_SCENE_CLIPS
        || bindings.iter().map(metadata_bytes).sum::<usize>() > MAX_CLIP_METADATA_BYTES
    {
        return Err(AudioError::Capacity);
    }
    let mut result = BTreeMap::new();
    for binding in bindings {
        if result.insert(binding.id, binding).is_some() {
            return Err(AudioError::Invalid("duplicate cooked clip ID"));
        }
    }
    Ok(result)
}
pub(super) fn name(id: StableId) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(32);
    for byte in id.0 {
        write!(&mut out, "{byte:02x}").unwrap();
    }
    out
}

// Count retained allocation capacities, including the Vec<String> allocation;
// the asset store's logical resident estimate is not a hard allocation budget.
fn metadata_bytes(binding: &ClipBinding) -> usize {
    let m = &binding.clip.metadata;
    std::mem::size_of::<CookedAudio>()
        + m.fingerprint.capacity()
        + m.dependency.path.capacity()
        + m.dependency.sha256.capacity()
        + m.content_sha256.capacity()
        + m.chunk_sha256.capacity() * std::mem::size_of::<String>()
        + m.chunk_sha256.iter().map(String::capacity).sum::<usize>()
}
