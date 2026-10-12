use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_input::{GamepadAxis, InputEvent};
use incant_play::{PlayError, PlayMode, PlaySession, PreparedPlay};
use incant_render_data::SceneInput;
use incant_render_game::{NativeRenderSettings, RenderInputConsumer};
use incant_render_native::UpdateKind;
use incant_runtime::{CookedEntity, CookedScene, CookedSpatial, StableId};
use incant_script::CompiledScript;
use incant_types::{Camera, CameraProjection, Transform};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn prepared(body: &str) -> PreparedPlay {
    PreparedPlay {
        scene: "00000000000000000000000063".into(),
        script_source_sha256: "test".into(),
        authored_scripts: false,
        cooked: CookedScene::new(
            id(99),
            60,
            vec![CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
        code: CompiledScript::compile(&format!(
            "exports.default={{update({{chunks}}){{{body}}}}};"
        ))
        .unwrap(),
        actions: Default::default(),
        clips: vec![],
        sample_rate: 48000,
        render: None,
    }
}
fn settings() -> AudioManagerSettings<OfflineBackend> {
    AudioManagerSettings {
        backend_settings: 48000,
        ..Default::default()
    }
}
fn position<C: RenderInputConsumer>(game: &PlaySession<OfflineBackend, C>) -> f64 {
    game.world()
        .unwrap()
        .inspect(id(1))
        .unwrap()
        .transform
        .translation[0]
}
fn invalid() -> InputEvent {
    InputEvent::GamepadAxis {
        id: 99,
        axis: GamepadAxis::LeftX,
        value: 0.5,
    }
}
const MOVE: &str = "chunks[0].transforms[0]+=2;";

#[test]
fn pause_step_resume_and_stop_act_on_disposable_native_state() {
    let mut game = PlaySession::<OfflineBackend>::new(prepared(MOVE), settings(), ()).unwrap();
    assert_eq!(game.mode(), PlayMode::Paused);
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(matches!(game.advance(&[]), Err(PlayError::Paused)));
    game.step(&[]).unwrap();
    assert_eq!(game.mode(), PlayMode::Paused);
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(position(&game), 2.);
    assert!(!game.refresh().unwrap());
    assert_eq!(game.world().unwrap().tick(), 1);
    game.resume().unwrap();
    assert!(matches!(game.step(&[]), Err(PlayError::NotPaused)));
    game.advance(&[]).unwrap();
    game.pause().unwrap();
    assert_eq!(position(&game), 4.);
    game.stop();
    assert_eq!(game.mode(), PlayMode::Stopped);
    assert!(matches!(game.world(), Err(PlayError::Stopped)));
    assert!(game.input().is_err());
    assert!(game.logs().is_err());
    assert!(game.resume().is_err());
    assert!(game.poll_audio().is_err());
    game.stop();
    let fresh = PlaySession::<OfflineBackend>::new(prepared(MOVE), settings(), ()).unwrap();
    assert_eq!(fresh.world().unwrap().tick(), 0);
    assert_eq!(position(&fresh), 0.);
}

#[test]
fn rejected_input_preserves_a_paused_step_and_guest_failure_is_terminal() {
    let mut game = PlaySession::<OfflineBackend>::new(prepared(MOVE), settings(), ()).unwrap();
    assert!(game.step(&[invalid()]).is_err());
    assert_eq!(game.mode(), PlayMode::Paused);
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(matches!(game.advance(&[]), Err(PlayError::Paused)));
    game.step(&[]).unwrap();
    assert_eq!(position(&game), 2.);
    let mut failed =
        PlaySession::<OfflineBackend>::new(prepared("throw Error('fixture');"), settings(), ())
            .unwrap();
    assert!(failed.step(&[]).is_err());
    assert_eq!(failed.mode(), PlayMode::Failed);
    assert!(matches!(failed.world(), Err(PlayError::Failed)));
    assert!(failed.refresh().is_err());
    assert!(failed.pause().is_err());
    assert!(failed.resume().is_err());
    failed.stop();
    assert_eq!(failed.mode(), PlayMode::Stopped);
}

#[derive(Default)]
struct Probe {
    accepts: AtomicUsize,
    drops: AtomicUsize,
    outcome: AtomicUsize,
}
struct Consumer(Arc<Probe>);
#[derive(Debug, thiserror::Error)]
#[error("fixture consumer failed")]
struct Failure;
impl Drop for Consumer {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl RenderInputConsumer for Consumer {
    type Settings = Arc<Probe>;
    type Error = Failure;
    fn prepare(_: &SceneInput, _: &str, settings: Arc<Probe>) -> Result<Self, Failure> {
        Ok(Self(settings))
    }
    fn accept(&mut self, _: &SceneInput, _: &str, _: UpdateKind) -> Result<(), Failure> {
        self.0.accepts.fetch_add(1, Ordering::SeqCst);
        match self.0.outcome.load(Ordering::SeqCst) {
            1 => Err(Failure),
            2 => panic!("fixture consumer unwound"),
            _ => Ok(()),
        }
    }
}
fn spatial(probe: Arc<Probe>) -> PlaySession<OfflineBackend, Consumer> {
    let mut inputs = prepared(MOVE);
    let mut spatial = CookedSpatial::empty(id(1));
    spatial.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    inputs.cooked = inputs.cooked.with_spatial(vec![], vec![spatial]).unwrap();
    let root = tempfile::tempdir().unwrap();
    inputs.render = Some(NativeRenderSettings {
        ui: None,
        text: None,
        model_cache: root.path().join("models"),
        texture_cache: root.path().join("textures"),
        camera: id(1),
        consumer: (),
    });
    PlaySession::new(inputs, settings(), probe).unwrap()
}

#[test]
fn paused_redraw_does_not_tick_and_stop_drops_the_retained_render_host() {
    let probe = Arc::new(Probe::default());
    let mut game = spatial(probe.clone());
    assert!(game.is_spatial());
    for _ in 0..3 {
        assert!(game.refresh().unwrap());
    }
    assert_eq!(probe.accepts.load(Ordering::SeqCst), 3);
    assert_eq!(game.world().unwrap().tick(), 0);
    assert_eq!(position(&game), 0.);
    game.step(&[]).unwrap();
    assert_eq!(probe.accepts.load(Ordering::SeqCst), 4);
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(position(&game), 2.);
    game.stop();
    assert_eq!(probe.drops.load(Ordering::SeqCst), 1);
    assert!(!game.is_spatial());
}

#[test]
fn presentation_errors_and_unwinds_destroy_native_state_even_during_pause() {
    for outcome in [1, 2] {
        for step in [false, true] {
            let probe = Arc::new(Probe::default());
            let mut game = spatial(probe.clone());
            probe.outcome.store(outcome, Ordering::SeqCst);
            let result = catch_unwind(AssertUnwindSafe(|| {
                if step {
                    game.step(&[])
                } else {
                    game.refresh().map(|_| ())
                }
            }));
            if outcome == 1 {
                assert!(result.unwrap().is_err());
            } else {
                assert!(result.is_err());
            }
            assert_eq!(game.mode(), PlayMode::Failed);
            assert!(game.world().is_err());
            assert!(game.input().is_err());
            assert!(game.logs().is_err());
            assert_eq!(probe.drops.load(Ordering::SeqCst), 1);
            assert!(game.step(&[]).is_err());
        }
    }
}
