#[path = "../../incant_assets/tests/support/audio.rs"]
mod support;
use incant_assets::cook_audio;
use incant_audio::{AudioError, Bus, Mixer, OfflineBackend, PlayOptions, PlaybackState, Spatial};
use std::{fs, path::Path, sync::Arc};
fn clip(root: &Path, frames: usize) -> Arc<incant_assets::CookedAudio> {
    fs::write(
        root.join("tone.wav"),
        support::wav(1, 48000, &vec![[8000, 8000]; frames]),
    )
    .unwrap();
    Arc::new(cook_audio(root, Path::new("tone.wav"), &root.join("cache")).unwrap())
}
fn render(mixer: &mut Mixer<OfflineBackend>, count: usize) -> Vec<f32> {
    let mut output = vec![0.; count * 2];
    mixer.render(&mut output).unwrap();
    assert!(output.iter().all(|v| v.is_finite()));
    output
}
fn rms(samples: &[f32], channel: usize) -> f64 {
    (samples
        .iter()
        .skip(channel)
        .step_by(2)
        .map(|s| f64::from(*s).powi(2))
        .sum::<f64>()
        / (samples.len() / 2) as f64)
        .sqrt()
}
#[test]
fn bus_gain_pause_resume_and_stop_change_real_samples() {
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 48000);
    let mut mixer = Mixer::offline(
        48000,
        &[
            Bus {
                id: "sfx".into(),
                parent: Some("master".into()),
                gain_db: 0.,
            },
            Bus {
                id: "master".into(),
                parent: None,
                gain_db: 0.,
            },
        ],
    )
    .unwrap();
    let voice = mixer
        .play(
            clip,
            PlayOptions {
                bus: Some("sfx".into()),
                looping: true,
                ..Default::default()
            },
        )
        .unwrap();
    render(&mut mixer, 256);
    let full = rms(&render(&mut mixer, 512), 0);
    assert!(full > 0.1);
    mixer.set_bus_gain(Some("master"), -6.).unwrap();
    render(&mut mixer, 256);
    let half = rms(&render(&mut mixer, 512), 0);
    assert!((half / full - 10_f64.powf(-6. / 20.)).abs() < 1e-5);
    mixer.set_paused(voice, true).unwrap();
    render(&mut mixer, 256);
    assert!(rms(&render(&mut mixer, 512), 0) < 1e-10);
    assert_eq!(mixer.state(voice).unwrap(), PlaybackState::Paused);
    mixer.set_paused(voice, false).unwrap();
    render(&mut mixer, 256);
    assert!((rms(&render(&mut mixer, 512), 0) - half).abs() < 1e-5);
    mixer.stop(voice).unwrap();
    render(&mut mixer, 256);
    assert!(rms(&render(&mut mixer, 512), 0) < 1e-10);
    assert_eq!(mixer.state(voice).unwrap(), PlaybackState::Stopped);
}
#[test]
fn spatial_position_distance_and_listener_rotation_change_stereo_output() {
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 48000);
    let mut mixer = Mixer::offline(48000, &[]).unwrap();
    let voice = mixer
        .play(
            clip,
            PlayOptions {
                looping: true,
                spatial: Some(Spatial {
                    position: [5., 0., 0.],
                    min_distance: 1.,
                    max_distance: 10.,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    render(&mut mixer, 256);
    let right = render(&mut mixer, 512);
    assert!(
        rms(&right, 1) > rms(&right, 0) * 5.,
        "R={} L={}",
        rms(&right, 1),
        rms(&right, 0)
    );
    mixer.set_position(voice, [-5., 0., 0.]).unwrap();
    render(&mut mixer, 256);
    let left = render(&mut mixer, 512);
    assert!(rms(&left, 0) > rms(&left, 1) * 5.);
    mixer.set_listener([0., 0., 0.], [0., 1., 0., 0.]).unwrap();
    render(&mut mixer, 256);
    let rotated = render(&mut mixer, 512);
    assert!(rms(&rotated, 1) > rms(&rotated, 0) * 5.);
    mixer.set_position(voice, [-11., 0., 0.]).unwrap();
    render(&mut mixer, 256);
    let far = render(&mut mixer, 512);
    assert!(rms(&far, 0) + rms(&far, 1) < 1e-7);
}
#[test]
fn invalid_configuration_and_bad_commands_leave_existing_sound_intact() {
    assert!(Mixer::offline(0, &[]).is_err());
    for definitions in [
        vec![Bus {
            id: "a".into(),
            parent: Some("a".into()),
            gain_db: 0.,
        }],
        vec![Bus {
            id: "a".into(),
            parent: Some("absent".into()),
            gain_db: 0.,
        }],
        vec![Bus {
            id: "a".into(),
            parent: None,
            gain_db: f32::NAN,
        }],
    ] {
        assert!(Mixer::offline(48000, &definitions).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 48000);
    let mut mixer = Mixer::offline(48000, &[]).unwrap();
    let voice = mixer
        .play(Arc::clone(&clip), PlayOptions::default())
        .unwrap();
    render(&mut mixer, 256);
    let before = render(&mut mixer, 256);
    for options in [
        PlayOptions {
            gain_db: f32::NAN,
            ..Default::default()
        },
        PlayOptions {
            bus: Some("absent".into()),
            ..Default::default()
        },
        PlayOptions {
            start_seconds: 2.,
            ..Default::default()
        },
    ] {
        assert!(mixer.play(Arc::clone(&clip), options).is_err());
    }
    assert!(matches!(mixer.stop(999), Err(AudioError::UnknownVoice)));
    assert!(mixer.set_bus_gain(None, f32::NAN).is_err());
    assert!(mixer.set_listener([0.; 3], [0.; 4]).is_err());
    assert_eq!(mixer.state(voice).unwrap(), PlaybackState::Playing);
    assert_eq!(render(&mut mixer, 256), before);
}
#[test]
fn voice_limit_rejects_new_playback_without_disturbing_the_existing_mix() {
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 48000);
    let mut mixer = Mixer::offline(48000, &[]).unwrap();
    for _ in 0..incant_audio::MAX_VOICES {
        mixer
            .play(
                Arc::clone(&clip),
                PlayOptions {
                    looping: true,
                    gain_db: -40.,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    render(&mut mixer, 256);
    let before = render(&mut mixer, 256);
    assert!((rms(&before, 0) - (8000. / 32768.) * 128. * 0.01).abs() < 1e-6);
    assert!(matches!(
        mixer.play(clip, PlayOptions::default()),
        Err(AudioError::Capacity)
    ));
    assert_eq!(render(&mut mixer, 256), before);
}
#[test]
fn streaming_decoder_seeks_exactly_and_keeps_independent_cursors() {
    use kira::sound::streaming::Decoder;
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 20000);
    let mut a = incant_audio::ClipDecoder::new(Arc::clone(&clip));
    let mut b = incant_audio::ClipDecoder::new(clip);
    assert_eq!(a.seek(9001).unwrap(), 9001);
    assert_eq!(a.decode().unwrap().len(), 16384 - 9001);
    assert_eq!(b.decode().unwrap().len(), 8192);
    assert_eq!(a.decode().unwrap().len(), 20000 - 16384);
    assert!(a.decode().unwrap().is_empty());
    assert_eq!(a.seek(usize::MAX).unwrap(), 20000);
    assert_eq!(b.seek(19999).unwrap(), 19999);
    assert_eq!(b.decode().unwrap().len(), 1);
}
#[test]
fn streaming_voice_outputs_real_samples_and_reports_cache_corruption() {
    use std::{
        io::{Seek, SeekFrom, Write},
        time::Duration,
    };
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 48000);
    let mut mixer = Mixer::<OfflineBackend>::new(
        incant_audio::AudioManagerSettings {
            backend_settings: 48000,
            ..Default::default()
        },
        &[],
    )
    .unwrap();
    let path = temp
        .path()
        .join("cache")
        .join(format!("{}.ipcm", clip.metadata.content_sha256));
    let voice = mixer
        .play(
            clip,
            PlayOptions {
                streaming: true,
                looping: true,
                ..Default::default()
            },
        )
        .unwrap();
    let mut heard = false;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(2));
        let out = render(&mut mixer, 256);
        heard |= rms(&out, 0) > 0.1;
        if heard {
            break;
        }
    }
    assert!(heard);
    assert!(mixer.stream_errors().is_empty());
    let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.seek(SeekFrom::Start(32000 * 8)).unwrap();
    file.write_all(&[1, 2, 3, 4]).unwrap();
    let mut error = false;
    for _ in 0..500 {
        std::thread::sleep(Duration::from_millis(2));
        render(&mut mixer, 256);
        if !mixer.stream_errors().is_empty() {
            error = true;
            break;
        }
    }
    assert!(error, "streaming corruption must reach the host");
    render(&mut mixer, 256);
    assert_eq!(mixer.state(voice).unwrap(), PlaybackState::Stopped);
}
