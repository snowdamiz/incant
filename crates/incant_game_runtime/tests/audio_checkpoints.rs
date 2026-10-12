//! Actual VM/timer + static/streamed Kira continuation through the ordinary game.
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::{HostedOfflineGameSession, OfflineGameSession, SceneHost};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedScene, NativeAudioBus, NativeAudioSource, NativeWorld,
    StableId,
};
use incant_script::CompiledScript;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        144,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Default::default(),
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
                gain_db: -3.,
            }),
            source: None,
            listener: false,
        },
        source(2, false, 90),
        source(3, true, 91),
    ])
    .unwrap()
}
fn source(n: u128, streaming: bool, clip: u128) -> CookedAudio {
    CookedAudio {
        entity: id(n),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: 0,
            clip: id(clip),
            bus: Some(id(1)),
            gain_db: -4.,
            pan: 0.2,
            rate: if streaming { 0.87 } else { 1.13 },
            start_seconds: 0.00037,
            playing: true,
            looping: true,
            streaming,
            spatial: None,
        }),
    }
}
fn fixture(root: &std::path::Path, n: u128, rate: u32) -> String {
    let root = root.join(n.to_string());
    let key = support::cooked(
        &root,
        &(0..2309)
            .map(|i| [(i as f32 * 0.13 + n as f32).sin() * 0.3; 2])
            .collect::<Vec<_>>(),
    );
    let path = root.join(format!("{key}.json"));
    let mut m: incant_assets_runtime::AudioMetadata =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    m.sample_rate = rate;
    std::fs::write(path, serde_json::to_vec(&m).unwrap()).unwrap();
    key
}
fn bindings(root: &std::path::Path, keys: &[String; 2]) -> Vec<ClipBinding> {
    [90u128, 91]
        .into_iter()
        .zip(keys)
        .map(|(idn, key)| ClipBinding::load(id(idn), &root.join(idn.to_string()), key).unwrap())
        .collect()
}
fn code() -> CompiledScript {
    CompiledScript::compile(r#"exports.default={persistence:{version:1},update(t){
 const n=Number(t.state.get()||0)+1+t.timers.events.length;t.state.set(n);
 if(t.clock.tick===1)t.timers.set('pulse',3,3);
 for(const c of t.chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]=n;
 const a='00000000000000000000000002',b='00000000000000000000000003',bus='00000000000000000000000001';
 if(t.clock.tick===14)t.commands.setAudioPlaying(a,false);
 if(t.clock.tick===18)t.commands.setAudioPlaying(a,true);
 if(t.clock.tick===21){t.commands.setAudioGain(b,-8);t.commands.setAudioBusGain(bus,-6);}
}}"#).unwrap()
}
fn tick(game: &mut OfflineGameSession) -> Vec<f32> {
    let mut out = vec![0.; 2 * game.next_output_frames().unwrap()];
    game.tick(&[], &mut out).unwrap();
    out
}
#[test]
fn ordinary_composed_restore_keeps_real_mixed_audio_gameplay_and_timers_exact() {
    let root = tempfile::tempdir().unwrap();
    let keys = [
        fixture(root.path(), 90, 22050),
        fixture(root.path(), 91, 32000),
    ];
    let scene = scene();
    let code = code();
    let mut live = OfflineGameSession::new(
        scene.clone(),
        code.clone(),
        Default::default(),
        bindings(root.path(), &keys),
        44100,
    )
    .unwrap();
    for _ in 0..13 {
        assert!(tick(&mut live).iter().any(|v| v.abs() > 0.001));
    }
    let save = live.checkpoint_bytes().unwrap();
    assert_eq!(&save[8..12], &3u32.to_le_bytes());
    assert_eq!(&save[136 + 8..136 + 12], &8u32.to_le_bytes());
    let mut restored = OfflineGameSession::from_checkpoint_bytes(
        &save,
        &scene,
        code,
        Default::default(),
        bindings(root.path(), &keys),
        44100,
    )
    .unwrap();
    assert_ne!(
        live.world().unwrap().spatial_view().unwrap().identity(),
        restored.world().unwrap().spatial_view().unwrap().identity()
    );
    for _ in 0..27 {
        assert_eq!(tick(&mut live), tick(&mut restored));
        assert_eq!(
            live.checkpoint_bytes().unwrap(),
            restored.checkpoint_bytes().unwrap()
        );
    }
    assert_eq!(restored.rendered_frames().unwrap(), 12250);
    assert_eq!(restored.world().unwrap().tick(), 40);
}
struct Host;
impl SceneHost for Host {
    type Settings = Arc<AtomicUsize>;
    type Error = std::convert::Infallible;
    fn prepare(_: &NativeWorld, calls: Self::Settings) -> Result<Self, Self::Error> {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(Self)
    }
    fn sync(&mut self, _: &NativeWorld) -> Result<(), Self::Error> {
        Ok(())
    }
}
#[test]
fn audio_catalog_and_output_clock_refuse_before_native_host_preparation() {
    let root = tempfile::tempdir().unwrap();
    let keys = [
        fixture(root.path(), 90, 22050),
        fixture(root.path(), 91, 32000),
    ];
    let scene = scene();
    let code = code();
    let mut live = OfflineGameSession::new(
        scene.clone(),
        code.clone(),
        Default::default(),
        bindings(root.path(), &keys),
        44100,
    )
    .unwrap();
    tick(&mut live);
    let save = live.checkpoint_bytes().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    for (clips, rate) in [(vec![], 44100), (bindings(root.path(), &keys), 48000)] {
        assert!(
            HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
                &save,
                &scene,
                code.clone(),
                Default::default(),
                clips,
                rate,
                Arc::clone(&calls)
            )
            .is_err()
        );
    }
    let mut corrupt = save.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
            &corrupt,
            &scene,
            code.clone(),
            Default::default(),
            bindings(root.path(), &keys),
            44100,
            Arc::clone(&calls)
        )
        .is_err()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(live.checkpoint_bytes().unwrap(), save);
    assert!(tick(&mut live).iter().any(|v| v.abs() > 0.001));
}

#[test]
fn startup_and_master_suspension_restore_without_spending_a_sample_or_game_tick() {
    let root = tempfile::tempdir().unwrap();
    let keys = [
        fixture(root.path(), 90, 22050),
        fixture(root.path(), 91, 32000),
    ];
    for before in [0, 13] {
        for suspended in [false, true] {
            let initial = scene();
            let code = code();
            let mut live = OfflineGameSession::new(
                initial.clone(),
                code.clone(),
                Default::default(),
                bindings(root.path(), &keys),
                44100,
            )
            .unwrap();
            for _ in 0..before {
                tick(&mut live);
            }
            // Both native command slots may be pending; the last logical state
            // does not uniquely describe Kira's independent pause/resume queues.
            live.set_suspended(true).unwrap();
            live.set_suspended(false).unwrap();
            if suspended {
                live.set_suspended(true).unwrap();
            }
            let saved = live.checkpoint_bytes().unwrap();
            let mut restored = OfflineGameSession::from_checkpoint_bytes(
                &saved,
                &initial,
                code,
                Default::default(),
                bindings(root.path(), &keys),
                44100,
            )
            .unwrap();
            assert_eq!(restored.checkpoint_bytes().unwrap(), saved);
            if suspended {
                let mut out = vec![7.; 2 * live.next_output_frames().unwrap()];
                assert!(live.tick(&[], &mut out).is_err());
                assert!(restored.tick(&[], &mut out).is_err());
                assert!(out.iter().all(|v| *v == 7.));
                assert_eq!(restored.world().unwrap().tick(), before);
                live.set_suspended(false).unwrap();
                restored.set_suspended(false).unwrap();
            }
            for _ in 0..27 {
                assert_eq!(
                    tick(&mut live),
                    tick(&mut restored),
                    "before={before} suspended={suspended}"
                );
                assert_eq!(
                    live.checkpoint_bytes().unwrap(),
                    restored.checkpoint_bytes().unwrap()
                );
            }
        }
    }
}
