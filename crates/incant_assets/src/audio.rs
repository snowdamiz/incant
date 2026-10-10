//! Compatibility boundary for the authoring importer; the reader is runtime-only.
pub use incant_assets_runtime::audio::{
    AUDIO_CHUNK_FRAMES, AudioMetadata, CookedAudio, FRAME_BYTES, MAX_AUDIO_PCM_BYTES, key, path,
};
pub fn load_audio(cache: &std::path::Path, fingerprint: &str) -> crate::Result<CookedAudio> {
    incant_assets_runtime::load_audio(cache, fingerprint).map_err(Into::into)
}
