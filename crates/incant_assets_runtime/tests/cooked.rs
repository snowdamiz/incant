mod support;
use incant_assets_runtime::{AUDIO_CHUNK_FRAMES, load_audio};
use std::fs;

#[test]
fn cooked_only_reader_seeks_with_bounded_reads_and_no_source() {
    let temp = tempfile::tempdir().unwrap();
    let frames: Vec<_> = (0..AUDIO_CHUNK_FRAMES + 17)
        .map(|i| [i as f32 / 32768., -0.25])
        .collect();
    let key = support::cooked(temp.path(), &frames);
    let audio = load_audio(temp.path(), &key).unwrap();
    assert!(!temp.path().join("missing/source.wav").exists());
    assert!(audio.resident_bytes() < 1024);
    assert_eq!(audio.read_chunk(1).unwrap(), frames[AUDIO_CHUNK_FRAMES..]);
    assert_eq!(audio.read_chunk(0).unwrap(), frames[..AUDIO_CHUNK_FRAMES]);
    assert_eq!(audio.read_all(frames.len()).unwrap(), frames);
    assert!(audio.read_all(frames.len() - 1).is_err());
    assert!(audio.read_chunk(2).is_err());
    assert!(audio.read_chunk(usize::MAX).is_err());
    assert!(load_audio(temp.path(), "../invalid").is_err());
}

#[test]
fn retained_reader_detects_later_corruption_without_publishing_samples() {
    use std::io::{Seek, SeekFrom, Write};
    let temp = tempfile::tempdir().unwrap();
    let frames = vec![[0.25, -0.25]; AUDIO_CHUNK_FRAMES + 17];
    let key = support::cooked(temp.path(), &frames);
    let audio = load_audio(temp.path(), &key).unwrap();
    let path = temp
        .path()
        .join(format!("{}.ipcm", audio.metadata.content_sha256));
    let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.seek(SeekFrom::Start(AUDIO_CHUNK_FRAMES as u64 * 8))
        .unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    assert_eq!(audio.read_chunk(0).unwrap(), frames[..AUDIO_CHUNK_FRAMES]);
    assert!(audio.read_chunk(1).is_err());
    assert!(audio.read_all(frames.len()).is_err());
    assert!(load_audio(temp.path(), &key).is_err());
}

#[test]
fn untrusted_manifest_and_pcm_are_checked_before_a_handle_is_returned() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.25, -0.25]; 17]);
    let manifest = temp.path().join(format!("{key}.json"));
    let good: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    for (field, value) in [
        ("frames", serde_json::json!(u64::MAX)),
        ("sample_rate", serde_json::json!(0)),
        ("chunk_sha256", serde_json::json!([])),
        ("content_sha256", serde_json::json!("../../escape")),
        ("version", serde_json::json!(2)),
    ] {
        let mut bad = good.clone();
        bad[field] = value;
        fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(load_audio(temp.path(), &key).is_err(), "{field}");
    }
    fs::write(&manifest, serde_json::to_vec(&good).unwrap()).unwrap();
    let pcm = temp
        .path()
        .join(format!("{}.ipcm", good["content_sha256"].as_str().unwrap()));
    fs::write(pcm, [0u8; 7]).unwrap();
    assert!(load_audio(temp.path(), &key).is_err());
    // Even a self-consistent content/hash manifest cannot publish nonfinite PCM.
    let key = support::cooked(temp.path(), &[[f32::INFINITY, 0.]]);
    assert!(load_audio(temp.path(), &key).is_err());
}
