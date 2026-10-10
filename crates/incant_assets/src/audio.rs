//! Validated, seekable PCM cache. Retained file handles survive atomic reimports;
//! each bounded read rechecks its chunk before exposing samples to a mixer.
use crate::{AssetError, Dependency, Result, invalid, sha256};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub const AUDIO_CHUNK_FRAMES: usize = 8192;
pub const MAX_AUDIO_PCM_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const FRAME_BYTES: usize = 8;
pub(crate) const COOK_VERSION: &str = "incant-pcm-f32-v1-symphonia-0.6.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioMetadata {
    pub version: u32,
    pub fingerprint: String,
    pub dependency: Dependency,
    pub sample_rate: u32,
    /// Source channels; mono is duplicated to stereo in the cooked frames.
    pub channels: u8,
    pub frames: u64,
    pub content_sha256: String,
    pub chunk_sha256: Vec<String>,
}
#[derive(Debug)]
pub struct CookedAudio {
    pub metadata: AudioMetadata,
    pub cache_hit: bool,
    file: Mutex<File>,
}
impl CookedAudio {
    /// Read one indexed chunk. No whole-clip allocation or source-file access.
    pub fn read_chunk(&self, index: usize) -> Result<Vec<[f32; 2]>> {
        let bytes = self.read_bytes(index)?;
        let samples: Vec<_> = bytes
            .as_chunks::<FRAME_BYTES>()
            .0
            .iter()
            .map(|b| {
                [
                    f32::from_le_bytes(b[..4].try_into().unwrap()),
                    f32::from_le_bytes(b[4..].try_into().unwrap()),
                ]
            })
            .collect();
        if samples.iter().flatten().any(|v| !v.is_finite()) {
            return Err(invalid("audio samples must be finite"));
        }
        Ok(samples)
    }
    fn read_bytes(&self, index: usize) -> Result<Vec<u8>> {
        let expected = self
            .metadata
            .chunk_sha256
            .get(index)
            .ok_or_else(|| invalid("audio chunk is outside the clip"))?;
        let start = index * AUDIO_CHUNK_FRAMES;
        let frames = (self.metadata.frames as usize - start).min(AUDIO_CHUNK_FRAMES);
        let mut bytes = vec![0; frames * FRAME_BYTES];
        let mut file = self
            .file
            .lock()
            .map_err(|_| invalid("audio cache reader lock poisoned"))?;
        file.seek(SeekFrom::Start((start * FRAME_BYTES) as u64))?;
        file.read_exact(&mut bytes)?;
        if sha256(&bytes) != *expected {
            return Err(invalid("audio chunk checksum mismatch"));
        }
        Ok(bytes)
    }
    /// Resident metadata only. PCM is file-backed; decoder/mixer buffers are
    /// accounted for by their owners, outside the asset-store resident budget.
    pub fn resident_bytes(&self) -> usize {
        self.metadata
            .chunk_sha256
            .iter()
            .map(String::len)
            .sum::<usize>()
            + self.metadata.dependency.path.len()
            + 256
    }
    /// Explicit bounded materialization for short static sound effects.
    pub fn read_all(&self, max_frames: usize) -> Result<Vec<[f32; 2]>> {
        if self.metadata.frames > max_frames as u64 {
            return Err(AssetError::Limit("static audio frames"));
        }
        let mut samples = Vec::with_capacity(self.metadata.frames as usize);
        for index in 0..self.metadata.chunk_sha256.len() {
            samples.extend(self.read_chunk(index)?);
        }
        Ok(samples)
    }
}
pub(crate) fn valid_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn path(cache: &Path, key: &str, extension: &str) -> Result<PathBuf> {
    if !valid_key(key) {
        return Err(invalid("invalid audio cache key"));
    }
    Ok(cache.join(format!("{key}.{extension}")))
}
pub(crate) fn key(dependency: &Dependency) -> Result<String> {
    Ok(sha256(&serde_json::to_vec(&(COOK_VERSION, dependency))?))
}
pub fn load_audio(cache: &Path, fingerprint: &str) -> Result<CookedAudio> {
    let metadata_path = path(cache, fingerprint, "json")?;
    let stat = fs::symlink_metadata(&metadata_path)?;
    if !stat.is_file() || stat.len() > 512 * 1024 {
        return Err(invalid("invalid audio manifest"));
    }
    let mut text = Vec::new();
    File::open(metadata_path)?
        .take(512 * 1024 + 1)
        .read_to_end(&mut text)?;
    if text.len() > 512 * 1024 {
        return Err(AssetError::Limit("audio manifest bytes"));
    }
    let metadata: AudioMetadata = serde_json::from_slice(&text)?;
    if metadata.version != 1
        || metadata.fingerprint != fingerprint
        || !valid_key(&metadata.dependency.sha256)
        || key(&metadata.dependency)? != fingerprint
        || !(8000..=192000).contains(&metadata.sample_rate)
        || !matches!(metadata.channels, 1 | 2)
        || metadata.frames == 0
        || metadata.frames > (MAX_AUDIO_PCM_BYTES / FRAME_BYTES) as u64
        || metadata.chunk_sha256.len() != (metadata.frames as usize).div_ceil(AUDIO_CHUNK_FRAMES)
        || metadata.chunk_sha256.iter().any(|hash| !valid_key(hash))
    {
        return Err(invalid(
            "audio manifest version, dimensions or key mismatch",
        ));
    }
    let pcm_path = path(cache, &metadata.content_sha256, "ipcm")?;
    let stat = fs::symlink_metadata(&pcm_path)?;
    if !stat.is_file() || stat.len() != metadata.frames * FRAME_BYTES as u64 {
        return Err(invalid("invalid audio PCM file length/type"));
    }
    let audio = CookedAudio {
        metadata,
        cache_hit: true,
        file: Mutex::new(File::open(pcm_path)?),
    };
    // Validate all chunks before publishing a replacement, without retaining PCM.
    // Per-read checks additionally detect in-place external corruption afterwards.
    use sha2::{Digest, Sha256};
    let mut content = Sha256::new();
    for index in 0..audio.metadata.chunk_sha256.len() {
        let bytes = audio.read_bytes(index)?;
        if bytes
            .as_chunks::<4>()
            .0
            .iter()
            .any(|sample| !f32::from_le_bytes(*sample).is_finite())
        {
            return Err(invalid("audio samples must be finite"));
        }
        content.update(bytes);
    }
    if format!("{:x}", content.finalize()) != audio.metadata.content_sha256 {
        return Err(invalid("audio PCM checksum mismatch"));
    }
    Ok(audio)
}
