#[path = "../../incant_assets/tests/support/audio.rs"]
mod support;
use incant_assets::cook_audio;
use incant_audio::{Mixer, PlayOptions, PlaybackState};
use std::{fs, path::Path, sync::Arc};

#[test]
fn fast_streaming_export_is_repeatable_across_chunks_rates_and_loop_seams() {
    let temp = tempfile::tempdir().unwrap();
    let samples: Vec<_> = (0..20000)
        .map(|i| {
            let sample = ((i as f64 * 0.06).sin() * 10000.) as i16;
            [sample, -sample]
        })
        .collect();
    fs::write(
        temp.path().join("tone.wav"),
        support::wav(2, 48000, &samples),
    )
    .unwrap();
    let clip = Arc::new(
        cook_audio(
            temp.path(),
            Path::new("tone.wav"),
            &temp.path().join("cache"),
        )
        .unwrap(),
    );
    for rate in [0.25, 1., 1.7, 4.] {
        let options = PlayOptions {
            streaming: true,
            looping: true,
            rate,
            start_seconds: 0.4,
            ..Default::default()
        };
        let mut a = Mixer::offline(44100, &[]).unwrap();
        let mut b = Mixer::offline(44100, &[]).unwrap();
        a.play(Arc::clone(&clip), options.clone()).unwrap();
        b.play(Arc::clone(&clip), options).unwrap();
        let mut whole = vec![0.; 88000];
        let mut split = vec![0.; whole.len()];
        a.render(&mut whole).unwrap();
        for block in split.chunks_mut(194) {
            b.render(block).unwrap();
        }
        assert_eq!(whole, split, "backend batch size must not change audio");
        assert!(whole.iter().any(|v| v.abs() > 0.2));
        // Stereo remains anti-phase and no decoder starvation inserts zero blocks.
        assert!(
            whole
                .as_chunks::<2>()
                .0
                .iter()
                .all(|f| (f[0] + f[1]).abs() < 1e-7)
        );
        assert!(whole.chunks(256).all(|b| b.iter().any(|v| v.abs() > 0.001)));
        assert!(a.stream_errors().is_empty());
    }
}

#[test]
fn offline_stream_pause_end_and_errors_reach_the_host_without_waits() {
    use std::io::{Seek, SeekFrom, Write};
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("tone.wav"),
        support::wav(1, 48000, &vec![[12000; 2]; 20000]),
    )
    .unwrap();
    let clip = Arc::new(
        cook_audio(
            temp.path(),
            Path::new("tone.wav"),
            &temp.path().join("cache"),
        )
        .unwrap(),
    );
    let mut mixer = Mixer::offline(48000, &[]).unwrap();
    let voice = mixer
        .play(
            Arc::clone(&clip),
            PlayOptions {
                streaming: true,
                ..Default::default()
            },
        )
        .unwrap();
    let mut output = vec![0.; 1024];
    mixer.render(&mut output).unwrap();
    assert!(output[128..].iter().all(|v| *v > 0.3));
    mixer.set_paused(voice, true).unwrap();
    mixer.render(&mut output).unwrap();
    assert!(output.iter().all(|v| *v == 0.));
    mixer.set_paused(voice, false).unwrap();
    mixer.render(&mut output).unwrap();
    assert!(output.iter().all(|v| *v > 0.3));
    let path = temp
        .path()
        .join("cache")
        .join(format!("{}.ipcm", clip.metadata.content_sha256));
    let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.seek(SeekFrom::Start(17000 * 8)).unwrap();
    file.write_all(&[1, 2, 3, 4]).unwrap();
    mixer.render(&mut vec![0.; 40000]).unwrap();
    assert_eq!(mixer.state(voice).unwrap(), PlaybackState::Stopped);
    fs::write(
        temp.path().join("next.wav"),
        support::wav(1, 48000, &[[1000; 2]; 100]),
    )
    .unwrap();
    let next = Arc::new(
        cook_audio(
            temp.path(),
            Path::new("next.wav"),
            &temp.path().join("cache"),
        )
        .unwrap(),
    );
    mixer.play(next, PlayOptions::default()).unwrap();
    // Reaping a failed voice while starting another must not lose its error.
    let errors = mixer.stream_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, voice);
    assert!(mixer.stream_errors().is_empty());
}
