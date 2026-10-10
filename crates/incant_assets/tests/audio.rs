#[path = "support/audio.rs"]
mod support;
use incant_assets::{AUDIO_CHUNK_FRAMES, cook_audio, load_audio};
use std::{fs, path::Path};

#[test]
fn wav_channels_chunks_seek_and_source_free_cache_are_exact() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    let frames: Vec<_> = (0..AUDIO_CHUNK_FRAMES + 17)
        .map(|i| [(i % 16000) as i16, -1234])
        .collect();
    fs::write(root.join("stereo.wav"), support::wav(2, 44100, &frames)).unwrap();
    let cooked = cook_audio(root, Path::new("stereo.wav"), &cache).unwrap();
    assert!(!cooked.cache_hit);
    assert_eq!(cooked.metadata.frames, frames.len() as u64);
    assert_eq!(cooked.metadata.channels, 2);
    assert_eq!(cooked.metadata.sample_rate, 44100);
    assert_eq!(cooked.metadata.chunk_sha256.len(), 2);
    assert!(cooked.resident_bytes() < 1024);
    let last = cooked.read_chunk(1).unwrap();
    assert_eq!(last.len(), 17);
    for (actual, expected) in last.iter().zip(&frames[AUDIO_CHUNK_FRAMES..]) {
        assert_eq!(
            *actual,
            [
                f32::from(expected[0]) / 32768.,
                f32::from(expected[1]) / 32768.
            ]
        );
    }
    assert!(cooked.read_all(frames.len() - 1).is_err());
    assert!(cooked.read_chunk(2).is_err());
    let cached = cook_audio(root, Path::new("stereo.wav"), &cache).unwrap();
    assert!(cached.cache_hit);
    fs::remove_file(root.join("stereo.wav")).unwrap();
    let loaded = load_audio(&cache, &cooked.metadata.fingerprint).unwrap();
    assert_eq!(
        loaded.read_all(frames.len()).unwrap(),
        cooked.read_all(frames.len()).unwrap()
    );
}

#[test]
fn vorbis_decodes_real_compressed_audio_with_gapless_duration() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("tone.ogg"),
        include_bytes!("fixtures/tone-440hz.ogg"),
    )
    .unwrap();
    let audio = cook_audio(root, Path::new("tone.ogg"), &root.join("cache")).unwrap();
    assert_eq!(audio.metadata.channels, 2);
    assert_eq!(audio.metadata.sample_rate, 48000);
    assert_eq!(audio.metadata.frames, 24000);
    let frames = audio.read_all(24000).unwrap();
    assert!(frames.iter().all(|f| (f[0] - f[1]).abs() < 1e-5));
    let rms = (frames.iter().map(|f| f64::from(f[0]).powi(2)).sum::<f64>() / 24000.).sqrt();
    assert!((0.24..=0.28).contains(&rms), "{rms}");
    // Compare the actual decoded signal, allowing lossy-codec error.
    let error = (frames
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let expected =
                12000. / 32768. * (2. * std::f64::consts::PI * 440. * i as f64 / 48000.).sin();
            (f64::from(f[0]) - expected).powi(2)
        })
        .sum::<f64>()
        / 24000.)
        .sqrt();
    assert!(error < 0.015, "signal RMSE {error}");
}

#[test]
fn invalid_sources_and_corrupted_cache_fail_without_publishing_valid_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    for (name, bytes) in [
        ("bad.wav", b"not audio".to_vec()),
        ("wrong.ogg", support::wav(1, 48000, &[[12, 12]; 50])),
        ("empty.wav", support::wav(1, 48000, &[])),
        ("rate.wav", support::wav(1, 1000, &[[12, 12]; 50])),
        ("truncated.wav", {
            let mut b = support::wav(1, 48000, &[[12, 12]; 50]);
            b.truncate(50);
            b
        }),
    ] {
        fs::write(root.join(name), bytes).unwrap();
        assert!(cook_audio(root, Path::new(name), &cache).is_err(), "{name}");
    }
    fs::write(
        root.join("tone.wav"),
        support::wav(1, 48000, &[[1200, 1200]; 100]),
    )
    .unwrap();
    let audio = cook_audio(root, Path::new("tone.wav"), &cache).unwrap();
    let path = cache.join(format!("{}.ipcm", audio.metadata.content_sha256));
    let mut data = fs::read(&path).unwrap();
    data[0] ^= 1;
    fs::write(&path, data).unwrap();
    assert!(
        audio.read_chunk(0).is_err(),
        "retained readers detect in-place corruption"
    );
    assert!(load_audio(&cache, &audio.metadata.fingerprint).is_err());
    let repaired = cook_audio(root, Path::new("tone.wav"), &cache).unwrap();
    assert!(!repaired.cache_hit);
    assert_eq!(repaired.read_chunk(0).unwrap()[0], [1200. / 32768.; 2]);
    assert!(
        audio.read_chunk(0).is_err(),
        "old handle still sees corrupt bytes"
    );
    // A different source identity can produce identical cooked PCM. Publishing
    // it must work with both the repaired and the corrupt reader still alive.
    fs::copy(root.join("tone.wav"), root.join("copy.wav")).unwrap();
    let copy = cook_audio(root, Path::new("copy.wav"), &cache).unwrap();
    assert_eq!(
        copy.metadata.content_sha256,
        repaired.metadata.content_sha256
    );
    assert_ne!(copy.metadata.fingerprint, repaired.metadata.fingerprint);
    assert_eq!(copy.read_chunk(0).unwrap(), repaired.read_chunk(0).unwrap());
    assert!(audio.read_chunk(0).is_err());
    assert!(load_audio(&cache, "../../escape").is_err());
}
