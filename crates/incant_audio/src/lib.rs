//! Kira-backed engine mixing. Offline output uses the real mixer and sample
//! renderer without creating an OS audio device; device output is explicit.
mod config;
mod mixer;
mod offline;
mod offline_stream;
mod scene;
pub use scene::SceneAudio;
#[cfg(not(target_arch = "wasm32"))]
mod stream;
pub use config::{Bus, PlayOptions, Spatial};
#[cfg(feature = "device")]
pub use kira::backend::cpal::CpalBackend as DeviceBackend;
pub use kira::{AudioManagerSettings, sound::PlaybackState};
pub use mixer::{MAX_BUSES, MAX_VOICES, Mixer, VoiceId};
pub use offline::OfflineBackend;
#[cfg(not(target_arch = "wasm32"))]
pub use stream::ClipDecoder;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("invalid audio setting: {0}")]
    Invalid(&'static str),
    #[error("unsupported audio capability: {0}")]
    Unsupported(&'static str),
    #[error("audio resource capacity exceeded")]
    Capacity,
    #[error("unknown audio bus")]
    UnknownBus,
    #[error("unknown audio voice")]
    UnknownVoice,
    #[error("audio backend: {0}")]
    Backend(String),
    #[error(transparent)]
    Asset(#[from] incant_assets::AssetError),
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("audio cache for asset {0} is absent or does not match the document")]
    MissingAsset(String),
    #[error("audio decoding failed: {0}")]
    Stream(String),
    #[error("audio session failed; create a new session")]
    FailedSession,
}
pub type Result<T> = std::result::Result<T, AudioError>;
