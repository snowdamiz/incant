#![cfg(feature = "native-checkpoints")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
#[path = "native_checkpoints/cases.rs"]
mod native_checkpoint_cases;
#[cfg(feature = "ui")]
#[path = "native_checkpoints/ui.rs"]
mod ui;

use incant_audio_runtime::ClipBinding;
use incant_game_runtime::{GameCheckpoint, OfflineGameSession, PendingGameCheckpoint};
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, CookedAudio, CookedEntity, CookedPhysics,
    CookedPhysics2D, CookedScene, NativeAudioSource, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget, StableId,
};
use incant_script::CompiledScript;
use incant_types::{Collider, Collider2D, ColliderShape, ColliderShape2D, RigidBody, Transform};
use std::path::Path;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn payload() -> CheckpointPayloadBudget {
    CheckpointPayloadBudget::new(64 * 1024 * 1024).unwrap()
}
fn planar() -> PlanarCheckpointWorkBudget {
    PlanarCheckpointWorkBudget::new(100_000_000)
}
fn spatial() -> SpatialCheckpointWorkBudget {
    SpatialCheckpointWorkBudget::new(100_000_000)
}
fn capture(game: &mut OfflineGameSession) -> GameCheckpoint {
    game.checkpoint(&mut payload(), &mut planar(), &mut spatial())
        .unwrap()
}
fn prepare(checkpoint: &GameCheckpoint) -> PendingGameCheckpoint {
    checkpoint
        .prepare(
            &mut payload(),
            &mut planar(),
            &mut spatial(),
            &mut CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000),
        )
        .unwrap()
}
fn scene(with_audio: bool) -> CookedScene {
    let scene = CookedScene::new(
        id(99),
        144,
        (1..=9)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 9).then(|| id(8)),
                velocity: None,
                transform: Transform {
                    translation: [
                        0.,
                        if n >= 8 {
                            0.
                        } else if [4, 6].contains(&n) {
                            -0.5
                        } else {
                            1.
                        },
                        0.,
                    ],
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap()
    .with_physics(
        (4..=5)
            .map(|n| CookedPhysics {
                entity: id(n),
                body: (n == 5).then(RigidBody::default),
                angular_velocity: None,
                collider: Collider {
                    shape: ColliderShape::Box {
                        half_extents: if n == 5 { [0.4; 3] } else { [4., 0.5, 4.] },
                    },
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap()
    .with_physics_2d(
        (6..=7)
            .map(|n| CookedPhysics2D {
                entity: id(n),
                body: (n == 7).then(RigidBody::default),
                angular_velocity: None,
                collider: Collider2D {
                    shape: ColliderShape2D::Box {
                        half_extents: if n == 7 { [0.4; 2] } else { [4., 0.5] },
                    },
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap();
    if !with_audio {
        return scene;
    }
    scene
        .with_audio(
            [2, 3]
                .into_iter()
                .map(|n| CookedAudio {
                    entity: id(n),
                    bus: None,
                    listener: false,
                    source: Some(NativeAudioSource {
                        transport_epoch: 0,
                        clip: id(n + 88),
                        bus: None,
                        gain_db: -4.,
                        pan: 0.2,
                        rate: if n == 2 { 1.13 } else { 0.87 },
                        start_seconds: 0.00037,
                        playing: true,
                        looping: true,
                        streaming: n == 3,
                        spatial: None,
                    }),
                })
                .collect(),
        )
        .unwrap()
}
fn code() -> CompiledScript {
    CompiledScript::compile(r#"exports.default={persistence:{version:1},update(t){
        const n=Number(t.state.get()||0)+1+t.timers.events.length; t.state.set(n);
        if(t.clock.tick===1)t.timers.set('pulse',3,3);
        for(const c of t.chunks)for(let i=0;i<c.ids.length;i++){
            if(c.ids[i]==='00000000000000000000000001')c.transforms[i*c.transformStride]=n;
            if(t.clock.tick===67&&c.ids[i]==='00000000000000000000000005')c.transforms[i*c.transformStride+1]+=0.75;
        }
    }}"#).unwrap()
}
fn clips(root: &Path) -> Vec<ClipBinding> {
    [90, 91]
        .into_iter()
        .map(|n| {
            let dir = root.join(n.to_string());
            let key = audio_support::cooked(
                &dir,
                &(0..2309)
                    .map(|i| [(i as f32 * 0.13 + n as f32).sin() * 0.3; 2])
                    .collect::<Vec<_>>(),
            );
            ClipBinding::load(id(n), &dir, &key).unwrap()
        })
        .collect()
}
fn tick(game: &mut OfflineGameSession) -> Vec<f32> {
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    pcm
}

#[test]
fn mixed_physics_script_and_real_audio_resume_with_the_captured_sample_clock() {
    let temp = tempfile::tempdir().unwrap();
    let bindings = clips(temp.path());
    let mut live = OfflineGameSession::new(
        scene(true),
        code(),
        Default::default(),
        bindings.clone(),
        44100,
    )
    .unwrap();
    for _ in 0..67 {
        assert!(tick(&mut live).iter().any(|v| v.abs() > 0.001));
    }
    let checkpoint = capture(&mut live);
    let pending = prepare(&checkpoint);
    assert_eq!(pending.sample_rate(), Some(44100));
    let mut restored = OfflineGameSession::from_checkpoint(pending, bindings).unwrap();
    assert_ne!(
        live.world().unwrap().incarnation(),
        restored.world().unwrap().incarnation()
    );
    assert_eq!(restored.rendered_frames().unwrap(), 20518);
    for _ in 0..37 {
        assert_eq!(tick(&mut live), tick(&mut restored));
        let a = live.world().unwrap();
        let b = restored.world().unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.physics_events(), b.physics_events());
        assert_eq!(a.physics_2d_events(), b.physics_2d_events());
        for n in [4, 5, 6, 7] {
            assert_eq!(a.physics_state(id(n)), b.physics_state(id(n)));
            assert_eq!(a.physics_2d_state(id(n)), b.physics_2d_state(id(n)));
        }
        for n in [2, 3] {
            assert_eq!(
                live.source_state(id(n)).unwrap(),
                restored.source_state(id(n)).unwrap()
            );
        }
    }
    assert!(live.checkpoint_bytes().is_err());
    assert!(restored.checkpoint_bytes().is_err());
}

#[test]
fn silent_suspended_checkpoint_remains_suspended_without_resetting_game_time() {
    let mut live =
        OfflineGameSession::new(scene(false), code(), Default::default(), vec![], 44100).unwrap();
    for _ in 0..13 {
        tick(&mut live);
    }
    live.set_suspended(true).unwrap();
    let mut restored =
        OfflineGameSession::from_checkpoint(prepare(&capture(&mut live)), vec![]).unwrap();
    let before = restored.world().unwrap().snapshot();
    let mut pcm = vec![7.; restored.next_output_frames().unwrap() * 2];
    assert!(matches!(
        restored.tick(&[], &mut pcm),
        Err(incant_game_runtime::GameError::Suspended)
    ));
    assert_eq!(restored.world().unwrap().snapshot(), before);
    assert!(pcm.iter().all(|n| *n == 7.));
    restored.set_suspended(false).unwrap();
    live.set_suspended(false).unwrap();
    assert_eq!(tick(&mut live), tick(&mut restored));
    assert_eq!(
        live.world().unwrap().snapshot(),
        restored.world().unwrap().snapshot()
    );
    // Recapture validates the real audio owner against the absolute world clock.
    let mut again =
        OfflineGameSession::from_checkpoint(prepare(&capture(&mut restored)), vec![]).unwrap();
    for _ in 0..17 {
        assert_eq!(tick(&mut restored), tick(&mut again));
        assert_eq!(
            restored.world().unwrap().snapshot(),
            again.world().unwrap().snapshot()
        );
    }
    capture(&mut again);
}
