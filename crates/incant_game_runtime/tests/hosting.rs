//! Observable lifetime/failure contract of the trusted native host boundary.
//! Actual retained render-input compatibility is tested by incant_render_game.
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
#[path = "support/spatial.rs"]
mod spatial;
use incant_audio_runtime::{AudioManagerSettings, ClipBinding};
use incant_game_runtime::{GameError, HostedGameSession, HostedOfflineGameSession, SceneHost};
use incant_input::{GamepadAxis, InputActions, InputEvent};
use incant_runtime::{CookedAudio, NativeAudioSource, NativeWorld, StableId, WorldIdentity};
use incant_script::{CompiledScript, PendingNativeSession, SessionError};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};

fn id() -> StableId {
    StableId(1u128.to_be_bytes())
}
fn script() -> CompiledScript {
    CompiledScript::compile(
        "exports.default={update({chunks,log}){chunks[0].transforms[0]+=2;log('accepted');}}",
    )
    .unwrap()
}
fn invalid() -> InputEvent {
    InputEvent::GamepadAxis {
        id: 99,
        axis: GamepadAxis::LeftX,
        value: 0.5,
    }
}
#[derive(Clone, Copy, Default)]
enum Outcome {
    #[default]
    Accept,
    Reject,
    Panic,
}
#[derive(Default)]
struct Probe {
    calls: Mutex<Vec<&'static str>>,
}
impl Probe {
    fn record(&self, call: &'static str) {
        self.calls.lock().unwrap().push(call);
    }
    fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}
#[derive(Clone, Default)]
struct HostSettings {
    probe: Arc<Probe>,
    prepare: Outcome,
    sync: Outcome,
}
#[derive(Debug, thiserror::Error)]
#[error("test host refused compatible binding")]
struct HostError;
struct Host {
    settings: HostSettings,
    identity: WorldIdentity,
    observed: Vec<(u64, f64)>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.settings.probe.record("host-drop");
    }
}
impl SceneHost for Host {
    type Settings = HostSettings;
    type Error = HostError;
    fn prepare(world: &NativeWorld, settings: HostSettings) -> Result<Self, HostError> {
        settings.probe.record("prepare");
        assert_eq!(world.tick(), 0);
        match settings.prepare {
            Outcome::Reject => return Err(HostError),
            Outcome::Panic => panic!("prepare unwound"),
            Outcome::Accept => {}
        }
        Ok(Self {
            settings,
            identity: world.spatial_view().unwrap().identity().clone(),
            observed: Vec::new(),
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), HostError> {
        assert_eq!(&self.identity, world.spatial_view().unwrap().identity());
        self.settings.probe.record("sync");
        self.observed.push((
            world.tick(),
            world.inspect(id()).unwrap().transform.translation[0],
        ));
        match self.settings.sync {
            Outcome::Reject => Err(HostError),
            Outcome::Panic => panic!("accepted host sync unwound"),
            Outcome::Accept => Ok(()),
        }
    }
}
#[derive(Default)]
struct BackendSettings {
    probe: Arc<Probe>,
    reject: bool,
}
struct Backend {
    probe: Arc<Probe>,
    renderer: Option<kira::backend::Renderer>,
}
impl Drop for Backend {
    fn drop(&mut self) {
        self.probe.record("backend-drop");
    }
}
impl kira::backend::Backend for Backend {
    type Settings = BackendSettings;
    type Error = &'static str;
    fn setup(settings: Self::Settings, _: usize) -> Result<(Self, u32), Self::Error> {
        settings.probe.record("backend-setup");
        if settings.reject {
            return Err("backend rejected setup");
        }
        Ok((
            Self {
                probe: settings.probe,
                renderer: None,
            },
            48000,
        ))
    }
    fn start(&mut self, renderer: kira::backend::Renderer) -> Result<(), Self::Error> {
        self.probe.record("backend-start");
        self.renderer = Some(renderer);
        Ok(())
    }
}
fn backend(probe: Arc<Probe>, reject: bool) -> AudioManagerSettings<Backend> {
    AudioManagerSettings {
        backend_settings: BackendSettings { probe, reject },
        ..Default::default()
    }
}
fn failed(session: &mut HostedOfflineGameSession<Host>) {
    assert!(session.is_failed());
    assert!(matches!(session.host(), Err(GameError::Failed)));
    assert!(matches!(session.world(), Err(GameError::Failed)));
    assert!(matches!(session.input(), Err(GameError::Failed)));
    assert!(matches!(session.logs(), Err(GameError::Failed)));
    assert!(matches!(session.source_state(id()), Err(GameError::Failed)));
    assert!(matches!(session.rendered_frames(), Err(GameError::Failed)));
    assert!(matches!(
        session.next_output_frames(),
        Err(GameError::Failed)
    ));
    assert!(matches!(
        session.rebind(InputActions::default()),
        Err(GameError::Failed)
    ));
    assert!(matches!(session.poll_audio(), Err(GameError::Failed)));
    assert!(matches!(session.tick(&[], &mut []), Err(GameError::Failed)));
}

#[test]
fn pending_start_preserves_the_exact_world_identity_without_guest_execution_during_prepare() {
    let pending =
        PendingNativeSession::new(spatial::scenes()[1].clone(), InputActions::default()).unwrap();
    let identity = pending.world().spatial_view().unwrap().identity().clone();
    let mut session = pending.start(script()).unwrap();
    assert_eq!(
        &identity,
        session.world().unwrap().spatial_view().unwrap().identity()
    );
    assert_eq!(session.world().unwrap().tick(), 0);
    session.tick(&[]).unwrap();
    assert_eq!(
        &identity,
        session.world().unwrap().spatial_view().unwrap().identity()
    );
    assert_eq!(
        session
            .world()
            .unwrap()
            .inspect(id())
            .unwrap()
            .transform
            .translation[0],
        2.
    );
}

#[test]
fn host_prepare_precedes_guest_and_backend_setup_and_failures_drop_prepared_state() {
    let throwing = || CompiledScript::compile("throw Error('top-level guest');").unwrap();
    for scene in spatial::scenes() {
        let probe = Arc::new(Probe::default());
        let result = HostedGameSession::<Backend, Host>::new(
            scene.clone(),
            throwing(),
            InputActions::default(),
            vec![],
            backend(probe.clone(), false),
            HostSettings {
                probe: probe.clone(),
                prepare: Outcome::Reject,
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(GameError::Host(HostError))));
        assert_eq!(probe.calls(), ["prepare"]);
        let probe = Arc::new(Probe::default());
        let result = HostedGameSession::<Backend, Host>::new(
            scene.clone(),
            throwing(),
            InputActions::default(),
            vec![],
            backend(probe.clone(), false),
            HostSettings {
                probe: probe.clone(),
                ..Default::default()
            },
        );
        assert!(matches!(
            result,
            Err(GameError::Session(SessionError::Script(_)))
        ));
        assert_eq!(probe.calls(), ["prepare", "host-drop"]);
        let probe = Arc::new(Probe::default());
        let result = HostedGameSession::<Backend, Host>::new(
            scene,
            script(),
            InputActions::default(),
            vec![],
            backend(probe.clone(), true),
            HostSettings {
                probe: probe.clone(),
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(GameError::Audio(_))));
        assert_eq!(probe.calls(), ["prepare", "backend-setup", "host-drop"]);
    }
}

#[test]
fn invalid_sample_rate_rejects_before_host_or_guest_setup() {
    let probe = Arc::new(Probe::default());
    assert!(matches!(
        HostedOfflineGameSession::<Host>::new(
            spatial::scenes()[1].clone(),
            CompiledScript::compile("throw Error('unreachable');").unwrap(),
            InputActions::default(),
            vec![],
            0,
            HostSettings {
                probe: probe.clone(),
                ..Default::default()
            },
        ),
        Err(GameError::SampleRate)
    ));
    assert!(probe.calls().is_empty());
}

#[test]
fn successful_ticks_sync_the_stored_host_once_and_retryable_rejections_leave_it_unchanged() {
    for scene in spatial::scenes() {
        let probe = Arc::new(Probe::default());
        let mut session = HostedOfflineGameSession::<Host>::new(
            scene,
            script(),
            InputActions::default(),
            vec![],
            44117,
            HostSettings {
                probe: probe.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(probe.calls(), ["prepare"]);
        for tick in 1..=3 {
            let mut output = vec![0.7; session.next_output_frames().unwrap() * 2];
            let len = output.len();
            assert!(matches!(
                session.tick(&[], &mut output[..len - 1]),
                Err(GameError::Buffer { .. })
            ));
            assert!(matches!(
                session.tick(&[invalid()], &mut output),
                Err(GameError::Session(SessionError::Input(_)))
            ));
            assert!(output.iter().all(|sample| *sample == 0.7));
            assert_eq!(session.world().unwrap().tick(), tick - 1);
            assert_eq!(session.host().unwrap().observed.len(), tick as usize - 1);
            assert_eq!(session.logs().unwrap().entries().len(), tick as usize - 1);
            session.tick(&[], &mut output).unwrap();
            assert_eq!(session.logs().unwrap().entries().len(), tick as usize);
            assert_eq!(
                session.host().unwrap().observed.last(),
                Some(&(tick, tick as f64 * 2.))
            );
            assert_eq!(
                session.rendered_frames().unwrap(),
                u128::from(tick) * 44117 / 60
            );
        }
        assert_eq!(probe.calls(), ["prepare", "sync", "sync", "sync"]);
        drop(session);
        assert_eq!(probe.calls().last(), Some(&"host-drop"));
    }
}

#[test]
fn host_error_and_unwind_terminalize_offline_session_and_silence_entire_output() {
    let temp = tempfile::tempdir().unwrap();
    let key = audio_support::cooked(temp.path(), &[[0.2; 2]; 1600]);
    let scene = spatial::scenes()[1]
        .clone()
        .with_audio(vec![CookedAudio {
            entity: id(),
            bus: None,
            listener: false,
            source: Some(NativeAudioSource {
                transport_epoch: 0,
                clip: id(),
                bus: None,
                gain_db: 0.,
                pan: 0.,
                rate: 1.,
                start_seconds: 0.,
                playing: true,
                looping: true,
                streaming: false,
                spatial: None,
            }),
        }])
        .unwrap();
    // The control proves this exact render produces audio before the late host
    // operation. A silent scene could not detect missing cleanup after rendering.
    for outcome in [Outcome::Accept, Outcome::Reject, Outcome::Panic] {
        let probe = Arc::new(Probe::default());
        let mut session = HostedOfflineGameSession::<Host>::new(
            scene.clone(),
            CompiledScript::compile("exports.default={update({chunks,log,commands}){chunks[0].transforms[0]+=2;log('accepted');commands.requestRestart();}}").unwrap(),
            InputActions::default(),
            vec![ClipBinding::load(id(), temp.path(), &key).unwrap()],
            48000,
            HostSettings {
                probe: probe.clone(),
                sync: outcome,
                ..Default::default()
            },
        )
        .unwrap();
        session.enable_restart_requests().unwrap();
        let mut output = vec![0.7; session.next_output_frames().unwrap() * 2];
        let result = catch_unwind(AssertUnwindSafe(|| session.tick(&[], &mut output)));
        match outcome {
            Outcome::Accept => {
                assert!(matches!(result, Ok(Ok(()))));
                assert!(output.iter().any(|sample| sample.abs() > 0.01));
                assert_eq!(session.host().unwrap().observed, [(1, 2.)]);
                assert_eq!(session.logs().unwrap().entries()[0].message, "accepted");
                assert_eq!(
                    session.take_session_request().unwrap(),
                    Some(incant_script::SessionRequest::Restart)
                );
                assert_eq!(session.take_session_request().unwrap(), None);
                continue;
            }
            Outcome::Reject => assert!(matches!(result, Ok(Err(GameError::Host(HostError))))),
            Outcome::Panic => assert!(result.is_err()),
        }
        assert!(output.iter().all(|sample| *sample == 0.));
        assert_eq!(probe.calls(), ["prepare", "sync", "host-drop"]);
        assert!(matches!(
            session.take_session_request(),
            Err(GameError::Failed)
        ));
        failed(&mut session);
    }
}

#[test]
fn accepted_script_failure_never_calls_host_and_also_drops_it() {
    let probe = Arc::new(Probe::default());
    let mut session = HostedOfflineGameSession::<Host>::new(
        spatial::scenes()[1].clone(),
        CompiledScript::compile("exports.default={update(){throw Error('accepted')}}").unwrap(),
        InputActions::default(),
        vec![],
        48000,
        HostSettings {
            probe: probe.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let mut output = vec![0.7; session.next_output_frames().unwrap() * 2];
    assert!(matches!(
        session.tick(&[], &mut output),
        Err(GameError::Session(SessionError::Script(_)))
    ));
    assert_eq!(probe.calls(), ["prepare", "host-drop"]);
    assert!(output.iter().all(|sample| *sample == 0.));
    failed(&mut session);
}

#[test]
fn backend_driven_host_error_or_unwind_drops_the_backend_and_hides_all_state() {
    for outcome in [Outcome::Reject, Outcome::Panic] {
        let probe = Arc::new(Probe::default());
        let mut session = HostedGameSession::<Backend, Host>::new(
            spatial::scenes()[1].clone(),
            script(),
            InputActions::default(),
            vec![],
            backend(probe.clone(), false),
            HostSettings {
                probe: probe.clone(),
                sync: outcome,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(session.tick(&[invalid()]).is_err());
        assert!(!session.is_failed());
        assert_eq!(probe.calls(), ["prepare", "backend-setup", "backend-start"]);
        let result = catch_unwind(AssertUnwindSafe(|| session.tick(&[])));
        match outcome {
            Outcome::Reject => assert!(matches!(result, Ok(Err(GameError::Host(HostError))))),
            Outcome::Panic => assert!(result.is_err()),
            _ => unreachable!(),
        }
        assert_eq!(
            probe.calls(),
            [
                "prepare",
                "backend-setup",
                "backend-start",
                "sync",
                "backend-drop",
                "host-drop"
            ]
        );
        assert!(session.is_failed());
        assert!(session.host().is_err() && session.world().is_err() && session.input().is_err());
        assert!(
            session.source_state(id()).is_err() && session.rebind(InputActions::default()).is_err()
        );
        assert!(session.poll_audio().is_err() && session.tick(&[]).is_err());
    }
}

#[test]
fn paused_refresh_preserves_tick_input_pose_and_exact_next_audio_samples() {
    let temp = tempfile::tempdir().unwrap();
    let samples: Vec<_> = (0..4800)
        .map(|n| {
            let v = (n as f32 * 0.071).sin() * 0.2;
            [v, -v]
        })
        .collect();
    let key = audio_support::cooked(temp.path(), &samples);
    let scene = spatial::scenes()[1]
        .clone()
        .with_audio(vec![CookedAudio {
            entity: id(),
            bus: None,
            listener: false,
            source: Some(NativeAudioSource {
                transport_epoch: 0,
                clip: id(),
                bus: None,
                gain_db: 0.,
                pan: 0.,
                rate: 1.,
                start_seconds: 0.,
                playing: true,
                looping: true,
                streaming: false,
                spatial: None,
            }),
        }])
        .unwrap();
    let create = || {
        HostedOfflineGameSession::<Host>::new(
            scene.clone(),
            script(),
            InputActions::default(),
            vec![ClipBinding::load(id(), temp.path(), &key).unwrap()],
            48000,
            HostSettings::default(),
        )
        .unwrap()
    };
    let (mut refreshed, mut control) = (create(), create());
    let event = InputEvent::Key {
        code: incant_input::KeyCode::Space,
        down: true,
    };
    let mut a = vec![0.; 1600];
    let mut b = a.clone();
    refreshed
        .tick(std::slice::from_ref(&event), &mut a)
        .unwrap();
    control.tick(&[event], &mut b).unwrap();
    assert_eq!(a, b);
    assert!(a.iter().any(|v| v.abs() > 0.01));
    refreshed.set_suspended(true).unwrap();
    control.set_suspended(true).unwrap();
    let world = refreshed.world().unwrap().snapshot();
    let input = refreshed.input().unwrap().clone();
    let frames = refreshed.rendered_frames().unwrap();
    let state = refreshed.source_state(id()).unwrap();
    for _ in 0..8 {
        refreshed.refresh_host().unwrap();
    }
    assert_eq!(refreshed.world().unwrap().tick(), 1);
    assert_eq!(refreshed.world().unwrap().snapshot(), world);
    assert_eq!(refreshed.input().unwrap(), &input);
    assert_eq!(refreshed.rendered_frames().unwrap(), frames);
    assert_eq!(refreshed.source_state(id()).unwrap(), state);
    assert_eq!(refreshed.host().unwrap().observed, vec![(1, 2.); 9]);
    assert!(matches!(
        refreshed.tick(&[], &mut a),
        Err(GameError::Suspended)
    ));
    refreshed.set_suspended(false).unwrap();
    control.set_suspended(false).unwrap();
    for _ in 0..3 {
        refreshed.tick(&[], &mut a).unwrap();
        control.tick(&[], &mut b).unwrap();
        assert_eq!(a, b);
    }
}

#[test]
fn presentation_only_error_and_unwind_drop_the_entire_backend_session() {
    for outcome in [Outcome::Reject, Outcome::Panic] {
        let probe = Arc::new(Probe::default());
        let mut session = HostedGameSession::<Backend, Host>::new(
            spatial::scenes()[1].clone(),
            script(),
            InputActions::default(),
            vec![],
            backend(probe.clone(), false),
            HostSettings {
                probe: probe.clone(),
                sync: outcome,
                ..Default::default()
            },
        )
        .unwrap();
        session.set_suspended(true).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| session.refresh_host()));
        match outcome {
            Outcome::Reject => assert!(matches!(result, Ok(Err(GameError::Host(HostError))))),
            Outcome::Panic => assert!(result.is_err()),
            _ => unreachable!(),
        }
        assert_eq!(
            probe.calls(),
            [
                "prepare",
                "backend-setup",
                "backend-start",
                "sync",
                "backend-drop",
                "host-drop"
            ]
        );
        assert!(session.is_failed());
        assert!(session.host().is_err() && session.world().is_err() && session.input().is_err());
        assert!(session.refresh_host().is_err() && session.set_suspended(false).is_err());
    }
}

#[test]
fn restart_drops_previous_host_before_new_prepare_and_failure_is_terminal() {
    let seed = incant_game_runtime::GameSeed::new(
        spatial::scenes()[1].clone(),
        script(),
        Default::default(),
        vec![],
    )
    .unwrap();
    for outcome in [Outcome::Accept, Outcome::Reject, Outcome::Panic] {
        let probe = Arc::new(Probe::default());
        let mut game = HostedOfflineGameSession::<Host>::from_seed(
            &seed,
            48000,
            HostSettings {
                probe: probe.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let old = game.host().unwrap().identity.clone();
        game.tick(&[], &mut vec![0.; game.next_output_frames().unwrap() * 2])
            .unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            game.restart_from(
                &seed,
                HostSettings {
                    probe: probe.clone(),
                    prepare: outcome,
                    ..Default::default()
                },
            )
        }));
        assert_eq!(
            probe.calls()[..4],
            ["prepare", "sync", "host-drop", "prepare"]
        );
        match outcome {
            Outcome::Accept => {
                result.unwrap().unwrap();
                assert_ne!(game.host().unwrap().identity, old);
                assert_eq!(game.world().unwrap().tick(), 0);
                assert!(game.host().unwrap().observed.is_empty());
            }
            Outcome::Reject => {
                assert!(matches!(result.unwrap(), Err(GameError::Host(_))));
                failed(&mut game);
            }
            Outcome::Panic => {
                assert!(result.is_err());
                failed(&mut game);
            }
        }
    }
}

#[test]
fn haptics_publish_only_after_host_acceptance_and_suspend_clears_ephemeral_authority() {
    use incant_input::haptics::{HapticCapabilities, HapticError};
    let caps = HapticCapabilities::default().with_gamepad(0, 3).unwrap();
    let script = || {
        CompiledScript::compile("exports.default={update({commands}){const targets=commands.hapticCapabilities();if(targets.length)commands.playHaptic(targets[0],{kind:'rumble',strong:0.5,weak:1,durationMs:125})}}").unwrap()
    };
    for outcome in [Outcome::Accept, Outcome::Reject, Outcome::Panic] {
        let mut session = HostedOfflineGameSession::<Host>::new(
            spatial::scenes()[1].clone(),
            script(),
            InputActions::default(),
            vec![],
            44117,
            HostSettings {
                sync: outcome,
                ..Default::default()
            },
        )
        .unwrap();
        session.set_haptic_capabilities(caps).unwrap();
        let mut output = vec![0.; session.next_output_frames().unwrap() * 2];
        let result = catch_unwind(AssertUnwindSafe(|| session.tick(&[], &mut output)));
        if matches!(outcome, Outcome::Accept) {
            result.unwrap().unwrap();
            let tick = session.world().unwrap().tick();
            assert!(matches!(
                session.tick(&[], &mut output),
                Err(GameError::Session(SessionError::Script(
                    incant_script::ScriptError::Haptics(HapticError::Pending)
                )))
            ));
            assert_eq!(session.world().unwrap().tick(), tick);
            assert_eq!(session.take_haptics().unwrap().len(), 1);
            session.tick(&[], &mut output).unwrap();
            session.set_suspended(true).unwrap();
            assert!(session.take_haptics().unwrap().is_empty());
            session.set_suspended(false).unwrap();
            let mut output = vec![0.; session.next_output_frames().unwrap() * 2];
            session.tick(&[], &mut output).unwrap();
            assert!(session.take_haptics().unwrap().is_empty());
        } else {
            assert!(!matches!(result, Ok(Ok(()))));
            assert!(matches!(session.take_haptics(), Err(GameError::Failed)));
        }
    }
}
