use super::*;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_game_runtime::{GameSession, HostedOfflineGameSession, SceneHost};
use incant_runtime::{NativeWorld, WorldIdentity};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy)]
pub(super) enum Failure {
    None,
    Error,
    Panic,
}
pub(super) struct Host {
    identity: WorldIdentity,
}
impl SceneHost for Host {
    type Settings = (Arc<AtomicUsize>, Failure);
    type Error = std::io::Error;
    fn prepare(world: &NativeWorld, settings: Self::Settings) -> Result<Self, Self::Error> {
        settings.0.fetch_add(1, Ordering::Relaxed);
        match settings.1 {
            Failure::None => {}
            Failure::Error => return Err(std::io::Error::other("refused host")),
            Failure::Panic => panic!("host setup trap"),
        }
        Ok(Self {
            identity: world.spatial_view().unwrap().identity().clone(),
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        assert_eq!(&self.identity, world.spatial_view().unwrap().identity());
        Ok(())
    }
}

#[test]
fn audio_and_host_failures_keep_the_original_game_live() {
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
    for _ in 0..13 {
        tick(&mut live);
    }
    let checkpoint = capture(&mut live);
    let before = live.world().unwrap().snapshot();
    let incarnation = live.world().unwrap().incarnation();
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint(
            prepare(&checkpoint),
            vec![],
            (Arc::clone(&calls), Failure::None),
        )
        .is_err()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    for fail in [Failure::Error, Failure::Panic] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            HostedOfflineGameSession::<Host>::from_checkpoint(
                prepare(&checkpoint),
                bindings.clone(),
                (Arc::clone(&calls), fail),
            )
        }));
        assert!(result.is_err() || result.unwrap().is_err());
        assert_eq!(live.world().unwrap().snapshot(), before);
        assert_eq!(live.world().unwrap().incarnation(), incarnation);
    }
    let mut hosted = HostedOfflineGameSession::<Host>::from_checkpoint(
        prepare(&checkpoint),
        bindings,
        (Arc::clone(&calls), Failure::None),
    )
    .unwrap();
    for _ in 0..7 {
        let mut pcm = vec![0.; hosted.next_output_frames().unwrap() * 2];
        hosted.tick(&[], &mut pcm).unwrap();
        assert_eq!(tick(&mut live), pcm);
        assert_eq!(
            live.world().unwrap().snapshot(),
            hosted.world().unwrap().snapshot()
        );
    }
}

#[test]
fn checkpoint_clock_families_cannot_be_silently_interchanged() {
    let mut offline =
        OfflineGameSession::new(scene(false), code(), Default::default(), vec![], 44100).unwrap();
    tick(&mut offline);
    let saved = capture(&mut offline);
    assert!(
        GameSession::<OfflineBackend>::from_checkpoint(
            prepare(&saved),
            vec![],
            AudioManagerSettings {
                backend_settings: 44100,
                ..Default::default()
            },
        )
        .is_err()
    );
    let mut backend = GameSession::<OfflineBackend>::new(
        scene(false),
        code(),
        Default::default(),
        vec![],
        AudioManagerSettings {
            backend_settings: 44100,
            ..Default::default()
        },
    )
    .unwrap();
    backend.tick(&[]).unwrap();
    let checkpoint = backend
        .checkpoint(&mut payload(), &mut planar(), &mut spatial())
        .unwrap();
    assert_eq!(prepare(&checkpoint).sample_rate(), None);
    assert!(OfflineGameSession::from_checkpoint(prepare(&checkpoint), vec![]).is_err());
    let mut restored = GameSession::<OfflineBackend>::from_checkpoint(
        prepare(&checkpoint),
        vec![],
        AudioManagerSettings {
            backend_settings: 44100,
            ..Default::default()
        },
    )
    .unwrap();
    backend.tick(&[]).unwrap();
    restored.tick(&[]).unwrap();
    assert_eq!(
        backend.world().unwrap().snapshot(),
        restored.world().unwrap().snapshot()
    );
}

#[test]
fn failed_complete_game_cannot_capture_and_small_budgets_do_not_change_a_live_one() {
    let fail = CompiledScript::compile(
        "exports.default={persistence:{version:1},update(){throw Error('fail')}};",
    )
    .unwrap();
    let mut game =
        OfflineGameSession::new(scene(false), fail, Default::default(), vec![], 44100).unwrap();
    let mut pcm = vec![7.; game.next_output_frames().unwrap() * 2];
    assert!(game.tick(&[], &mut pcm).is_err());
    assert!(game.is_failed());
    assert!(
        game.checkpoint(&mut payload(), &mut planar(), &mut spatial())
            .is_err()
    );

    let mut live =
        OfflineGameSession::new(scene(false), code(), Default::default(), vec![], 44100).unwrap();
    for _ in 0..13 {
        tick(&mut live);
    }
    let before = live.world().unwrap().snapshot();
    let incarnation = live.world().unwrap().incarnation();
    assert!(
        live.checkpoint(
            &mut CheckpointPayloadBudget::new(0).unwrap(),
            &mut planar(),
            &mut spatial()
        )
        .is_err()
    );
    assert_eq!(live.world().unwrap().snapshot(), before);
    assert_eq!(live.world().unwrap().incarnation(), incarnation);
    tick(&mut live);
}
