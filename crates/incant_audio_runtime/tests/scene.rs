#![cfg(feature = "scene")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
use incant_audio_runtime::{AudioError, ClipBinding, Mixer, PlayOptions, SceneAudio, SourceState};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedScene, FrameCommands, NativeAudioBus, NativeAudioSource,
    NativeWorld, StableId, StructuralCommand,
};
use incant_types::{AudioSpatial, Transform, Velocity};
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn source(streaming: bool) -> CookedAudio {
    CookedAudio {
        entity: id(2),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            clip: id(90),
            bus: Some(id(1)),
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming,
            spatial: None,
        }),
    }
}
fn bus() -> CookedAudio {
    CookedAudio {
        entity: id(1),
        bus: Some(NativeAudioBus {
            parent: None,
            gain_db: 0.,
        }),
        source: None,
        listener: false,
    }
}
fn world(nodes: Vec<CookedAudio>) -> NativeWorld {
    let scene = CookedScene::new(
        id(99),
        60,
        (1..=5)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity { linear: [0.; 3] }),
            })
            .collect(),
    )
    .unwrap()
    .with_audio(nodes)
    .unwrap();
    NativeWorld::from_bytes(&scene.to_bytes()).unwrap()
}
fn edit(world: &mut NativeWorld, node: CookedAudio) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetAudio {
            entity: node.entity,
            audio: Some(node),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}
fn binding(temp: &tempfile::TempDir, key: &str) -> ClipBinding {
    ClipBinding::load(id(90), temp.path(), key).unwrap()
}
fn tone(temp: &tempfile::TempDir) -> String {
    support::cooked(
        temp.path(),
        &(0..20000)
            .map(|i| {
                let v = (i as f32 * 0.035).sin() * 0.25;
                [v; 2]
            })
            .collect::<Vec<_>>(),
    )
}
fn energy(pcm: &[f32]) -> f32 {
    pcm.iter().map(|v| v * v).sum::<f32>() / pcm.len() as f32
}
fn render(host: &mut SceneAudio<incant_audio_runtime::OfflineBackend>, frames: usize) -> Vec<f32> {
    let mut output = vec![0.; frames * 2];
    host.render(&mut output).unwrap();
    output
}

#[test]
fn binary_scene_static_and_streamed_playback_remain_batch_invariant() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    for streaming in [false, true] {
        let mut node = source(streaming);
        let source = node.source.as_mut().unwrap();
        source.rate = 1.7;
        source.start_seconds = 0.15;
        let world = world(vec![bus(), node]);
        let mut a = SceneAudio::offline(&world, vec![binding(&temp, &key)], 44100).unwrap();
        let mut b = SceneAudio::offline(&world, vec![binding(&temp, &key)], 44100).unwrap();
        let whole = render(&mut a, 32000);
        let mut split = vec![0.; whole.len()];
        for block in split.chunks_mut(194) {
            b.sync(&world).unwrap();
            b.render(block).unwrap();
        }
        assert_eq!(whole, split, "streaming={streaming}");
        assert!(energy(&whole) > 0.01);
    }
}

#[test]
fn pause_resume_preserves_cursor_and_typed_gain_pan_rate_changes_reach_mixer() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    for streaming in [false, true] {
        let mut node = source(streaming);
        let mut world = world(vec![bus(), node.clone()]);
        let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
        let mut reference = Mixer::offline(
            48000,
            &[incant_audio_runtime::Bus {
                id: "bus".into(),
                parent: None,
                gain_db: 0.,
            }],
        )
        .unwrap();
        let voice = reference
            .play(
                Arc::new(incant_assets_runtime::load_audio(temp.path(), &key).unwrap()),
                PlayOptions {
                    bus: Some("bus".into()),
                    streaming,
                    looping: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let reference_render = |mixer: &mut Mixer<_>, frames: usize| {
            let mut out = vec![0.; frames * 2];
            mixer.render(&mut out).unwrap();
            out
        };
        assert_eq!(
            render(&mut host, 1024),
            reference_render(&mut reference, 1024)
        );
        node.source.as_mut().unwrap().playing = false;
        edit(&mut world, node.clone());
        host.sync(&world).unwrap();
        reference.set_paused(voice, true).unwrap();
        let paused = render(&mut host, 2048);
        assert_eq!(paused, reference_render(&mut reference, 2048));
        assert!(energy(&paused[64..]) < 1e-10);
        assert_eq!(host.source_state(id(2)).unwrap(), SourceState::Paused);
        node.source.as_mut().unwrap().playing = true;
        edit(&mut world, node.clone());
        host.sync(&world).unwrap();
        reference.set_paused(voice, false).unwrap();
        let resumed = render(&mut host, 1024);
        let expected = reference_render(&mut reference, 1024);
        assert!(
            resumed == expected,
            "host must resume the existing voice and cursor: streaming={streaming}"
        );
        let s = node.source.as_mut().unwrap();
        s.gain_db = -12.;
        s.pan = 1.;
        s.rate = 2.;
        edit(&mut world, node);
        host.sync(&world).unwrap();
        reference.configure_voice(voice, -12., 1., 2.).unwrap();
        let changed = render(&mut host, 1024);
        assert!(
            changed == reference_render(&mut reference, 1024),
            "typed parameters must match direct mixer changes"
        );
        let frames = changed[512..].as_chunks::<2>().0;
        assert!(frames.iter().all(|f| f[0].abs() < 1e-7));
        assert!(frames.iter().any(|f| f[1].abs() > 0.02));
        assert!(energy(&changed) < energy(&expected) * 0.15);
    }
}

#[test]
fn finished_one_shots_do_not_restart_until_an_explicit_edge_or_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 128]);
    for streaming in [false, true] {
        let mut node = source(streaming);
        node.source.as_mut().unwrap().looping = false;
        let mut world = world(vec![bus(), node.clone()]);
        let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
        assert!(energy(&render(&mut host, 256)) > 0.001);
        for _ in 0..3 {
            host.sync(&world).unwrap();
            assert_eq!(render(&mut host, 256), vec![0.; 512]);
        }
        node.source.as_mut().unwrap().playing = false;
        edit(&mut world, node.clone());
        host.sync(&world).unwrap();
        node.source.as_mut().unwrap().playing = true;
        edit(&mut world, node.clone());
        host.sync(&world).unwrap();
        assert!(energy(&render(&mut host, 256)) > 0.001);
        node.source.as_mut().unwrap().looping = true;
        edit(&mut world, node);
        host.sync(&world).unwrap();
        assert!(energy(&render(&mut host, 256)) > 0.01);
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetAudio {
                entity: id(2),
                audio: None,
            })
            .unwrap();
        world.apply(&mut commands).unwrap();
        host.sync(&world).unwrap();
        assert!(matches!(
            host.source_state(id(2)),
            Err(AudioError::UnknownVoice)
        ));
        assert!(energy(&render(&mut host, 512)[64..]) < 1e-10);
        edit(&mut world, source(streaming));
        host.sync(&world).unwrap();
        assert!(energy(&render(&mut host, 256)) > 0.01);
    }
}

#[test]
fn whole_snapshot_preflight_rejects_late_bad_source_before_bus_or_voice_changes() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let mut world = world(vec![bus(), source(true)]);
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    let mut control = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    assert_eq!(render(&mut host, 1024), render(&mut control, 1024));
    let mut changed_bus = bus();
    changed_bus.bus.as_mut().unwrap().gain_db = -20.;
    edit(&mut world, changed_bus);
    let mut bad = source(true);
    bad.entity = id(4);
    bad.source.as_mut().unwrap().start_seconds = 1.;
    edit(&mut world, bad.clone());
    assert!(matches!(host.sync(&world), Err(AudioError::Invalid(_))));
    assert!(!host.is_failed());
    assert_eq!(
        render(&mut host, 1024),
        render(&mut control, 1024),
        "preflight issued mixer commands"
    );
    bad.source.as_mut().unwrap().start_seconds = 0.;
    edit(&mut world, bad);
    host.sync(&world).unwrap();
    assert!(energy(&render(&mut host, 1024)) < energy(&render(&mut control, 1024)) * 0.1);
}

#[test]
fn bus_topology_edits_reject_without_changing_existing_audio_and_can_be_retried() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let mut world = world(vec![bus(), source(false)]);
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    let mut added = bus();
    added.entity = id(5);
    edit(&mut world, added);
    assert!(matches!(host.sync(&world), Err(AudioError::Unsupported(_))));
    assert!(!host.is_failed());
    assert!(energy(&render(&mut host, 1024)) > 0.01);
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetAudio {
            entity: id(5),
            audio: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    host.sync(&world).unwrap();
    assert!(energy(&render(&mut host, 1024)) > 0.01);
}

struct RejectBackend;
impl kira::backend::Backend for RejectBackend {
    type Settings = Arc<AtomicUsize>;
    type Error = &'static str;
    fn setup(calls: Self::Settings, _: usize) -> Result<(Self, u32), Self::Error> {
        calls.fetch_add(1, Ordering::SeqCst);
        Err("expected backend rejection")
    }
    fn start(&mut self, _: kira::backend::Renderer) -> Result<(), Self::Error> {
        unreachable!()
    }
}
#[test]
fn constructor_checks_bindings_duration_and_pose_before_backend_setup() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let calls = Arc::new(AtomicUsize::new(0));
    let mut node = source(false);
    node.source.as_mut().unwrap().start_seconds = 1.;
    let invalid = world(vec![bus(), node]);
    let settings = || incant_audio_runtime::AudioManagerSettings::<RejectBackend> {
        backend_settings: calls.clone(),
        ..Default::default()
    };
    assert!(matches!(
        SceneAudio::new(&invalid, vec![binding(&temp, &key)], settings()),
        Err(AudioError::Invalid(_))
    ));
    let valid = world(vec![bus(), source(false)]);
    assert!(matches!(
        SceneAudio::new(&valid, vec![], settings()),
        Err(AudioError::Invalid(_))
    ));
    assert!(matches!(
        SceneAudio::new(
            &valid,
            vec![binding(&temp, &key), binding(&temp, &key)],
            settings()
        ),
        Err(AudioError::Invalid(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        SceneAudio::new(&valid, vec![binding(&temp, &key)], settings()),
        Err(AudioError::Backend(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(ClipBinding::load(id(90), temp.path(), &"0".repeat(64)).is_err());
    assert!(ClipBinding::load(id(90), temp.path(), "../escape").is_err());
}

#[test]
fn streamed_corruption_silences_output_and_permanently_fails_the_host() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let world = world(vec![bus(), source(true)]);
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    assert!(energy(&render(&mut host, 1024)) > 0.01);
    let pcm = fs::read_dir(temp.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.extension().unwrap() == "ipcm")
        .unwrap();
    {
        let mut file = fs::OpenOptions::new().write(true).open(pcm).unwrap();
        file.seek(SeekFrom::Start(15000 * 8)).unwrap();
        file.write_all(&[255; 8]).unwrap();
    }
    let mut output = vec![1.; 40000];
    assert!(matches!(
        host.render(&mut output),
        Err(AudioError::Stream(_))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    assert!(host.is_failed());
    assert!(matches!(host.sync(&world), Err(AudioError::FailedSession)));
    assert!(matches!(
        host.check_errors(),
        Err(AudioError::FailedSession)
    ));
    assert!(matches!(
        host.render(&mut output),
        Err(AudioError::FailedSession)
    ));
}

#[test]
fn parent_motion_and_listener_rotation_change_spatial_output_without_scale_rotation_confusion() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 20000]);
    let mut node = source(true);
    node.source.as_mut().unwrap().spatial = Some(AudioSpatial {
        min_distance: 1.,
        max_distance: 100.,
    });
    let listener = CookedAudio {
        entity: id(3),
        bus: None,
        source: None,
        listener: true,
    };
    let mut world = world(vec![bus(), node, listener]);
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(4)),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(3),
            parent: Some(id(5)),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.for_each_moving_mut(|entity, t, _| {
        if entity == id(4) {
            t.translation = [-5., 0., 0.];
        }
        if entity == id(5) {
            t.scale = [2., 3., 4.];
        }
    });
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    let left = render(&mut host, 1024);
    let balance = |pcm: &[f32]| {
        let f = pcm[128..].as_chunks::<2>().0;
        f.iter().map(|v| v[0] * v[0] - v[1] * v[1]).sum::<f32>()
    };
    assert!(balance(&left) > 0.1);
    world.for_each_moving_mut(|entity, t, _| {
        if entity == id(4) {
            t.translation[0] = 5.;
        }
    });
    host.sync(&world).unwrap();
    assert!(balance(&render(&mut host, 1024)) < -0.1);
    world.for_each_moving_mut(|entity, t, _| {
        if entity == id(5) {
            t.rotation = [0., 1., 0., 0.];
        }
    });
    host.sync(&world).unwrap();
    assert!(balance(&render(&mut host, 1024)) > 0.1);
    world.for_each_moving_mut(|entity, t, _| {
        if entity == id(4) {
            t.translation[0] = 2_000_000.;
        }
    });
    assert!(matches!(host.sync(&world), Err(AudioError::Invalid(_))));
    assert!(!host.is_failed());
}

#[test]
fn full_voice_capacity_replacement_waits_for_acknowledged_stops_without_growth() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.001; 2]; 1024]);
    let entities = (1..=128)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        })
        .collect();
    let nodes: Vec<_> = (1..=128)
        .map(|n| {
            let mut s = source(false);
            s.entity = id(n);
            s.source.as_mut().unwrap().bus = None;
            s
        })
        .collect();
    let mut world = NativeWorld::new(
        CookedScene::new(id(999), 60, entities)
            .unwrap()
            .with_audio(nodes.clone())
            .unwrap(),
    );
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    assert!(energy(&render(&mut host, 256)) > 0.001);
    for generation in 0..3 {
        let mut commands = FrameCommands::default();
        for mut node in nodes.clone() {
            node.source.as_mut().unwrap().start_seconds = (generation + 1) as f64 / 1000.;
            commands
                .push(StructuralCommand::SetAudio {
                    entity: node.entity,
                    audio: Some(node),
                })
                .unwrap();
        }
        world.apply(&mut commands).unwrap();
        host.sync(&world).unwrap();
        assert_eq!(host.source_state(id(1)).unwrap(), SourceState::Pending);
        host.sync(&world).unwrap();
        assert_eq!(host.source_state(id(1)).unwrap(), SourceState::Pending);
        for _ in 0..3 {
            render(&mut host, 256);
            host.sync(&world).unwrap();
            if host.source_state(id(1)).unwrap() != SourceState::Pending {
                break;
            }
        }
        assert_ne!(host.source_state(id(1)).unwrap(), SourceState::Pending);
        assert!(energy(&render(&mut host, 256)) > 0.001);
    }
}

#[test]
fn clip_table_bounds_fingerprints_and_retained_file_identity_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 1024]);
    let world = world(vec![bus(), source(true)]);
    let too_many = (0..129)
        .map(|n| ClipBinding::load(id(1000 + n), temp.path(), &key).unwrap())
        .collect();
    assert!(matches!(
        SceneAudio::offline(&world, too_many, 48000),
        Err(AudioError::Capacity)
    ));
    let other = "0".repeat(64);
    fs::copy(
        temp.path().join(format!("{key}.json")),
        temp.path().join(format!("{other}.json")),
    )
    .unwrap();
    assert!(
        ClipBinding::load(id(90), temp.path(), &other).is_err(),
        "existing manifest cannot be rebound under an unexpected fingerprint"
    );
    let retained = binding(&temp, &key);
    let pcm = fs::read_dir(temp.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.extension().unwrap() == "ipcm")
        .unwrap();
    let previous = temp.path().join("retained-original");
    fs::rename(&pcm, &previous).unwrap();
    fs::write(&pcm, [255; 32]).unwrap();
    let mut host = SceneAudio::offline(&world, vec![retained], 48000).unwrap();
    assert!(
        energy(&render(&mut host, 2048)) > 0.01,
        "runtime must use retained verified handle, not re-open a replacement path"
    );
}

#[test]
fn failed_render_is_terminal_and_clock_rewind_is_retryable_without_mixer_changes() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let mut world = world(vec![bus(), source(false)]);
    let bytes = CookedScene::new(
        id(99),
        60,
        (1..=5)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_audio(vec![bus(), source(false)])
    .unwrap()
    .to_bytes();
    let previous = NativeWorld::from_bytes(&bytes).unwrap();
    let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
    world.step().unwrap();
    host.sync(&world).unwrap();
    assert!(matches!(host.sync(&previous), Err(AudioError::Invalid(_))));
    assert!(!host.is_failed());
    host.sync(&world).unwrap();
    assert!(energy(&render(&mut host, 1024)) > 0.01);
    let mut odd = [1.; 3];
    assert!(matches!(host.render(&mut odd), Err(AudioError::Backend(_))));
    assert_eq!(odd, [0.; 3]);
    assert!(host.is_failed());
    assert!(matches!(host.sync(&world), Err(AudioError::FailedSession)));
}

#[test]
fn metadata_limit_counts_retained_allocations_not_only_logical_string_lengths() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 16]);
    let mut metadata: incant_assets_runtime::AudioMetadata =
        serde_json::from_slice(&fs::read(temp.path().join(format!("{key}.json"))).unwrap())
            .unwrap();
    metadata.dependency.path = "a".repeat(incant_audio_runtime::MAX_CLIP_METADATA_BYTES / 16 - 400);
    metadata.fingerprint = incant_assets_runtime::audio::key(&metadata.dependency).unwrap();
    let key = metadata.fingerprint.clone();
    fs::write(
        temp.path().join(format!("{key}.json")),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let approximate = incant_assets_runtime::load_audio(temp.path(), &key)
        .unwrap()
        .resident_bytes();
    assert!(approximate * 16 < incant_audio_runtime::MAX_CLIP_METADATA_BYTES);
    let bindings = (0..16)
        .map(|n| ClipBinding::load(id(90 + n), temp.path(), &key).unwrap())
        .collect();
    let world = world(vec![bus(), source(false)]);
    assert!(matches!(
        SceneAudio::offline(&world, bindings, 48000),
        Err(AudioError::Capacity)
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn actual_background_decoder_error_is_observable_without_an_audio_device() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    let binding = binding(&temp, &key);
    // Binding validation succeeded; corrupt its retained open file before the
    // real Kira decoder thread starts. This bypasses the synchronous offline
    // stream adapter while retaining the device-free pull backend.
    let pcm = fs::read_dir(temp.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.extension().unwrap() == "ipcm")
        .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(pcm)
        .unwrap()
        .write_all(&[255; 8])
        .unwrap();
    let world = world(vec![bus(), source(true)]);
    let settings =
        incant_audio_runtime::AudioManagerSettings::<incant_audio_runtime::OfflineBackend> {
            backend_settings: 48000,
            ..Default::default()
        };
    let mut host = SceneAudio::new(&world, vec![binding], settings).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match host.check_errors() {
            Err(AudioError::Stream(_)) => break,
            Ok(()) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "decoder error was lost"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(error) => panic!("unexpected error: {error}"),
        }
    }
    assert!(host.is_failed());
    assert!(matches!(host.sync(&world), Err(AudioError::FailedSession)));
    assert!(matches!(
        host.source_state(id(2)),
        Err(AudioError::FailedSession)
    ));
}

#[test]
fn master_suspend_freezes_static_and_streamed_cursors_without_resuming_paused_sources() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    for streaming in [false, true] {
        let world = world(vec![bus(), source(streaming)]);
        let mut control = SceneAudio::offline(&world, vec![binding(&temp, &key)], 44100).unwrap();
        let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 44100).unwrap();
        assert_eq!(render(&mut host, 512), render(&mut control, 512));
        host.set_suspended(true).unwrap();
        assert!(render(&mut host, 2048).iter().all(|v| *v == 0.));
        assert_eq!(host.source_state(id(2)).unwrap(), SourceState::Playing);
        host.set_suspended(false).unwrap();
        let resumed = render(&mut host, 1024);
        let expected = render(&mut control, 1024);
        // Kira interpolates the resume gain over its first internal chunk even
        // for a zero-duration tween. Subsequent exact samples prove no cursor
        // advance occurred while suspended, in both static and streaming paths.
        assert_eq!(&resumed[256..], &expected[256..]);
        let mut paused = source(streaming);
        paused.source.as_mut().unwrap().playing = false;
        let paused_world = self::world(vec![bus(), paused]);
        host.sync(&paused_world).unwrap();
        render(&mut host, 256);
        host.set_suspended(true).unwrap();
        render(&mut host, 256);
        host.set_suspended(false).unwrap();
        assert!(render(&mut host, 1024).iter().all(|v| *v == 0.));
        assert_eq!(host.source_state(id(2)).unwrap(), SourceState::Paused);
    }
}
