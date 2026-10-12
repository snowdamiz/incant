use super::*;
mod support {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../incant_assets_runtime/tests/support/mod.rs"
    ));
}
fn clip(root: &std::path::Path, rate: u32, phase: f32, count: usize) -> Arc<CookedAudio> {
    let frames: Vec<_> = (0..count)
        .map(|i| {
            [
                (i as f32 * 0.13 + phase).sin() * 0.3,
                (i as f32 * 0.21 + phase).cos() * 0.2,
            ]
        })
        .collect();
    let key = support::cooked(root, &frames);
    let path = root.join(format!("{key}.json"));
    let mut metadata: incant_assets_runtime::AudioMetadata =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    metadata.sample_rate = rate;
    std::fs::write(path, serde_json::to_vec(&metadata).unwrap()).unwrap();
    Arc::new(incant_assets_runtime::load_audio(root, &key).unwrap())
}
fn render(mixer: &mut Mixer<crate::OfflineBackend>, n: usize) -> Vec<f32> {
    let mut out = vec![0.; n * 2];
    mixer.render(&mut out).unwrap();
    assert!(mixer.stream_errors().is_empty());
    out
}
#[test]
fn fresh_mixer_preserves_fractional_static_streamed_order_buses_and_spatial_history() {
    let temp = tempfile::tempdir().unwrap();
    let a = clip(&temp.path().join("a"), 22050, 0.1, 2003);
    let b = clip(&temp.path().join("b"), 32000, 0.7, 1703);
    for output_rate in [32000, 44100, 48000] {
        let buses = [
            Bus {
                id: "child".into(),
                parent: Some("parent".into()),
                gain_db: -3.,
            },
            Bus {
                id: "parent".into(),
                parent: None,
                gain_db: -6.,
            },
        ];
        let mut live = Mixer::offline(output_rate, &buses).unwrap();
        let mut ids = Vec::new();
        for i in 0..8 {
            ids.push(
                live.play(
                    Arc::clone(if i % 2 == 0 { &a } else { &b }),
                    PlayOptions {
                        bus: if i % 3 == 0 {
                            None
                        } else {
                            Some("child".into())
                        },
                        streaming: i % 2 == 1,
                        looping: true,
                        rate: 0.87 + i as f64 * 0.11,
                        start_seconds: 0.00031,
                        gain_db: -9.,
                        spatial: (i % 3 == 0).then_some(crate::Spatial {
                            position: [2., 0.5, -1.],
                            min_distance: 0.1,
                            max_distance: 10.,
                        }),
                        ..Default::default()
                    },
                )
                .unwrap(),
            );
        }
        live.set_listener([0.4, 0.2, 0.1], [0., 0., 0., 1.])
            .unwrap();
        assert!(render(&mut live, 187).iter().any(|v| v.abs() > 0.001));
        live.set_bus_gain(Some("child"), -2.).unwrap();
        live.set_bus_gain(None, -1.).unwrap();
        live.set_listener([0.7, -0.1, 0.2], [0., 0.2, 0., (0.96f32).sqrt()])
            .unwrap();
        live.set_position(ids[0], [1.3, 0.7, -0.5]).unwrap();
        live.configure_voice(ids[2], -4., 0.3, 1.7).unwrap();
        live.set_paused(ids[1], true).unwrap();
        live.stop(ids[4]).unwrap();
        assert!(live.checkpoint().is_err());
        render(&mut live, 59);
        let state = live.checkpoint().unwrap();
        let mut restored = Mixer::restore(&state).unwrap();
        for block in [1, 3, 11, 127, 33, 2049] {
            assert_eq!(
                render(&mut live, block),
                render(&mut restored, block),
                "rate={output_rate} block={block}"
            );
        }
        for m in [&mut live, &mut restored] {
            m.set_paused(ids[1], false).unwrap();
            m.stop(ids[0]).unwrap();
            m.play(
                Arc::clone(&b),
                PlayOptions {
                    looping: true,
                    streaming: true,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        for block in [29, 401, 67] {
            assert_eq!(render(&mut live, block), render(&mut restored, block));
        }
        assert_eq!(
            live.checkpoint().unwrap().rendered_frames,
            restored.checkpoint().unwrap().rendered_frames
        );
    }
}
#[test]
fn stopped_retained_and_removed_slots_remain_distinct_and_bad_candidate_is_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let a = clip(temp.path(), 48000, 0.3, 7);
    let mut live = Mixer::offline(48000, &[]).unwrap();
    let id = live.play(Arc::clone(&a), PlayOptions::default()).unwrap();
    render(&mut live, 40);
    let held = live.checkpoint().unwrap();
    assert!(held.voices[0].retained);
    let mut resumed = Mixer::restore(&held).unwrap();
    assert_eq!(live.master.num_sounds(), resumed.master.num_sounds());
    assert_eq!(render(&mut live, 19), render(&mut resumed, 19));
    let removed = live.checkpoint().unwrap();
    assert!(!removed.voices[0].retained);
    let mut resumed = Mixer::restore(&removed).unwrap();
    assert_eq!(live.master.num_sounds(), resumed.master.num_sounds());
    assert_eq!(resumed.state(id).unwrap(), PlaybackState::Stopped);
    for bad in [0, id] {
        let mut state = removed.clone();
        state.next = bad;
        assert!(Mixer::restore(&state).is_err());
    }
    let mut state = removed.clone();
    state.listener.0[0] = f32::NAN;
    assert!(Mixer::restore(&state).is_err());
    let mut state = removed.clone();
    state.voices.push(state.voices[0].clone());
    assert!(Mixer::restore(&state).is_err());
    assert_eq!(render(&mut live, 31), render(&mut resumed, 31));
}

#[test]
fn processed_master_pause_keeps_real_fade_and_child_cursors_on_resume() {
    let temp = tempfile::tempdir().unwrap();
    let clip = clip(temp.path(), 22050, 0.3, 2309);
    let mut live = Mixer::offline(44100, &[]).unwrap();
    for streaming in [false, true] {
        live.play(
            Arc::clone(&clip),
            PlayOptions {
                streaming,
                looping: true,
                rate: 1.13,
                ..Default::default()
            },
        )
        .unwrap();
    }
    render(&mut live, 103);
    live.set_suspended(true);
    assert!(render(&mut live, 193).iter().all(|v| *v == 0.));
    let saved = live.checkpoint().unwrap();
    assert!(saved.master_paused);
    let mut restored = Mixer::restore(&saved).unwrap();
    assert_eq!(render(&mut live, 71), render(&mut restored, 71));
    live.set_suspended(false);
    restored.set_suspended(false);
    // Compare even the first internal interpolation chunk, not just the tail.
    assert_eq!(render(&mut live, 731), render(&mut restored, 731));
}
