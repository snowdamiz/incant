#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
use incant_assets_runtime::load_audio;
use incant_audio_runtime::{AudioError, Bus, Mixer, PlayOptions};
use std::sync::Arc;

#[test]
fn cooked_only_mixer_is_batch_invariant_across_seek_loop_and_resampling() {
    let temp = tempfile::tempdir().unwrap();
    let frames: Vec<_> = (0..20000)
        .map(|i| {
            let v = (i as f32 * 0.06).sin() * 0.3;
            [v, -v]
        })
        .collect();
    let key = support::cooked(temp.path(), &frames);
    let clip = Arc::new(load_audio(temp.path(), &key).unwrap());
    for streaming in [false, true] {
        let options = PlayOptions {
            streaming,
            looping: true,
            rate: 1.7,
            start_seconds: 0.4,
            ..Default::default()
        };
        let mut whole = Mixer::offline(44100, &[]).unwrap();
        let mut split = Mixer::offline(44100, &[]).unwrap();
        whole.play(Arc::clone(&clip), options.clone()).unwrap();
        split.play(Arc::clone(&clip), options).unwrap();
        let mut a = vec![0.; 88000];
        let mut b = vec![0.; a.len()];
        whole.render(&mut a).unwrap();
        for block in b.chunks_mut(194) {
            split.render(block).unwrap();
        }
        assert_eq!(a, b, "streaming={streaming}");
        assert!(a.iter().any(|v| v.abs() > 0.2));
        assert!(
            a.as_chunks::<2>()
                .0
                .iter()
                .all(|f| (f[0] + f[1]).abs() < 1e-7)
        );
        assert!(whole.stream_errors().is_empty());
    }
}

#[test]
fn invalid_buses_and_playback_fail_without_interrupting_existing_mix() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.25; 2]; 10000]);
    let clip = Arc::new(load_audio(temp.path(), &key).unwrap());
    let cycle = [
        Bus {
            id: "a".into(),
            parent: Some("b".into()),
            gain_db: 0.,
        },
        Bus {
            id: "b".into(),
            parent: Some("a".into()),
            gain_db: 0.,
        },
    ];
    assert!(Mixer::offline(48000, &cycle).is_err());
    let mut mixer = Mixer::offline(48000, &[]).unwrap();
    mixer
        .play(Arc::clone(&clip), PlayOptions::default())
        .unwrap();
    assert!(matches!(
        mixer.play(
            Arc::clone(&clip),
            PlayOptions {
                rate: f64::NAN,
                ..Default::default()
            }
        ),
        Err(AudioError::Invalid(_))
    ));
    assert!(matches!(
        mixer.play(
            clip,
            PlayOptions {
                bus: Some("missing".into()),
                ..Default::default()
            }
        ),
        Err(AudioError::UnknownBus)
    ));
    let mut output = vec![0.; 1024];
    mixer.render(&mut output).unwrap();
    assert!(output[128..].iter().all(|v| *v > 0.2));
}
