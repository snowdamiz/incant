#![cfg(feature = "scene")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
#[path = "checkpoints/transport.rs"]
mod transport;
use incant_audio_runtime::{
    ClipBinding, OfflineBackend, PendingAudioCheckpoint, SceneAudio, SourceState,
};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedScene, FrameCommands, NativeAudioBus, NativeAudioSource,
    NativeWorld, StableId, StructuralCommand,
};
use incant_types::Transform;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn node(n: u128, streaming: bool) -> CookedAudio {
    CookedAudio {
        entity: id(n),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: 0,
            clip: id(90),
            bus: Some(id(1)),
            gain_db: -4.,
            pan: 0.,
            rate: 1.13,
            start_seconds: 0.00037,
            playing: true,
            looping: true,
            streaming,
            spatial: None,
        }),
    }
}
fn scene() -> CookedScene {
    CookedScene::new(
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
    .with_audio(vec![
        CookedAudio {
            entity: id(1),
            bus: Some(NativeAudioBus {
                parent: None,
                gain_db: -2.,
            }),
            source: None,
            listener: false,
        },
        node(2, false),
        node(3, true),
    ])
    .unwrap()
}
fn fixture(root: &std::path::Path) -> String {
    let key = support::cooked(
        root,
        &(0..2309)
            .map(|i| [(i as f32 * 0.13).sin() * 0.3; (2)])
            .collect::<Vec<_>>(),
    );
    let p = root.join(format!("{key}.json"));
    let mut m: incant_assets_runtime::AudioMetadata =
        serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    m.sample_rate = 22050;
    std::fs::write(p, serde_json::to_vec(&m).unwrap()).unwrap();
    key
}
fn clips(root: &std::path::Path, key: &str) -> Vec<ClipBinding> {
    vec![ClipBinding::load(id(90), root, key).unwrap()]
}
fn render(audio: &mut SceneAudio<OfflineBackend>, n: usize) -> Vec<f32> {
    let mut out = vec![0.; 2 * n];
    audio.render(&mut out).unwrap();
    out
}
fn edit(w: &mut NativeWorld, n: CookedAudio) {
    let mut c = FrameCommands::default();
    c.push(StructuralCommand::SetAudio {
        entity: n.entity,
        audio: Some(n),
    })
    .unwrap();
    w.apply(&mut c).unwrap();
}
#[test]
fn binary_owner_resumes_pending_replacements_and_future_mixed_pcm_exactly() {
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let mut world = NativeWorld::new(scene());
    let mut fresh = NativeWorld::new(scene());
    let mut live = SceneAudio::offline(&world, clips(temp.path(), &key), 44100).unwrap();
    render(&mut live, 193);
    let mut changed = node(2, true);
    changed.source.as_mut().unwrap().start_seconds = 0.02;
    edit(&mut world, changed.clone());
    edit(&mut fresh, changed);
    assert!(live.checkpoint_for_world(&world).is_err());
    live.sync(&world).unwrap();
    render(&mut live, 71);
    assert_eq!(live.source_state(id(2)).unwrap(), SourceState::Pending);
    let snapshot = live.checkpoint_for_world(&world).unwrap();
    assert!(snapshot.validate_world(&fresh).is_err());
    let pending = PendingAudioCheckpoint::new(&fresh, snapshot.bytes()).unwrap();
    let mut restored = pending
        .restore(&fresh, clips(temp.path(), &key), 44100)
        .unwrap();
    assert_eq!(restored.source_state(id(2)).unwrap(), SourceState::Pending);
    assert_eq!(
        restored.checkpoint_for_world(&fresh).unwrap().bytes(),
        snapshot.bytes()
    );
    for (step, count) in [3, 171, 31, 700, 23].into_iter().enumerate() {
        if step == 2 {
            for w in [&mut world, &mut fresh] {
                let mut changed = node(3, true);
                changed.source.as_mut().unwrap().playing = false;
                edit(w, changed);
            }
        }
        if step == 3 {
            for w in [&mut world, &mut fresh] {
                let mut changed = node(3, true);
                changed.source.as_mut().unwrap().rate = 0.43;
                changed.source.as_mut().unwrap().gain_db = -8.;
                edit(w, changed);
            }
        }
        live.sync(&world).unwrap();
        restored.sync(&fresh).unwrap();
        assert_eq!(render(&mut live, count), render(&mut restored, count));
        assert_eq!(
            live.checkpoint_for_world(&world).unwrap().bytes(),
            restored.checkpoint_for_world(&fresh).unwrap().bytes()
        );
    }
}
#[test]
fn malformed_sections_and_catalog_refuse_without_changing_live_audio() {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let world = NativeWorld::new(scene());
    let mut live = SceneAudio::offline(&world, clips(temp.path(), &key), 32000).unwrap();
    render(&mut live, 53);
    let snapshot = live.checkpoint_for_world(&world).unwrap();
    let mut control = snapshot
        .restore(&world, clips(temp.path(), &key), 32000)
        .unwrap();
    for n in [
        0,
        7,
        47,
        snapshot.bytes().len() - 1,
        incant_audio_runtime::MAX_AUDIO_CHECKPOINT_BYTES + 1,
    ] {
        let mut b = snapshot.bytes().to_vec();
        b.resize(n, 0);
        assert!(PendingAudioCheckpoint::new(&world, &b).is_err());
    }
    for p in [8, 12, 48, 80, 96, 156, 158, 160, 162] {
        let mut b = snapshot.bytes().to_vec();
        b[p] = 255;
        let mut h = Sha256::new();
        h.update(&b[..16]);
        h.update(&b[48..]);
        b[16..48].copy_from_slice(&h.finalize());
        assert!(
            PendingAudioCheckpoint::new(&world, &b).is_err(),
            "offset {p}"
        );
    }
    for (p, value) in [
        (104, u32::MAX),
        (124, f32::NAN.to_bits()),
        (152, f32::INFINITY.to_bits()),
    ] {
        let mut b = snapshot.bytes().to_vec();
        b[p..p + 4].copy_from_slice(&value.to_le_bytes());
        let mut h = Sha256::new();
        h.update(&b[..16]);
        h.update(&b[48..]);
        b[16..48].copy_from_slice(&h.finalize());
        assert!(PendingAudioCheckpoint::new(&world, &b).is_err());
    }
    assert!(snapshot.restore(&world, vec![], 32000).is_err());
    assert!(
        snapshot
            .restore(&world, clips(temp.path(), &key), 48000)
            .is_err()
    );
    assert_eq!(render(&mut live, 731), render(&mut control, 731));
}

#[test]
fn playing_declaration_cannot_disagree_with_saved_transport_before_reconstruction() {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let mut world = NativeWorld::new(scene());
    let mut live = SceneAudio::offline(&world, clips(temp.path(), &key), 44100).unwrap();
    render(&mut live, 53);
    let saved = live.checkpoint_for_world(&world).unwrap();
    let mut paused = node(2, false);
    paused.source.as_mut().unwrap().playing = false;
    edit(&mut world, paused);
    // Forged world data is individually valid, but does not authorize an active
    // retained static voice under a paused declaration.
    assert!(PendingAudioCheckpoint::new(&world, saved.bytes()).is_err());
    live.sync(&world).unwrap();
    render(&mut live, 53);
    let paused = live.checkpoint_for_world(&world).unwrap();
    edit(&mut world, node(2, false));
    assert!(PendingAudioCheckpoint::new(&world, paused.bytes()).is_err());
    live.sync(&world).unwrap();
    render(&mut live, 53);
    let saved = live.checkpoint_for_world(&world).unwrap();
    let mut forged = saved.bytes().to_vec();
    let tag = forged.windows(8).position(|v| v == b"KIRAST01").unwrap();
    forged[tag + 12] = 1; // A paused transport under a playing declaration.
    let mut h = Sha256::new();
    h.update(&forged[..16]);
    h.update(&forged[48..]);
    forged[16..48].copy_from_slice(&h.finalize());
    assert!(PendingAudioCheckpoint::new(&world, &forged).is_err());
    assert_eq!(live.source_state(id(2)).unwrap(), SourceState::Playing);
}

#[test]
fn removed_listener_history_preserves_future_spatial_restart_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let base = scene();
    let mut entities = base.entities().to_vec();
    entities[3].transform.translation = [2., 0., 0.];
    let mut nodes = base.audio().to_vec();
    let mut spatial = node(3, true);
    spatial.source.as_mut().unwrap().spatial = Some(incant_types::AudioSpatial {
        min_distance: 0.1,
        max_distance: 10.,
    });
    nodes[2] = spatial.clone();
    let listener = CookedAudio {
        entity: id(4),
        bus: None,
        source: None,
        listener: true,
    };
    nodes.push(listener.clone());
    let scene = CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_audio(nodes)
        .unwrap();
    let mut world = NativeWorld::new(scene);
    let mut live = SceneAudio::offline(&world, clips(temp.path(), &key), 44100).unwrap();
    render(&mut live, 197);
    let mut removals = FrameCommands::default();
    for n in [3, 4] {
        removals
            .push(StructuralCommand::SetAudio {
                entity: id(n),
                audio: None,
            })
            .unwrap();
    }
    world.apply(&mut removals).unwrap();
    live.sync(&world).unwrap();
    render(&mut live, 113);
    let save = live.checkpoint_for_world(&world).unwrap();
    let mut fresh = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let mut restored = PendingAudioCheckpoint::new(&fresh, save.bytes())
        .unwrap()
        .restore(&fresh, clips(temp.path(), &key), 44100)
        .unwrap();
    assert_eq!(
        restored.checkpoint_for_world(&fresh).unwrap().bytes(),
        save.bytes()
    );
    for w in [&mut world, &mut fresh] {
        edit(w, listener.clone());
        edit(w, spatial.clone());
    }
    live.sync(&world).unwrap();
    restored.sync(&fresh).unwrap();
    // Retired voices are drained first; then the new spatial voice is admitted.
    assert_eq!(render(&mut live, 113), render(&mut restored, 113));
    live.sync(&world).unwrap();
    restored.sync(&fresh).unwrap();
    assert_eq!(render(&mut live, 731), render(&mut restored, 731));
}
