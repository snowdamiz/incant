use incant_audio_runtime::AudioManagerSettings;
use incant_game_runtime::{
    GameError, GameSession, HostedGameSession, HostedOfflineGameSession, OfflineGameSession,
    SceneHost,
};
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId, WorldIdentity};
use incant_script::CompiledScript;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        144,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn code() -> CompiledScript {
    CompiledScript::compile(
        r#"exports.default={persistence:{version:1},update(t){
 const n=Number(t.state.get()||0)+1;t.state.set(n);
 if(t.clock.tick===1)t.timers.set('periodic',3,3);
 t.chunks[0].transforms[0]=n;
}}"#,
    )
    .unwrap()
}
fn tick(game: &mut OfflineGameSession) -> Vec<f32> {
    let mut pcm = vec![7.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    pcm
}
#[test]
fn offline_resume_preserves_fractional_clock_state_and_neutralizes_input() {
    let initial = scene();
    let code = code();
    let mut baseline = OfflineGameSession::new(
        initial.clone(),
        code.clone(),
        Default::default(),
        vec![],
        44100,
    )
    .unwrap();
    for _ in 0..12 {
        tick(&mut baseline);
    }
    let mut pcm = vec![7.; baseline.next_output_frames().unwrap() * 2];
    baseline
        .tick(
            &[InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            }],
            &mut pcm,
        )
        .unwrap();
    let saved = baseline.checkpoint_bytes().unwrap();
    let before = baseline.rendered_frames().unwrap();
    let mut resumed = OfflineGameSession::from_checkpoint_bytes(
        &saved,
        &initial,
        code,
        Default::default(),
        vec![],
        44100,
    )
    .unwrap();
    assert_eq!(resumed.world().unwrap().tick(), 13);
    assert!(resumed.input().unwrap().keyboard.held.is_empty());
    assert!(resumed.input().unwrap().keyboard.released.is_empty());
    let mut frames = 0;
    for _ in 0..27 {
        let left = tick(&mut baseline);
        let right = tick(&mut resumed);
        assert_eq!(left, right);
        assert!(right.iter().all(|v| *v == 0.));
        frames += right.len() / 2;
        assert_eq!(
            baseline.checkpoint_bytes().unwrap(),
            resumed.checkpoint_bytes().unwrap()
        );
    }
    assert_eq!(frames, 8269);
    assert_eq!(resumed.rendered_frames().unwrap() - before, 8269);
}
#[derive(Clone, Copy, Default)]
enum Fail {
    #[default]
    No,
    Error,
    Panic,
}
#[derive(Default)]
struct Settings {
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail: Fail,
}
#[derive(Debug, thiserror::Error)]
#[error("test host failure")]
struct HostError;
struct Host {
    calls: Arc<Mutex<Vec<&'static str>>>,
    identity: WorldIdentity,
    prepared_tick: u64,
}
impl SceneHost for Host {
    type Settings = Settings;
    type Error = HostError;
    fn prepare(world: &NativeWorld, s: Settings) -> Result<Self, HostError> {
        s.calls.lock().unwrap().push("host-prepare");
        match s.fail {
            Fail::Error => return Err(HostError),
            Fail::Panic => panic!("host prepare"),
            Fail::No => {}
        }
        Ok(Self {
            calls: s.calls,
            identity: world.spatial_view().unwrap().identity().clone(),
            prepared_tick: world.tick(),
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), HostError> {
        assert_eq!(&self.identity, world.spatial_view().unwrap().identity());
        self.calls.lock().unwrap().push("host-sync");
        Ok(())
    }
}
struct Backend;
impl kira::backend::Backend for Backend {
    type Settings = Settings;
    type Error = &'static str;
    fn setup(s: Settings, _: usize) -> Result<(Self, u32), Self::Error> {
        s.calls.lock().unwrap().push("backend-setup");
        match s.fail {
            Fail::Error => return Err("backend setup"),
            Fail::Panic => panic!("backend setup"),
            Fail::No => {}
        }
        Ok((Self, 44100))
    }
    fn start(&mut self, _: kira::backend::Renderer) -> Result<(), Self::Error> {
        Ok(())
    }
}
fn settings(calls: &Arc<Mutex<Vec<&'static str>>>, fail: Fail) -> Settings {
    Settings {
        calls: Arc::clone(calls),
        fail,
    }
}
#[test]
fn restored_host_observes_the_exact_nonzero_world_before_backend_and_keeps_identity() {
    let initial = scene();
    let code = code();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut original = OfflineGameSession::new(
        initial.clone(),
        code.clone(),
        Default::default(),
        vec![],
        44100,
    )
    .unwrap();
    for _ in 0..5 {
        tick(&mut original);
    }
    let saved = original.checkpoint_bytes().unwrap();
    let mut game = HostedGameSession::<Backend, Host>::from_checkpoint_bytes(
        &saved,
        &initial,
        code,
        Default::default(),
        vec![],
        AudioManagerSettings {
            backend_settings: settings(&calls, Fail::No),
            ..Default::default()
        },
        settings(&calls, Fail::No),
    )
    .unwrap();
    assert_eq!(*calls.lock().unwrap(), ["host-prepare", "backend-setup"]);
    assert_eq!(game.host().unwrap().prepared_tick, 5);
    assert_ne!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        original.world().unwrap().spatial_view().unwrap().identity()
    );
    let before = game.checkpoint_bytes().unwrap();
    assert_eq!(before, saved);
    assert_eq!(game.world().unwrap().tick(), 5);
    game.tick(&[]).unwrap();
    assert_eq!(game.world().unwrap().tick(), 6);
    assert_eq!(calls.lock().unwrap().last(), Some(&"host-sync"));
}
#[test]
fn failed_candidate_setup_never_replaces_or_advances_the_previous_game() {
    let initial = scene();
    let code = code();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut previous = OfflineGameSession::new(
        initial.clone(),
        code.clone(),
        Default::default(),
        vec![],
        44100,
    )
    .unwrap();
    tick(&mut previous);
    let saved = previous.checkpoint_bytes().unwrap();
    for (host, backend) in [
        (Fail::Error, Fail::No),
        (Fail::Panic, Fail::No),
        (Fail::No, Fail::Error),
        (Fail::No, Fail::Panic),
    ] {
        calls.lock().unwrap().clear();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            HostedGameSession::<Backend, Host>::from_checkpoint_bytes(
                &saved,
                &initial,
                code.clone(),
                Default::default(),
                vec![],
                AudioManagerSettings {
                    backend_settings: settings(&calls, backend),
                    ..Default::default()
                },
                settings(&calls, host),
            )
        }));
        assert!(outcome.is_err() || outcome.unwrap().is_err());
        assert_eq!(previous.checkpoint_bytes().unwrap(), saved);
        if !matches!(host, Fail::No) {
            assert_eq!(*calls.lock().unwrap(), ["host-prepare"]);
        }
    }
    let mut bad = saved.clone();
    bad[0] ^= 1;
    calls.lock().unwrap().clear();
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
            &bad,
            &initial,
            code.clone(),
            Default::default(),
            vec![],
            44100,
            settings(&calls, Fail::No)
        )
        .is_err()
    );
    assert!(calls.lock().unwrap().is_empty());
    let wrong = CookedScene::new(id(98), 144, initial.entities().to_vec()).unwrap();
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
            &saved,
            &wrong,
            code,
            Default::default(),
            vec![],
            44100,
            settings(&calls, Fail::No)
        )
        .is_err()
    );
    assert!(calls.lock().unwrap().is_empty());
    tick(&mut previous);
    assert_eq!(previous.world().unwrap().tick(), 2);
}
#[test]
fn plain_and_hosted_factories_preserve_explicit_unsupported_world_guards() {
    let initial = scene();
    let code = code();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let plain = GameSession::<Backend>::new(
        initial.clone(),
        code.clone(),
        Default::default(),
        vec![],
        AudioManagerSettings {
            backend_settings: settings(&calls, Fail::No),
            ..Default::default()
        },
    )
    .unwrap();
    let save = plain.checkpoint_bytes().unwrap();
    GameSession::<Backend>::from_checkpoint_bytes(
        &save,
        &initial,
        code.clone(),
        Default::default(),
        vec![],
        AudioManagerSettings {
            backend_settings: settings(&calls, Fail::No),
            ..Default::default()
        },
    )
    .unwrap();
    let physical = initial
        .with_physics(vec![incant_runtime::CookedPhysics {
            entity: id(1),
            body: Some(Default::default()),
            collider: Default::default(),
            angular_velocity: None,
        }])
        .unwrap();
    assert!(matches!(
        OfflineGameSession::new(
            physical.clone(),
            code.clone(),
            Default::default(),
            vec![],
            44100
        )
        .unwrap()
        .checkpoint_bytes(),
        Err(GameError::Checkpoint(_))
    ));
    calls.lock().unwrap().clear();
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
            &save,
            &physical,
            code,
            Default::default(),
            vec![],
            44100,
            settings(&calls, Fail::No)
        )
        .is_err()
    );
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn audio_and_newly_added_spatial_capabilities_are_not_silently_dropped() {
    use incant_runtime::{CookedAudio, CookedSpatial, StructuralCommand};
    let initial = scene();
    let code = code();
    let audible = initial
        .clone()
        .with_audio(vec![CookedAudio {
            entity: id(1),
            bus: None,
            listener: true,
            source: None,
        }])
        .unwrap();
    let audio =
        OfflineGameSession::new(audible, code.clone(), Default::default(), vec![], 44100).unwrap();
    assert!(matches!(
        audio.checkpoint_bytes(),
        Err(GameError::Checkpoint(_))
    ));
    // Origin stays the trusted initial scene, while a committed command adds a
    // spatial capability. Plain restore must inspect the restored declarations.
    let native =
        incant_script::NativeSession::new(initial.clone(), code.clone(), Default::default())
            .unwrap();
    let mut world = NativeWorld::new(initial.clone());
    let mut camera = CookedSpatial::empty(id(1));
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: incant_types::CameraProjection::Perspective {},
    });
    let mut commands = incant_runtime::FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(camera),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    let mut saved = native.checkpoint_bytes().unwrap();
    let old_len = u32::from_le_bytes(saved[80..84].try_into().unwrap()) as usize;
    let replacement = world.save_bytes().unwrap();
    saved.splice(128..128 + old_len, replacement.iter().copied());
    saved[80..84].copy_from_slice(&(replacement.len() as u32).to_le_bytes());
    let mut content = saved[..96].to_vec();
    content.extend_from_slice(&saved[128..]);
    let checksum = incant_assets_runtime::sha256(&content);
    for i in 0..32 {
        saved[96 + i] = u8::from_str_radix(&checksum[i * 2..i * 2 + 2], 16).unwrap();
    }
    assert!(matches!(
        OfflineGameSession::from_checkpoint_bytes(
            &saved,
            &initial,
            code.clone(),
            Default::default(),
            vec![],
            44100
        ),
        Err(GameError::SpatialHost)
    ));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hosted = HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
        &saved,
        &initial,
        code,
        Default::default(),
        vec![],
        44100,
        settings(&calls, Fail::No),
    )
    .unwrap();
    assert!(
        hosted
            .world()
            .unwrap()
            .spatial_view()
            .unwrap()
            .nodes()
            .any(|node| node.declaration.is_some())
    );
    assert_eq!(*calls.lock().unwrap(), ["host-prepare"]);
}
