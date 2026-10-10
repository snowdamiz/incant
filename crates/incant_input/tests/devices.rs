use incant_input::*;
use serde_json::json;
const DT: f64 = 1. / 60.;

#[test]
fn keys_track_edges_without_repeat_and_keep_short_taps_within_one_tick() {
    let mut input = InputRuntime::default();
    let down = InputEvent::Key {
        code: KeyCode::KeyW,
        down: true,
    };
    let up = InputEvent::Key {
        code: KeyCode::KeyW,
        down: false,
    };
    let f = input.advance(&[down.clone(), down.clone()], DT).unwrap();
    assert!(f.keyboard.held.contains(&KeyCode::KeyW));
    assert_eq!(f.keyboard.pressed.len(), 1);
    let f = input.advance(&[down], DT).unwrap();
    assert!(f.keyboard.held.contains(&KeyCode::KeyW));
    assert!(f.keyboard.pressed.is_empty());
    let f = input.advance(&[up], DT).unwrap();
    assert!(f.keyboard.held.is_empty());
    assert!(f.keyboard.released.contains(&KeyCode::KeyW));
    let f = input.advance(&[], DT).unwrap();
    assert!(f.keyboard.released.is_empty());
    let f = input
        .advance(
            &[
                InputEvent::Key {
                    code: KeyCode::Space,
                    down: true,
                },
                InputEvent::Key {
                    code: KeyCode::Space,
                    down: false,
                },
            ],
            DT,
        )
        .unwrap();
    assert!(f.keyboard.pressed.contains(&KeyCode::Space));
    assert!(f.keyboard.released.contains(&KeyCode::Space));
    assert!(f.keyboard.held.is_empty());
}

#[test]
fn mouse_motion_and_wheel_units_accumulate_only_for_the_current_tick() {
    let mut input = InputRuntime::default();
    let f = input
        .advance(
            &[InputEvent::PointerPosition {
                position: [300., 200.],
            }],
            DT,
        )
        .unwrap();
    assert_eq!(f.mouse.motion, [0., 0.]);
    let f = input
        .advance(
            &[
                InputEvent::PointerPosition {
                    position: [305., 198.],
                },
                InputEvent::PointerPosition {
                    position: [306., 201.],
                },
                InputEvent::PointerMotion { delta: [-1., 2.] },
                InputEvent::Wheel {
                    delta: [0., 3.],
                    unit: WheelUnit::Lines,
                },
                InputEvent::Wheel {
                    delta: [0., 2.],
                    unit: WheelUnit::Lines,
                },
                InputEvent::Wheel {
                    delta: [4., -20.],
                    unit: WheelUnit::Pixels,
                },
                InputEvent::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                },
            ],
            DT,
        )
        .unwrap();
    assert_eq!(f.mouse.motion, [5., 3.]);
    assert_eq!(f.mouse.position, Some([306., 201.]));
    assert_eq!(f.mouse.wheel_lines, [0., 5.]);
    assert_eq!(f.mouse.wheel_pixels, [4., -20.]);
    let f = input.advance(&[], DT).unwrap();
    assert_eq!(f.mouse.motion, [0., 0.]);
    assert_eq!(f.mouse.wheel_lines, [0., 0.]);
    assert_eq!(f.mouse.wheel_pixels, [0., 0.]);
    assert!(f.mouse.buttons.pressed.is_empty());
    assert!(f.mouse.buttons.held.contains(&MouseButton::Left));
}

#[test]
fn controllers_preserve_analog_values_threshold_edges_and_hotplug_identity() {
    let mut input = InputRuntime::default();
    let f = input
        .advance(
            &[
                InputEvent::GamepadConnected { id: 7 },
                InputEvent::GamepadConnected { id: 9 },
                InputEvent::GamepadButton {
                    id: 7,
                    button: GamepadButton::LeftTrigger,
                    value: 0.49,
                },
                InputEvent::GamepadAxis {
                    id: 9,
                    axis: GamepadAxis::RightY,
                    value: -0.75,
                },
            ],
            DT,
        )
        .unwrap();
    assert_eq!(f.connected.iter().copied().collect::<Vec<_>>(), [7, 9]);
    assert!(f.gamepads[&7].buttons.held.is_empty());
    assert_eq!(f.gamepads[&9].right_stick, [0., -0.75]);
    let f = input
        .advance(
            &[InputEvent::GamepadButton {
                id: 7,
                button: GamepadButton::LeftTrigger,
                value: 0.5,
            }],
            DT,
        )
        .unwrap();
    assert_eq!(f.gamepads[&7].values[&GamepadButton::LeftTrigger], 0.5);
    assert!(
        f.gamepads[&7]
            .buttons
            .pressed
            .contains(&GamepadButton::LeftTrigger)
    );
    let f = input
        .advance(
            &[InputEvent::GamepadButton {
                id: 7,
                button: GamepadButton::LeftTrigger,
                value: 0.9,
            }],
            DT,
        )
        .unwrap();
    assert!(f.gamepads[&7].buttons.pressed.is_empty());
    assert!(f.connected.is_empty());
    let f = input
        .advance(
            &[
                InputEvent::GamepadButton {
                    id: 7,
                    button: GamepadButton::LeftTrigger,
                    value: 0.1,
                },
                InputEvent::GamepadDisconnected { id: 9 },
            ],
            DT,
        )
        .unwrap();
    assert!(
        f.gamepads[&7]
            .buttons
            .released
            .contains(&GamepadButton::LeftTrigger)
    );
    assert!(!f.gamepads.contains_key(&9));
    assert!(f.disconnected.contains(&9));
    let f = input
        .advance(&[InputEvent::GamepadConnected { id: 9 }], DT)
        .unwrap();
    assert_eq!(f.gamepads[&9], GamepadFrame::default());
}

#[test]
fn focus_loss_releases_controls_cancels_touches_and_drops_unfocused_actions() {
    let mut input = InputRuntime::default();
    input
        .advance(
            &[
                InputEvent::GamepadConnected { id: 0 },
                InputEvent::Key {
                    code: KeyCode::KeyW,
                    down: true,
                },
                InputEvent::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                },
                InputEvent::GamepadButton {
                    id: 0,
                    button: GamepadButton::South,
                    value: 1.,
                },
                InputEvent::GamepadAxis {
                    id: 0,
                    axis: GamepadAxis::LeftX,
                    value: 1.,
                },
                InputEvent::Touch {
                    id: 1,
                    phase: TouchPhase::Down,
                    position: [10., 10.],
                },
            ],
            DT,
        )
        .unwrap();
    let f = input
        .advance(
            &[
                InputEvent::Key {
                    code: KeyCode::Space,
                    down: true,
                },
                InputEvent::Focus { focused: false },
                InputEvent::Key {
                    code: KeyCode::KeyA,
                    down: true,
                },
                InputEvent::PointerMotion { delta: [100., 10.] },
                InputEvent::Touch {
                    id: 1,
                    phase: TouchPhase::Up,
                    position: [10., 10.],
                },
            ],
            DT,
        )
        .unwrap();
    assert!(!f.focused);
    assert!(f.keyboard.held.is_empty());
    assert!(f.keyboard.pressed.is_empty());
    assert!(f.keyboard.released.contains(&KeyCode::KeyW));
    assert!(f.mouse.buttons.held.is_empty());
    assert_eq!(f.gamepads[&0].left_stick, [0., 0.]);
    assert_eq!(f.gamepads[&0].values[&GamepadButton::South], 0.);
    assert_eq!(f.mouse.motion, [0., 0.]);
    assert!(f.touches.is_empty());
    assert!(f.gestures.is_empty());
    assert_eq!(f.touch_changes[0].phase, TouchPhase::Cancel);
    let f = input
        .advance(
            &[
                InputEvent::Focus { focused: true },
                InputEvent::PointerPosition {
                    position: [900., 600.],
                },
            ],
            DT,
        )
        .unwrap();
    assert_eq!(f.mouse.motion, [0., 0.]);
    assert!(f.keyboard.held.is_empty());
}

#[test]
fn malformed_and_oversized_packets_are_atomic_and_unknown_tags_fail() {
    let mut input = InputRuntime::default();
    input
        .advance(&[InputEvent::GamepadConnected { id: 0 }], DT)
        .unwrap();
    let before = input.frame().clone();
    for invalid in [
        InputEvent::PointerMotion {
            delta: [f64::NAN, 0.],
        },
        InputEvent::PointerPosition {
            position: [1_000_001., 0.],
        },
        InputEvent::GamepadButton {
            id: 0,
            button: GamepadButton::South,
            value: 1.1,
        },
        InputEvent::GamepadAxis {
            id: 0,
            axis: GamepadAxis::LeftX,
            value: f64::INFINITY,
        },
        InputEvent::GamepadButton {
            id: 3,
            button: GamepadButton::South,
            value: 1.,
        },
        InputEvent::GamepadConnected { id: 0 },
        InputEvent::GamepadDisconnected { id: 3 },
        InputEvent::Touch {
            id: 4,
            phase: TouchPhase::Up,
            position: [0., 0.],
        },
    ] {
        assert!(
            input
                .advance(
                    &[
                        InputEvent::Key {
                            code: KeyCode::KeyW,
                            down: true
                        },
                        invalid
                    ],
                    DT
                )
                .is_err()
        );
        assert_eq!(input.frame(), &before);
    }
    assert!(
        input
            .advance(
                &vec![InputEvent::Focus { focused: true }; MAX_EVENTS_PER_TICK + 1],
                DT
            )
            .is_err()
    );
    for dt in [0., -1., f64::NAN, 1.1, 1. / 241.] {
        assert_eq!(input.advance(&[], dt).unwrap_err(), InputError::Step);
    }
    assert_eq!(input.frame(), &before);
    for value in [
        json!({"type":"shell","command":"x"}),
        json!({"type":"key","code":"Unsupported","down":true}),
        json!({"type":"key","code":"KeyW","down":true,"extra":1}),
    ] {
        assert!(serde_json::from_value::<InputEvent>(value).is_err());
    }
    let f = input
        .advance(
            &[InputEvent::Key {
                code: KeyCode::KeyW,
                down: true,
            }],
            DT,
        )
        .unwrap();
    assert!(f.keyboard.held.contains(&KeyCode::KeyW));
}

#[test]
fn connected_device_and_contact_counts_are_bounded_without_losing_existing_state() {
    let mut input = InputRuntime::default();
    let mut events = Vec::new();
    for id in 0..MAX_GAMEPADS as u32 {
        events.push(InputEvent::GamepadConnected { id });
    }
    for id in 0..MAX_TOUCHES as u32 {
        events.push(InputEvent::Touch {
            id,
            phase: TouchPhase::Down,
            position: [id as f64, 0.],
        });
    }
    input.advance(&events, DT).unwrap();
    let before = input.frame().clone();
    for event in [
        InputEvent::GamepadConnected { id: 100 },
        InputEvent::Touch {
            id: 100,
            phase: TouchPhase::Down,
            position: [0., 0.],
        },
        InputEvent::Touch {
            id: 0,
            phase: TouchPhase::Down,
            position: [0., 0.],
        },
    ] {
        assert!(input.advance(&[event], DT).is_err());
        assert_eq!(input.frame(), &before);
    }
}
