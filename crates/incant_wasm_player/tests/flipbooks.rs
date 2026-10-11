#![cfg(feature = "fixture")]
#[path = "../../incant_runtime/tests/support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_game_runtime::{GameError, HostedOfflineGameSession, SceneHost};
use incant_input::{GamepadAxis, InputAction, InputActions, InputBinding, InputEvent, KeyCode};
use incant_runtime::{NativeWorld, WorldIdentity};
use incant_script::SessionError;
use std::convert::Infallible;
// Capability lifecycle test only. Actual prepared native visual binding and GPU
// publication are covered by the separate source-free existing-player proof.
struct Observer {
    identity: WorldIdentity,
    frames: Vec<u16>,
}
impl SceneHost for Observer {
    type Settings = ();
    type Error = Infallible;
    fn prepare(world: &NativeWorld, _: ()) -> Result<Self, Self::Error> {
        Ok(Self {
            identity: world.spatial_view().unwrap().identity().clone(),
            frames: vec![],
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        assert_eq!(&self.identity, world.spatial_view().unwrap().identity());
        self.frames
            .push(world.flipbook_state(id(1)).unwrap().frame());
        Ok(())
    }
}
fn actions() -> InputActions {
    [
        ("flip_pause", KeyCode::KeyP),
        ("flip_play", KeyCode::KeyL),
        ("flip_invalid", KeyCode::KeyI),
    ]
    .into_iter()
    .map(|(name, code)| {
        (
            name.into(),
            InputAction::Button {
                bindings: vec![InputBinding::Key {
                    code,
                    scale: [1., 0.],
                }],
                threshold: 0.5,
            },
        )
    })
    .collect()
}
#[test]
fn real_static_controls_keep_preaccept_retry_and_postaccept_terminal_ordering() {
    let mut game = HostedOfflineGameSession::<Observer>::new(
        scene(2),
        incant_wasm_runtime::embedded::linked("flipbook-game").unwrap(),
        actions(),
        vec![],
        44117,
        (),
    )
    .unwrap();
    let count = game.next_output_frames().unwrap() * 2;
    let pause = InputEvent::Key {
        code: KeyCode::KeyP,
        down: true,
    };
    let mut short = vec![0.7; count - 1];
    assert!(matches!(
        game.tick(std::slice::from_ref(&pause), &mut short),
        Err(GameError::Buffer { .. })
    ));
    assert!(short.iter().all(|&n| n == 0.7));
    let mut output = vec![0.7; count];
    assert!(matches!(
        game.tick(
            &[
                pause.clone(),
                InputEvent::GamepadAxis {
                    id: 99,
                    axis: GamepadAxis::LeftX,
                    value: 0.5
                }
            ],
            &mut output
        ),
        Err(GameError::Session(SessionError::Input(_)))
    ));
    assert!(output.iter().all(|&n| n == 0.7));
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(game.host().unwrap().frames.is_empty());
    game.tick(&[pause], &mut output).unwrap();
    assert_eq!(game.host().unwrap().frames, [0]);
    assert!(
        !game
            .world()
            .unwrap()
            .flipbook_state(id(1))
            .unwrap()
            .playing()
    );
    assert!(game.input().unwrap().actions["flip_pause"].pressed);
    let mut output = vec![0.7; game.next_output_frames().unwrap() * 2];
    assert!(
        game.tick(
            &[InputEvent::Key {
                code: KeyCode::KeyI,
                down: true
            }],
            &mut output
        )
        .is_err()
    );
    assert!(output.iter().all(|&n| n == 0.));
    assert!(game.is_failed());
    assert!(game.world().is_err());
    assert!(game.input().is_err());
    assert!(game.host().is_err());
    assert!(game.next_output_frames().is_err());
}
