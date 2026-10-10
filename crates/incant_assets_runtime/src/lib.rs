//! Bounded cooked-asset readers. No project, importer, command bus or agent.
//! Audio manifests are decoded once at load; chunk reads use retained typed data.
pub mod audio;
pub use audio::{AUDIO_CHUNK_FRAMES, AudioMetadata, CookedAudio, MAX_AUDIO_PCM_BYTES, load_audio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Error)]
pub enum AssetError {
    #[error("asset IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid asset: {0}")]
    Invalid(String),
    #[error("asset exceeds {0} limit")]
    Limit(&'static str),
}
pub type Result<T> = std::result::Result<T, AssetError>;
fn invalid(reason: impl Into<String>) -> AssetError {
    AssetError::Invalid(reason.into())
}
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
