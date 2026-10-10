use incant_input::*;
use serde_json::{Value, json};

fn map(value: Value) -> InputActions {
    serde_json::from_value(value).unwrap()
}
fn key(code: KeyCode, down: bool) -> InputEvent {
    InputEvent::Key { code, down }
}
fn step<'a>(
    runtime: &'a mut InputRuntime,
    actions: &InputActions,
    events: &[InputEvent],
) -> &'a InputFrame {
    runtime.advance_mapped(events, 1. / 60., actions).unwrap()
}

#[test]
fn logical_button_edges_combine_alternatives_and_preserve_subtick_taps() {
    let actions = map(json!({"jump":{"kind":"button","bindings":[
        {"type":"key","code":"Space"},{"type":"mouse_button","button":"left"}]}}));
    let mut input = InputRuntime::default();
    let action = &step(&mut input, &actions, &[key(KeyCode::Space, true)]).actions["jump"];
    assert!(action.active && action.pressed && !action.released);
    let action = &step(
        &mut input,
        &actions,
        &[InputEvent::MouseButton {
            button: MouseButton::Left,
            down: true,
        }],
    )
    .actions["jump"];
    assert!(action.active && !action.pressed && !action.released);
    let action = &step(&mut input, &actions, &[key(KeyCode::Space, false)]).actions["jump"];
    assert!(action.active && !action.pressed && !action.released);
    let action = &step(
        &mut input,
        &actions,
        &[
            InputEvent::MouseButton {
                button: MouseButton::Left,
                down: false,
            },
            key(KeyCode::Space, true),
            key(KeyCode::Space, false),
        ],
    )
    .actions["jump"];
    assert!(!action.active && action.pressed && action.released);
    let action = &step(&mut input, &actions, &[]).actions["jump"];
    assert!(!action.active && !action.pressed && !action.released);
}

#[test]
fn analog_axes_use_dead_zones_normalize_diagonals_and_cancel_opposites() {
    let actions = map(json!({"move":{"kind":"axis2","dead_zone":0.2,"bindings":[
        {"type":"key","code":"KeyW","scale":[0,1]}, {"type":"key","code":"KeyS","scale":[0,-1]},
        {"type":"key","code":"KeyD","scale":[1,0]}, {"type":"gamepad_stick","id":7,"stick":"left","invert_y":true}]},
        "steer":{"kind":"axis1","dead_zone":0.2,"bindings":[{"type":"gamepad_axis","id":7,"axis":"left_x"}]}}));
    let mut input = InputRuntime::default();
    let a = &step(
        &mut input,
        &actions,
        &[key(KeyCode::KeyW, true), key(KeyCode::KeyD, true)],
    )
    .actions["move"];
    assert!((a.value[0] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert_eq!(a.value[0], a.value[1]);
    let a = &step(
        &mut input,
        &actions,
        &[key(KeyCode::KeyD, false), key(KeyCode::KeyS, true)],
    )
    .actions["move"];
    assert_eq!(a.value, [0.; 2]);
    assert!(a.released);
    step(
        &mut input,
        &actions,
        &[
            key(KeyCode::KeyW, false),
            key(KeyCode::KeyS, false),
            InputEvent::GamepadConnected { id: 7 },
        ],
    );
    let a = &step(
        &mut input,
        &actions,
        &[InputEvent::GamepadAxis {
            id: 7,
            axis: GamepadAxis::LeftX,
            value: 0.1,
        }],
    )
    .actions["move"];
    assert_eq!(a.value, [0.; 2]);
    assert!(!a.active);
    let frame = step(
        &mut input,
        &actions,
        &[InputEvent::GamepadAxis {
            id: 7,
            axis: GamepadAxis::LeftX,
            value: 0.6,
        }],
    );
    assert!((frame.actions["move"].value[0] - 0.5).abs() < 1e-12);
    assert_eq!(frame.actions["move"].value, frame.actions["steer"].value);
    let a = &step(
        &mut input,
        &actions,
        &[InputEvent::GamepadAxis {
            id: 7,
            axis: GamepadAxis::LeftY,
            value: -1.,
        }],
    )
    .actions["move"];
    assert!(a.value[0] > 0. && a.value[1] > 0.);
    assert!((a.value[0].hypot(a.value[1]) - 1.).abs() < 1e-12);
}

#[test]
fn focus_loss_disconnects_and_analog_thresholds_have_explicit_release_edges() {
    let actions = map(json!({"fire":{"kind":"button","threshold":0.7,"bindings":[
        {"type":"key","code":"Space"},{"type":"gamepad_button","id":2,"button":"right_trigger"}]}}));
    let mut input = InputRuntime::default();
    let a = &step(
        &mut input,
        &actions,
        &[
            InputEvent::GamepadConnected { id: 2 },
            InputEvent::GamepadButton {
                id: 2,
                button: GamepadButton::RightTrigger,
                value: 0.6,
            },
        ],
    )
    .actions["fire"];
    assert_eq!(a.value, [0.6, 0.]);
    assert!(!a.active);
    let a = &step(
        &mut input,
        &actions,
        &[InputEvent::GamepadButton {
            id: 2,
            button: GamepadButton::RightTrigger,
            value: 0.8,
        }],
    )
    .actions["fire"];
    assert!(a.pressed && a.active);
    let a = &step(
        &mut input,
        &actions,
        &[InputEvent::GamepadDisconnected { id: 2 }],
    )
    .actions["fire"];
    assert!(!a.active && a.released);
    let a = &step(
        &mut input,
        &actions,
        &[
            key(KeyCode::Space, true),
            InputEvent::Focus { focused: false },
        ],
    )
    .actions["fire"];
    assert!(!a.active && !a.pressed && a.released);
    let a = &step(
        &mut input,
        &actions,
        &[
            key(KeyCode::Space, true),
            InputEvent::Focus { focused: true },
        ],
    )
    .actions["fire"];
    assert!(!a.active && !a.pressed && !a.released);
}

#[test]
fn touch_gesture_actions_are_one_tick_pulses_and_rebinding_does_not_fake_a_press() {
    let actions =
        map(json!({"use":{"kind":"button","bindings":[{"type":"gesture","gesture":"tap"}]}}));
    let mut input = InputRuntime::default();
    let a = &step(
        &mut input,
        &actions,
        &[
            InputEvent::Touch {
                id: 1,
                phase: TouchPhase::Down,
                position: [10., 20.],
            },
            InputEvent::Touch {
                id: 1,
                phase: TouchPhase::Up,
                position: [10., 20.],
            },
            key(KeyCode::KeyE, true),
        ],
    )
    .actions["use"];
    assert!(a.active && a.pressed);
    let a = &step(&mut input, &actions, &[]).actions["use"];
    assert!(!a.active && a.released);
    let rebound = map(json!({"use":{"kind":"button","bindings":[{"type":"key","code":"KeyE"}]}}));
    let a = &step(&mut input, &rebound, &[]).actions["use"];
    assert!(a.active && !a.pressed && !a.released);
    let a = &step(&mut input, &rebound, &[key(KeyCode::KeyE, false)]).actions["use"];
    assert!(!a.active && a.released);
}

#[test]
fn invalid_maps_or_event_suffixes_leave_all_action_and_physical_state_unchanged() {
    let actions = map(json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}));
    let mut input = InputRuntime::default();
    step(&mut input, &actions, &[key(KeyCode::Space, true)]);
    let before = json!(input.frame());
    for bad in [
        json!({"bad name":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}),
        json!({"jump":{"kind":"button","threshold":0,"bindings":[{"type":"key","code":"Space"}]}}),
        json!({"jump":{"kind":"axis1","dead_zone":1,"bindings":[{"type":"key","code":"Space"}]}}),
        json!({"jump":{"kind":"axis1","bindings":[{"type":"key","code":"Space","scale":[1,1]}]}}),
        json!({"jump":{"kind":"button","bindings":[{"type":"gamepad_stick","id":0,"stick":"left"}]}}),
        json!({"jump":{"kind":"axis2","bindings":[{"type":"key","code":"Space","scale":[2,0]}]}}),
    ] {
        assert!(
            input
                .advance_mapped(&[key(KeyCode::Space, false)], 1. / 60., &map(bad))
                .is_err()
        );
        assert_eq!(json!(input.frame()), before);
    }
    let too_many = (0..65)
        .map(|n| (format!("action{n}"), actions["jump"].clone()))
        .collect();
    assert!(validate_actions(&too_many).is_err());
    let mut too_many_bindings = actions.clone();
    too_many_bindings.insert(
        "jump".into(),
        InputAction::Button {
            threshold: 0.5,
            bindings: vec![
                InputBinding::Key {
                    code: KeyCode::Space,
                    scale: [1., 0.]
                };
                17
            ],
        },
    );
    assert!(validate_actions(&too_many_bindings).is_err());
    assert!(
        input
            .advance_mapped(
                &[
                    key(KeyCode::Space, false),
                    InputEvent::GamepadAxis {
                        id: 99,
                        axis: GamepadAxis::LeftX,
                        value: 0.1
                    }
                ],
                1. / 60.,
                &actions
            )
            .is_err()
    );
    assert_eq!(json!(input.frame()), before);
    assert!(step(&mut input, &actions, &[key(KeyCode::Space, false)]).actions["jump"].released);
    let unbound = map(json!({"jump":{"kind":"button","bindings":[]}}));
    let a = &step(&mut input, &unbound, &[key(KeyCode::Space, true)]).actions["jump"];
    assert!(!a.active && !a.pressed && !a.released);
}

#[test]
fn replay_seek_reconstructs_held_action_without_replaying_its_press() {
    let actions = map(json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}));
    let replay = InputReplay::from_text(
        &json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":5,
        "frames":[{"tick":1,"events":[{"type":"key","code":"Space","down":true}]},
        {"tick":4,"events":[{"type":"key","code":"Space","down":false}]}]})
        .to_string(),
        2,
        60,
    )
    .unwrap();
    let mut input = replay.initial_state();
    let a = &step(&mut input, &actions, replay.events_at(3).unwrap()).actions["jump"];
    assert!(a.active && !a.pressed && !a.released);
    let a = &step(&mut input, &actions, replay.events_at(4).unwrap()).actions["jump"];
    assert!(!a.active && a.released);
}
