#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
use incant_audio_runtime::{AudioError, AudioManagerSettings, ClipBinding};
use incant_game_runtime::{GameError, GameSession, OfflineGameSession};
use incant_input::{GamepadAxis, InputActions, InputEvent, KeyCode};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedPhysics, CookedScene, NativeAudioSource, StableId,
};
use incant_script::{CompiledScript, SessionError};
use incant_types::{AudioSpatial, BodyMotion, Collider, RigidBody, Transform, Velocity};
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
fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,dt,input}}){{{body}}}}};"
    ))
    .unwrap()
}
fn scene(rate: u32) -> CookedScene {
    CookedScene::new(
        id(99),
        rate,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        }],
    )
    .unwrap()
}
fn actions() -> InputActions {
    serde_json::from_value(
        serde_json::json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}),
    )
    .unwrap()
}
fn key(down: bool) -> InputEvent {
    InputEvent::Key {
        code: KeyCode::Space,
        down,
    }
}
fn invalid() -> InputEvent {
    InputEvent::GamepadAxis {
        id: 99,
        axis: GamepadAxis::LeftX,
        value: 0.5,
    }
}
fn next(session: &OfflineGameSession) -> Vec<f32> {
    vec![0.7; session.next_output_frames().unwrap() * 2]
}
fn failed(session: &mut OfflineGameSession) {
    assert!(session.is_failed());
    assert!(matches!(session.world(), Err(GameError::Failed)));
    assert!(matches!(session.input(), Err(GameError::Failed)));
    assert!(matches!(session.logs(), Err(GameError::Failed)));
    assert!(matches!(session.rendered_frames(), Err(GameError::Failed)));
    assert!(matches!(
        session.next_output_frames(),
        Err(GameError::Failed)
    ));
    assert!(matches!(session.rebind(actions()), Err(GameError::Failed)));
    assert!(matches!(session.poll_audio(), Err(GameError::Failed)));
    assert!(matches!(session.tick(&[], &mut []), Err(GameError::Failed)));
}
fn audio_scene(rate: u32, streaming: bool, physics: bool) -> CookedScene {
    let mut entities = scene(rate).entities().to_vec();
    entities[0].transform.translation[0] = -2.;
    entities.push(CookedEntity {
        id: id(2),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    });
    let scene = CookedScene::new(id(99), rate, entities).unwrap();
    let scene = if physics {
        scene
            .with_physics(vec![CookedPhysics {
                entity: id(1),
                collider: Collider::default(),
                body: Some(RigidBody {
                    motion: BodyMotion::Kinematic,
                    ..Default::default()
                }),
                angular_velocity: None,
            }])
            .unwrap()
    } else {
        scene
    };
    scene
        .with_audio(vec![
            CookedAudio {
                entity: id(1),
                bus: None,
                listener: false,
                source: Some(NativeAudioSource {
                    clip: id(90),
                    bus: None,
                    gain_db: 0.,
                    pan: 0.,
                    rate: 1.,
                    start_seconds: 0.,
                    playing: true,
                    looping: true,
                    streaming,
                    spatial: Some(AudioSpatial {
                        min_distance: 1.,
                        max_distance: 100.,
                    }),
                }),
            },
            CookedAudio {
                entity: id(2),
                bus: None,
                source: None,
                listener: true,
            },
        ])
        .unwrap()
}
fn clip(temp: &tempfile::TempDir) -> (String, ClipBinding) {
    let key = support::cooked(temp.path(), &[[0.2; 2]; 24000]);
    let binding = ClipBinding::load(id(90), temp.path(), &key).unwrap();
    (key, binding)
}

#[test]
fn rational_clock_covers_fractional_frames_without_drift_and_validates_buffer_before_input() {
    for (rate, sample_rate) in [(240, 8000), (59, 44100), (64, 48000), (1, 8000)] {
        let mut session = OfflineGameSession::new(
            scene(rate),
            code("chunks[0].transforms[1]+=dt;"),
            actions(),
            vec![],
            sample_rate,
        )
        .unwrap();
        for tick in 1..=67u64 {
            let expected = (u128::from(tick) * u128::from(sample_rate) / u128::from(rate)) as usize;
            let mut output = next(&session);
            if tick == 1 {
                let mut short = vec![0.7; output.len() - 1];
                assert!(matches!(
                    session.tick(&[key(true)], &mut short),
                    Err(GameError::Buffer { .. })
                ));
                assert!(short.iter().all(|v| *v == 0.7));
                assert_eq!(session.world().unwrap().tick(), 0);
                assert_eq!(session.rendered_frames().unwrap(), 0);
            }
            session.tick(&[], &mut output).unwrap();
            assert!(output.iter().all(|v| *v == 0.));
            assert_eq!(session.rendered_frames().unwrap(), expected as u128);
            let p = session
                .world()
                .unwrap()
                .inspect(id(1))
                .unwrap()
                .transform
                .translation;
            assert!((p[0] - tick as f64 / f64::from(rate)).abs() < 1e-10);
            assert!((p[1] - p[0]).abs() < 1e-10);
        }
    }
    for rate in [0, 1, 7999, 192001, u32::MAX] {
        assert!(matches!(
            OfflineGameSession::new(scene(60), code(""), actions(), vec![], rate),
            Err(GameError::SampleRate)
        ));
    }
}

#[test]
fn invalid_input_preserves_pending_edges_vm_world_clock_and_audio_then_retry_succeeds() {
    let mut session=OfflineGameSession::new(scene(60),code("globalThis.n=(globalThis.n||0)+1; chunks[0].transforms[2]=n; if(input.actions.jump.released) chunks[0].transforms[1]+=1;"),actions(),vec![],48000).unwrap();
    let mut output = next(&session);
    session.tick(&[key(true)], &mut output).unwrap();
    let before = session.world().unwrap().snapshot();
    let input = session.input().unwrap().clone();
    for _ in 0..8 {
        output.fill(0.7);
        assert!(matches!(
            session.tick(&[key(false), invalid()], &mut output),
            Err(GameError::Session(SessionError::Input(_)))
        ));
        assert_eq!(session.world().unwrap().snapshot(), before);
        assert_eq!(session.input().unwrap(), &input);
        assert_eq!(session.rendered_frames().unwrap(), 800);
        assert!(output.iter().all(|v| *v == 0.7));
        assert!(!session.is_failed());
    }
    session.tick(&[key(false)], &mut output).unwrap();
    assert_eq!(
        session
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation,
        [2. / 60., 1., 2.]
    );
    assert_eq!(session.rendered_frames().unwrap(), 1600);
}

#[test]
fn input_script_physics_and_audio_use_the_same_accepted_tick_and_no_double_integration() {
    for streaming in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (_, binding) = clip(&temp);
        let script = code(
            "for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001' && input.actions.jump.pressed) c.transforms[i*10]+=4;",
        );
        let mut session = OfflineGameSession::new(
            audio_scene(64, streaming, true),
            script,
            actions(),
            vec![binding],
            48000,
        )
        .unwrap();
        let mut output = next(&session);
        session.tick(&[key(true)], &mut output).unwrap();
        let p = session
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0];
        assert!((p - (2. + 1. / 64.)).abs() < 1e-10);
        let balance: f32 = output[128..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|f| f[0] * f[0] - f[1] * f[1])
            .sum();
        assert!(
            balance < -0.01,
            "audio missed script publication: {balance}"
        );
        session.tick(&[], &mut output).unwrap();
        assert!(
            (session
                .world()
                .unwrap()
                .physics_state(id(1))
                .unwrap()
                .translation[0]
                - (2. + 2. / 64.))
                .abs()
                < 1e-10
        );
    }
}

#[test]
fn accepted_script_or_runtime_failure_silences_and_hides_every_live_state() {
    let mut session = OfflineGameSession::new(
        scene(60),
        code("throw Error('accepted');"),
        actions(),
        vec![],
        48000,
    )
    .unwrap();
    let mut output = next(&session);
    assert!(matches!(
        session.tick(&[key(true)], &mut output),
        Err(GameError::Session(SessionError::Script(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut session);
    let mut entity = scene(1).entities()[0].clone();
    entity.transform.translation[0] = 1e308;
    entity.velocity.as_mut().unwrap().linear[0] = 1e308;
    let mut session = OfflineGameSession::new(
        CookedScene::new(id(99), 1, vec![entity]).unwrap(),
        code("throw Error('unreachable');"),
        actions(),
        vec![],
        8000,
    )
    .unwrap();
    let mut output = next(&session);
    assert!(matches!(
        session.tick(&[], &mut output),
        Err(GameError::Session(SessionError::Runtime(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut session);
}

#[test]
fn audio_preflight_after_script_is_terminal_even_though_standalone_host_can_retry() {
    let temp = tempfile::tempdir().unwrap();
    let (_, binding) = clip(&temp);
    let mut session=OfflineGameSession::new(audio_scene(60,false,false),code("for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001') c.transforms[i*10]=1000001;"),actions(),vec![binding],48000).unwrap();
    let mut output = next(&session);
    assert!(matches!(
        session.tick(&[key(true)], &mut output),
        Err(GameError::Audio(AudioError::Invalid(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut session);
}

#[test]
fn render_corruption_is_terminal_and_silences_the_entire_tick_buffer() {
    let temp = tempfile::tempdir().unwrap();
    let (_, binding) = clip(&temp);
    let mut session = OfflineGameSession::new(
        audio_scene(60, true, false),
        code(""),
        actions(),
        vec![binding],
        48000,
    )
    .unwrap();
    let mut output = next(&session);
    session.tick(&[], &mut output).unwrap();
    let pcm = fs::read_dir(temp.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.extension().unwrap() == "ipcm")
        .unwrap();
    let mut file = fs::OpenOptions::new().write(true).open(pcm).unwrap();
    file.seek(SeekFrom::Start(15000 * 8)).unwrap();
    file.write_all(&[255; 8]).unwrap();
    let mut observed = false;
    for _ in 0..30 {
        output.fill(0.7);
        match session.tick(&[], &mut output) {
            Ok(()) => {}
            Err(GameError::Audio(AudioError::Stream(_))) => {
                observed = true;
                assert!(output.iter().all(|v| *v == 0.));
                break;
            }
            Err(e) => panic!("{e}"),
        }
    }
    assert!(observed);
    failed(&mut session);
}

#[derive(Default)]
struct Counts {
    starts: AtomicUsize,
    drops: AtomicUsize,
}
struct Backend {
    counts: Arc<Counts>,
    renderer: Option<kira::backend::Renderer>,
}
impl Drop for Backend {
    fn drop(&mut self) {
        self.counts.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl kira::backend::Backend for Backend {
    type Settings = Arc<Counts>;
    type Error = &'static str;
    fn setup(counts: Self::Settings, _: usize) -> std::result::Result<(Self, u32), Self::Error> {
        counts.starts.fetch_add(1, Ordering::SeqCst);
        Ok((
            Self {
                counts,
                renderer: None,
            },
            48000,
        ))
    }
    fn start(&mut self, renderer: kira::backend::Renderer) -> std::result::Result<(), Self::Error> {
        self.renderer = Some(renderer);
        Ok(())
    }
}
#[test]
fn constructor_rejection_has_no_backend_effect_and_terminal_tick_drops_the_backend() {
    let counts = Arc::new(Counts::default());
    let settings = || AudioManagerSettings::<Backend> {
        backend_settings: counts.clone(),
        ..Default::default()
    };
    assert!(
        GameSession::new(
            audio_scene(60, false, false),
            code(""),
            actions(),
            vec![],
            settings()
        )
        .is_err()
    );
    assert_eq!(counts.starts.load(Ordering::SeqCst), 0);
    let mut session = GameSession::new(
        scene(60),
        code("throw Error('fail');"),
        actions(),
        vec![],
        settings(),
    )
    .unwrap();
    assert_eq!(counts.starts.load(Ordering::SeqCst), 1);
    assert!(session.tick(&[invalid()]).is_err());
    assert!(!session.is_failed());
    assert_eq!(counts.drops.load(Ordering::SeqCst), 0);
    assert!(session.tick(&[]).is_err());
    assert!(session.is_failed());
    assert_eq!(counts.drops.load(Ordering::SeqCst), 1);
    assert!(session.world().is_err() && session.input().is_err() && session.poll_audio().is_err());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn background_decoder_failure_between_ticks_destroys_the_whole_session() {
    for suspend in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (_, binding) = clip(&temp);
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
        let counts = Arc::new(Counts::default());
        let mut session = GameSession::new(
            audio_scene(60, true, false),
            code(""),
            actions(),
            vec![binding],
            AudioManagerSettings::<Backend> {
                backend_settings: counts.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(session.world().unwrap().tick(), 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match if suspend {
                session.set_suspended(true)
            } else {
                session.poll_audio()
            } {
                Err(GameError::Audio(AudioError::Stream(_))) => break,
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
        assert!(session.is_failed());
        assert_eq!(counts.drops.load(Ordering::SeqCst), 1);
        assert!(session.world().is_err() && session.input().is_err());
        assert!(session.source_state(id(1)).is_err() && session.rebind(actions()).is_err());
        assert!(matches!(session.tick(&[]), Err(GameError::Failed)));
    }
}

#[path = "support/spatial.rs"]
mod spatial;
#[test]
fn spatial_and_manifest_only_scenes_reject_before_vm_or_backend_setup() {
    let counts = Arc::new(Counts::default());
    for scene in spatial::scenes().into_iter().skip(1) {
        // Top-level guest execution would fail if the VM were constructed.
        let script = || CompiledScript::compile("throw Error('VM must not run');").unwrap();
        assert!(matches!(
            GameSession::new(
                scene.clone(),
                script(),
                actions(),
                vec![],
                AudioManagerSettings::<Backend> {
                    backend_settings: counts.clone(),
                    ..Default::default()
                }
            ),
            Err(GameError::SpatialHost)
        ));
        assert!(matches!(
            OfflineGameSession::new(scene, script(), actions(), vec![], 48000),
            Err(GameError::SpatialHost)
        ));
    }
    assert_eq!(counts.starts.load(Ordering::SeqCst), 0);
}

#[test]
fn suspension_rejects_ticks_without_changing_world_or_output_and_resumes() {
    let temp = tempfile::tempdir().unwrap();
    let (_, binding) = clip(&temp);
    let mut game = OfflineGameSession::new(
        audio_scene(60, false, false),
        code(""),
        actions(),
        vec![binding],
        48000,
    )
    .unwrap();
    let mut output = vec![1.; game.next_output_frames().unwrap() * 2];
    game.set_suspended(true).unwrap();
    game.set_suspended(true).unwrap();
    assert!(matches!(
        game.tick(&[], &mut output),
        Err(GameError::Suspended)
    ));
    assert!(!game.is_failed());
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(output.iter().all(|v| *v == 1.));
    game.set_suspended(false).unwrap();
    game.tick(&[], &mut output).unwrap();
    assert_eq!(game.world().unwrap().tick(), 1);
}
