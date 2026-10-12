//! Independent cooked PCM fixture; no document/importer or audio guest code.
use incant_assets_runtime::{AUDIO_CHUNK_FRAMES, AudioMetadata, Dependency, audio, sha256};
use incant_audio_runtime::{ClipBinding, OfflineBackend, SceneAudio};
use incant_runtime::{NativeWorld, StableId};
use std::{fs, path::Path};
pub fn clip(cache: &Path, rate: u32, name: &str) -> String {
    fs::create_dir_all(cache).unwrap();
    let dependency = Dependency {
        path: format!("missing/{name}.wav"),
        sha256: sha256(name.as_bytes()),
    };
    let fingerprint = audio::key(&dependency).unwrap();
    let pcm: Vec<u8> = (0..2309)
        .flat_map(|i| {
            [
                (i as f32 * 0.13).sin() * 0.2,
                (i as f32 * 0.17).cos() * 0.18,
            ]
        })
        .flat_map(f32::to_le_bytes)
        .collect();
    let metadata = AudioMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        dependency,
        sample_rate: rate,
        channels: 2,
        frames: 2309,
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
pub fn host(world: &NativeWorld, path: &Path, keys: &[String; 2]) -> SceneAudio<OfflineBackend> {
    let clips = keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            ClipBinding::load(StableId((90 + i as u128).to_be_bytes()), path, key).unwrap()
        })
        .collect();
    SceneAudio::offline(world, clips, 48000).unwrap()
}
pub fn render(host: &mut SceneAudio<OfflineBackend>, world: &NativeWorld) -> Vec<f32> {
    host.sync(world).unwrap();
    let mut pcm = vec![0.; 1600];
    host.render(&mut pcm).unwrap();
    pcm
}
