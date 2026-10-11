#![cfg(feature = "fixture")]
use incant_game_runtime::{GameError, HostedOfflineGameSession, OfflineGameSession, SceneHost};
use incant_input::{GamepadAxis, InputEvent, KeyCode};
use incant_runtime::NativeWorld;
use incant_script::{NativeSession, PendingNativeSession, SessionError};
use incant_wasm_player::{
    SAMPLE_RATE, actions, id, load_clip, recording, replay, scene, write_fixture,
};
use incant_wasm_runtime::embedded;
use std::{
    convert::Infallible,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
};
fn module(name: &str) -> incant_wasm_runtime::ModuleExports {
    embedded::linked(name).unwrap()
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
fn buffer(game: &OfflineGameSession) -> Vec<f32> {
    vec![0.7; game.next_output_frames().unwrap() * 2]
}
fn failed(game: &mut OfflineGameSession) {
    assert!(game.is_failed());
    assert!(matches!(game.world(), Err(GameError::Failed)));
    assert!(matches!(game.input(), Err(GameError::Failed)));
    assert!(matches!(game.rendered_frames(), Err(GameError::Failed)));
    assert!(matches!(game.next_output_frames(), Err(GameError::Failed)));
    assert!(matches!(game.source_state(id(1)), Err(GameError::Failed)));
    assert!(matches!(game.rebind(actions()), Err(GameError::Failed)));
    assert!(matches!(game.poll_audio(), Err(GameError::Failed)));
    assert!(matches!(game.tick(&[], &mut []), Err(GameError::Failed)));
}
#[test]
fn pending_static_start_keeps_world_identity_and_retries_do_not_execute_the_module() {
    let pending = PendingNativeSession::new(scene(false), actions()).unwrap();
    let identity = pending.world().spatial_view().unwrap().identity().clone();
    let mut game = pending.start(module("game")).unwrap();
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    for _ in 0..3 {
        assert!(matches!(
            game.tick(&[key(true), invalid()]),
            Err(SessionError::Input(_))
        ));
    }
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(game.world().unwrap().inspect(id(3)).is_none());
    game.tick(&[key(true)]).unwrap();
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[1],
        1. / 64.
    );
    assert!(game.input().unwrap().actions["jump"].pressed);
    assert!(game.world().unwrap().inspect(id(3)).is_some());
}
#[test]
fn composed_static_motion_physics_audio_and_replay_share_one_tick_after_cache_deletion() {
    for streaming in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        let key = write_fixture(&cache, streaming).unwrap();
        let mut game = OfflineGameSession::new(
            scene(streaming),
            module("game"),
            actions(),
            vec![load_clip(&cache, &key).unwrap()],
            SAMPLE_RATE,
        )
        .unwrap();
        fs::remove_dir_all(&cache).unwrap();
        let replay = replay(&recording(), &scene(streaming)).unwrap();
        let mut edges = (0, 0);
        for tick in 1..=64 {
            let mut output = buffer(&game);
            game.tick(replay.events_at(tick).unwrap(), &mut output)
                .unwrap();
            let frame = &game.input().unwrap().actions["jump"];
            edges.0 += u32::from(frame.pressed);
            edges.1 += u32::from(frame.released);
            let position = game
                .world()
                .unwrap()
                .inspect(id(1))
                .unwrap()
                .transform
                .translation;
            assert!(
                (position[0] - (2. + tick as f64 / 64.)).abs() < 2e-6,
                "{position:?}"
            );
            assert_eq!(position[1], tick as f64 / 64.);
            let spawned = game.world().unwrap().inspect(id(3)).unwrap();
            assert_eq!(spawned.transform.translation[0], (tick - 1) as f64 / 32.);
            assert_eq!(
                game.rendered_frames().unwrap(),
                u128::from(tick) * u128::from(SAMPLE_RATE) / 64
            );
            assert!(output.iter().all(|v| v.is_finite()));
            if tick > 4 {
                let energy = output
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .fold([0.; 2], |mut sum, sample| {
                        sum[0] += sample[0].abs();
                        sum[1] += sample[1].abs();
                        sum
                    });
                assert!(energy[1] > energy[0] && energy[1] > 0., "{energy:?}");
            }
        }
        assert_eq!(edges, (1, 1));
    }
}
#[test]
fn rejected_buffer_and_input_preserve_module_counter_world_audio_and_edges() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    let key = write_fixture(&cache, false).unwrap();
    let mut game = OfflineGameSession::new(
        scene(false),
        module("game"),
        actions(),
        vec![load_clip(&cache, &key).unwrap()],
        SAMPLE_RATE,
    )
    .unwrap();
    let mut output = buffer(&game);
    game.tick(&[key_event(true)], &mut output).unwrap();
    let before = game.world().unwrap().snapshot();
    let input = game.input().unwrap().clone();
    let frames = game.rendered_frames().unwrap();
    for _ in 0..3 {
        let mut short = vec![0.7; game.next_output_frames().unwrap() * 2 - 1];
        assert!(matches!(
            game.tick(&[key_event(false)], &mut short),
            Err(GameError::Buffer { .. })
        ));
        assert!(short.iter().all(|v| *v == 0.7));
        let mut output = buffer(&game);
        assert!(matches!(
            game.tick(&[key_event(false), invalid()], &mut output),
            Err(GameError::Session(SessionError::Input(_)))
        ));
        assert!(output.iter().all(|v| *v == 0.7));
        assert_eq!(game.world().unwrap().snapshot(), before);
        assert_eq!(game.input().unwrap(), &input);
        assert_eq!(game.rendered_frames().unwrap(), frames);
    }
    let mut output = buffer(&game);
    game.tick(&[key_event(false)], &mut output).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[1],
        2. / 64.
    );
    assert!(game.input().unwrap().actions["jump"].released);
}
fn key_event(down: bool) -> InputEvent {
    key(down)
}
#[test]
fn accepted_module_trap_is_terminal_and_silences_offline_output() {
    let mut game = OfflineGameSession::new(
        incant_runtime::CookedScene::new(id(99), 64, vec![]).unwrap(),
        module("trap"),
        actions(),
        vec![],
        SAMPLE_RATE,
    )
    .unwrap();
    let mut output = buffer(&game);
    assert!(matches!(
        game.tick(&[key(true)], &mut output),
        Err(GameError::Session(SessionError::Module(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut game);
    let mut native = NativeSession::new(scene(false), module("trap"), actions()).unwrap();
    assert!(native.tick(&[key(true)]).is_err());
    assert!(native.is_failed());
    assert!(matches!(native.world(), Err(SessionError::Failed)));
    assert!(matches!(native.input(), Err(SessionError::Failed)));
}
#[test]
fn camera_and_manifest_only_scenes_refuse_before_a_trapping_static_start() {
    for spatial in incant_wasm_player::spatial_scenes() {
        assert!(matches!(
            OfflineGameSession::new(
                spatial,
                module("start-loop"),
                actions(),
                vec![],
                SAMPLE_RATE
            ),
            Err(GameError::SpatialHost)
        ));
    }
}
struct PanickingHost;
impl SceneHost for PanickingHost {
    type Settings = ();
    type Error = Infallible;
    fn prepare(_: &NativeWorld, _: ()) -> Result<Self, Infallible> {
        Ok(Self)
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Infallible> {
        assert!(world.inspect(id(3)).is_some());
        panic!("host after real module publication")
    }
}
#[test]
fn accepted_host_unwind_drops_static_session_and_silences_output() {
    let mut game = HostedOfflineGameSession::<PanickingHost>::new(
        incant_runtime::CookedScene::new(id(99), 64, vec![]).unwrap(),
        module("game"),
        actions(),
        vec![],
        SAMPLE_RATE,
        (),
    )
    .unwrap();
    let mut output = vec![0.7; game.next_output_frames().unwrap() * 2];
    assert!(catch_unwind(AssertUnwindSafe(|| game.tick(&[], &mut output))).is_err());
    assert!(output.iter().all(|v| *v == 0.));
    assert!(game.is_failed());
    assert!(matches!(game.world(), Err(GameError::Failed)));
    assert!(matches!(game.host(), Err(GameError::Failed)));
}

#[test]
fn accepted_native_failure_precedes_guest_execution_and_is_terminal() {
    let mut game = OfflineGameSession::new(
        incant_wasm_player::overflow_scene(),
        module("loop"),
        actions(),
        vec![],
        SAMPLE_RATE,
    )
    .unwrap();
    let mut output = buffer(&game);
    assert!(matches!(
        game.tick(&[key(true)], &mut output),
        Err(GameError::Session(SessionError::Runtime(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut game);
}
#[test]
fn accepted_numeric_and_structural_guest_rejections_are_terminal() {
    for name in ["nan", "cycle", "remove-listener"] {
        let mut game = OfflineGameSession::new(
            scene(false),
            module(name),
            actions(),
            {
                let temp = tempfile::tempdir().unwrap();
                let cache = temp.path().join("cache");
                let fingerprint = write_fixture(&cache, false).unwrap();
                vec![load_clip(&cache, &fingerprint).unwrap()]
            },
            SAMPLE_RATE,
        )
        .unwrap();
        let mut output = buffer(&game);
        assert!(matches!(
            game.tick(&[key(true)], &mut output),
            Err(GameError::Session(SessionError::Module(_)))
        ));
        assert!(output.iter().all(|v| *v == 0.));
        failed(&mut game);
    }
}
#[test]
fn audio_rejection_after_real_module_publication_terminalizes_the_composite() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    let fingerprint = write_fixture(&cache, false).unwrap();
    let mut game = OfflineGameSession::new(
        scene(false),
        module("audio-pose"),
        actions(),
        vec![load_clip(&cache, &fingerprint).unwrap()],
        SAMPLE_RATE,
    )
    .unwrap();
    let mut output = buffer(&game);
    assert!(matches!(
        game.tick(&[key(true)], &mut output),
        Err(GameError::Audio(_))
    ));
    assert!(output.iter().all(|v| *v == 0.));
    failed(&mut game);
}
