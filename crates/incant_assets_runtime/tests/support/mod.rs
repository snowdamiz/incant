use incant_assets_runtime::{AUDIO_CHUNK_FRAMES, AudioMetadata, Dependency, audio, sha256};
use std::{fs, path::Path};

// A format fixture, deliberately independent of all authoring/importer crates.
pub fn cooked(cache: &Path, frames: &[[f32; 2]]) -> String {
    fs::create_dir_all(cache).unwrap();
    let dependency = Dependency {
        path: "missing/source.wav".into(),
        sha256: sha256(b"fixture source identity"),
    };
    let fingerprint = audio::key(&dependency).unwrap();
    let pcm: Vec<_> = frames
        .iter()
        .flatten()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let metadata = AudioMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        dependency,
        sample_rate: 48000,
        channels: 2,
        frames: frames.len() as u64,
        content_sha256: sha256(&pcm),
        chunk_sha256: pcm.chunks(AUDIO_CHUNK_FRAMES * 8).map(sha256).collect(),
    };
    fs::write(cache.join(format!("{}.ipcm", metadata.content_sha256)), pcm).unwrap();
    fs::write(
        cache.join(format!("{fingerprint}.json")),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    fingerprint
}
