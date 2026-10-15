#![cfg(feature = "static-conformance")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
use incant_game_runtime::{GameSeed, HostedOfflineGameSession, OfflineGameSession, SceneHost};
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, NativeSession, SessionProgram, SessionRequest};
use incant_types::Transform;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};

fn scene() -> CookedScene {
    CookedScene::new(
        StableId([0; 16]),
        64,
        vec![CookedEntity {
            id: StableId(1u128.to_be_bytes()),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn program(development: bool, name: &str) -> SessionProgram {
    if development {
        SessionProgram::DevelopmentWasm(guest(name))
    } else {
        SessionProgram::Static(embedded::linked(name).unwrap())
    }
}
#[test]
fn real_rust_restart_requests_match_across_both_native_session_hosts() {
    for development in [false, true] {
        let program = program(development, "rust-session-restart");
        let mut session = NativeSession::new(scene(), program, Default::default()).unwrap();
        session.enable_restart_requests().unwrap();
        for frame in 1..=3 {
            session.tick(&[]).unwrap();
            assert_eq!(
                session.world().unwrap().snapshot()[0].transform.translation[0],
                f64::from(frame)
            );
            assert_eq!(
                session.take_session_request().unwrap(),
                Some(SessionRequest::Restart)
            );
            assert_eq!(session.take_session_request().unwrap(), None);
        }
    }
}

#[test]
fn request_once_remains_pending_until_consumed_in_all_three_session_hosts() {
    let bytecode = CompiledScript::compile(
        "exports.default={update(t){t.chunks[0].transforms[0]++;if(t.clock.tick===1)t.commands.requestRestart();}}",
    ).unwrap();
    for program in [
        SessionProgram::Bytecode(bytecode),
        program(false, "rust-session-once"),
        program(true, "rust-session-once"),
    ] {
        let mut session = NativeSession::new(scene(), program, Default::default()).unwrap();
        session.enable_restart_requests().unwrap();
        session.tick(&[]).unwrap();
        session.tick(&[]).unwrap();
        assert_eq!(
            session.take_session_request().unwrap(),
            Some(SessionRequest::Restart)
        );
        assert_eq!(session.take_session_request().unwrap(), None);
        session.tick(&[]).unwrap();
        assert_eq!(session.take_session_request().unwrap(), None);
    }
}

#[test]
fn actual_wasm_cold_restart_recreates_world_vm_input_and_output_clock() {
    for development in [false, true] {
        let seed = GameSeed::new(
            scene(),
            program(development, "rust-session-once"),
            Default::default(),
            vec![],
        )
        .unwrap();
        let mut game = OfflineGameSession::from_seed(&seed, 44117).unwrap();
        let initial_identity = game
            .world()
            .unwrap()
            .spatial_view()
            .unwrap()
            .identity()
            .clone();
        let mut first_counts = vec![];
        for _ in 0..2 {
            let count = game.next_output_frames().unwrap();
            first_counts.push(count);
            let mut pcm = vec![9.; count * 2];
            game.tick(
                &[InputEvent::Key {
                    code: KeyCode::KeyD,
                    down: true,
                }],
                &mut pcm,
            )
            .unwrap();
            assert!(pcm.iter().all(|&value| value == 0.));
        }
        assert_eq!(
            game.take_session_request().unwrap(),
            Some(SessionRequest::Restart)
        );
        assert!(game.input().unwrap().keyboard.held.contains(&KeyCode::KeyD));
        game.restart_from(&seed).unwrap();
        assert_ne!(
            game.world().unwrap().spatial_view().unwrap().identity(),
            &initial_identity
        );
        assert_eq!(game.world().unwrap().tick(), 0);
        assert_eq!(
            game.world().unwrap().snapshot()[0].transform.translation[0],
            0.
        );
        assert!(!game.input().unwrap().keyboard.held.contains(&KeyCode::KeyD));
        assert_eq!(game.take_session_request().unwrap(), None);
        for (index, count) in first_counts.into_iter().enumerate() {
            assert_eq!(game.next_output_frames().unwrap(), count);
            let mut pcm = vec![9.; count * 2];
            game.tick(&[], &mut pcm).unwrap();
            // The actual guest also checks its fresh linear-memory call counter.
            assert_eq!(
                game.world().unwrap().snapshot()[0].transform.translation[0],
                (index + 1) as f64
            );
            assert!(pcm.iter().all(|&value| value == 0.));
        }
        assert_eq!(
            game.take_session_request().unwrap(),
            Some(SessionRequest::Restart)
        );
        assert_eq!(game.take_session_request().unwrap(), None);
    }
}

struct ObserveAfterScript {
    observed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    reject: bool,
}
impl SceneHost for ObserveAfterScript {
    type Settings = (std::sync::Arc<std::sync::atomic::AtomicBool>, bool);
    type Error = std::io::Error;
    fn prepare(_: &NativeWorld, (observed, reject): Self::Settings) -> Result<Self, Self::Error> {
        Ok(Self { observed, reject })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        assert_eq!(world.snapshot()[0].transform.translation[0], 1.);
        self.observed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if self.reject {
            Err(std::io::Error::other(
                "test consumer refused accepted script state",
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn composed_consumer_refusal_hides_wasm_request_and_clears_entire_pcm() {
    let root = tempfile::tempdir().unwrap();
    let key = audio_support::cooked(root.path(), &vec![[0.25, -0.125]; 4096]);
    let clip_id = StableId(90u128.to_be_bytes());
    let clip = incant_audio_runtime::ClipBinding::load(clip_id, root.path(), &key).unwrap();
    let audible = scene()
        .with_audio(vec![incant_runtime::CookedAudio {
            entity: StableId(1u128.to_be_bytes()),
            bus: None,
            listener: false,
            source: Some(incant_runtime::NativeAudioSource {
                transport_epoch: 0,
                clip: clip_id,
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
    for development in [false, true] {
        let seed = GameSeed::new(
            audible.clone(),
            program(development, "rust-session-once"),
            Default::default(),
            vec![clip.clone()],
        )
        .unwrap();
        for reject in [false, true] {
            let observed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut game = HostedOfflineGameSession::<ObserveAfterScript>::from_seed(
                &seed,
                44117,
                (observed.clone(), reject),
            )
            .unwrap();
            let mut pcm = vec![9.; game.next_output_frames().unwrap() * 2];
            let result = game.tick(&[], &mut pcm);
            assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
            if reject {
                assert!(result.is_err());
                assert!(game.is_failed());
                assert!(game.world().is_err());
                assert!(game.take_session_request().is_err());
                assert!(pcm.iter().all(|&value| value == 0.));
            } else {
                result.unwrap();
                assert!(pcm.iter().any(|&value| value.abs() > 0.01));
                assert_eq!(
                    game.take_session_request().unwrap(),
                    Some(SessionRequest::Restart)
                );
            }
        }
    }
}
#[test]
fn real_rust_request_refusals_never_commit_numeric_prefix_on_either_backend() {
    for suffix in ["restart", "quota", "owner", "callback", "trap"] {
        let name = format!("rust-session-{suffix}");
        let mut a = NativeWorld::new(scene());
        let mut b = NativeWorld::new(scene());
        let before = a.save_bytes().unwrap();
        let mut compiled = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        if suffix != "restart" {
            compiled.enable_restart_requests().unwrap();
            dev.enable_restart_requests().unwrap();
        }
        assert!(compiled.tick(&mut a, 0.25).is_err());
        assert!(dev.tick(&mut b, 0.25).is_err());
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(compiled.take_restart_request().is_err());
        assert!(dev.take_restart_request().is_err());
    }
}
