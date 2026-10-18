#![cfg(feature = "static-conformance")]
//! Real Rust SDK decoding, complementing input_parity's unchanged raw WAT hash.
use incant_input::*;
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
use std::fmt::Write;

const DT: f64 = 0.125;
const PAD: u32 = u32::MAX;
const BUTTONS: [GamepadButton; 17] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::West,
    GamepadButton::North,
    GamepadButton::LeftShoulder,
    GamepadButton::RightShoulder,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::Guide,
    GamepadButton::LeftStick,
    GamepadButton::RightStick,
    GamepadButton::DpadUp,
    GamepadButton::DpadDown,
    GamepadButton::DpadLeft,
    GamepadButton::DpadRight,
];
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([0; 16]),
            8,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn bits(value: [f64; 2]) -> [u64; 2] {
    value.map(f64::to_bits)
}
fn mask<T: Ord>(set: &std::collections::BTreeSet<T>, tag: impl Fn(&T) -> u8) -> u128 {
    set.iter().fold(0, |bits, value| bits | 1u128 << tag(value))
}
// Expected values come from the native typed InputFrame, never the wire encoder
// or the guest parser. Hex bit patterns retain signed zero and every f64 bit.
fn expected(frame: &InputFrame, tick: usize) -> Vec<String> {
    let mut rows = vec![format!("frame {tick} {}", frame.focused)];
    let k = &frame.keyboard;
    rows.push(format!(
        "keys {:x} {:x} {:x}",
        mask(&k.held, |v| *v as u8),
        mask(&k.pressed, |v| *v as u8),
        mask(&k.released, |v| *v as u8)
    ));
    let m = &frame.mouse;
    rows.push(format!(
        "mouse {:x} {:x} {:x} {:x?} {:x?} {:x?} {:x?}",
        mask(&m.buttons.held, |v| *v as u8),
        mask(&m.buttons.pressed, |v| *v as u8),
        mask(&m.buttons.released, |v| *v as u8),
        m.position.map(bits),
        bits(m.motion),
        bits(m.wheel_pixels),
        bits(m.wheel_lines)
    ));
    for (name, a) in &frame.actions {
        rows.push(format!(
            "action {name} {} {} {} {} {:x?}",
            a.kind as u8,
            a.active,
            a.pressed,
            a.released,
            bits(a.value)
        ));
    }
    for (id, p) in &frame.gamepads {
        let tag = |v: &GamepadButton| BUTTONS.iter().position(|b| b == v).unwrap() as u8;
        let present = p.values.keys().fold(0u32, |mask, b| mask | 1u32 << tag(b));
        rows.push(format!(
            "pad {id} {:x} {:x} {:x} {:x} {:x?} {:x?}",
            mask(&p.buttons.held, tag),
            mask(&p.buttons.pressed, tag),
            mask(&p.buttons.released, tag),
            present,
            bits(p.left_stick),
            bits(p.right_stick)
        ));
        let mut values = format!("values {id}");
        for button in BUTTONS {
            write!(
                values,
                " {:x}/{:x?}",
                p.values.get(&button).copied().unwrap_or(0.).to_bits(),
                p.values.get(&button).map(|v| v.to_bits())
            )
            .unwrap();
        }
        rows.push(values);
    }
    rows.extend(frame.connected.iter().map(|id| format!("connected {id}")));
    rows.extend(
        frame
            .disconnected
            .iter()
            .map(|id| format!("disconnected {id}")),
    );
    for (id, t) in &frame.touches {
        rows.push(format!(
            "touch {id} {:x?} {:x?}",
            bits(t.position),
            bits(t.delta)
        ));
    }
    for c in &frame.touch_changes {
        rows.push(format!(
            "change {} {} {:x?}",
            c.id,
            c.phase as u8,
            bits(c.position)
        ));
    }
    for g in &frame.gestures {
        rows.push(match g {
            Gesture::Tap { id, position } => format!("tap {id} {:x?}", bits(*position)),
            Gesture::LongPress { id, position } => format!("long {id} {:x?}", bits(*position)),
            Gesture::Swipe {
                id,
                direction,
                distance,
                duration_seconds,
            } => format!(
                "swipe {id} {:x?} {:x} {:x}",
                bits(*direction),
                distance.to_bits(),
                duration_seconds.to_bits()
            ),
            Gesture::Pinch {
                ids,
                center,
                scale_delta,
                rotation_delta,
            } => format!(
                "pinch {ids:?} {:x?} {:x} {:x}",
                bits(*center),
                scale_delta.to_bits(),
                rotation_delta.to_bits()
            ),
        });
    }
    rows
}
fn touch(id: u32, phase: TouchPhase, position: [f64; 2]) -> InputEvent {
    InputEvent::Touch {
        id,
        phase,
        position,
    }
}
fn actions() -> ValidatedActions {
    ValidatedActions::new(serde_json::from_value(serde_json::json!({
        "jump": {"kind":"button", "bindings":[{"type":"key", "code":"Space"}]},
        "move": {"kind":"axis2", "bindings":[{"type":"gamepad_stick", "id":PAD, "stick":"left"}]},
        "turn": {"kind":"axis1", "bindings":[{"type":"gamepad_axis", "id":PAD, "axis":"right_x"}]},
        "tap": {"kind":"button", "bindings":[{"type":"gesture", "gesture":"tap"}]},
        "hold": {"kind":"button", "bindings":[{"type":"gesture", "gesture":"long_press"}]},
        "swipe": {"kind":"button", "bindings":[{"type":"gesture", "gesture":"swipe"}]}
    })).unwrap()).unwrap()
}
fn packets() -> Vec<Vec<InputEvent>> {
    let mut first = vec![
        InputEvent::GamepadConnected { id: PAD },
        InputEvent::GamepadConnected { id: 7 },
        InputEvent::Key {
            code: KeyCode::Space,
            down: true,
        },
        InputEvent::Key {
            code: KeyCode::F12,
            down: true,
        },
        InputEvent::PointerPosition {
            position: [1.5, -2.25],
        },
        InputEvent::PointerMotion { delta: [-0.5, 4.] },
        InputEvent::Wheel {
            delta: [1., -2.],
            unit: WheelUnit::Pixels,
        },
        InputEvent::Wheel {
            delta: [-3., 4.],
            unit: WheelUnit::Lines,
        },
        touch(u32::MAX, TouchPhase::Down, [5., -0.]),
        touch(u32::MAX, TouchPhase::Up, [5., -0.]),
    ];
    first.extend(
        BUTTONS
            .into_iter()
            .enumerate()
            .map(|(i, button)| InputEvent::GamepadButton {
                id: PAD,
                button,
                value: i as f64 / 16.,
            }),
    );
    first.extend(
        [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ]
        .map(|button| InputEvent::MouseButton { button, down: true }),
    );
    let mut packets = vec![
        first,
        vec![
            InputEvent::GamepadButton {
                id: PAD,
                button: GamepadButton::South,
                value: 1.,
            },
            InputEvent::GamepadAxis {
                id: PAD,
                axis: GamepadAxis::LeftX,
                value: 0.5,
            },
            InputEvent::GamepadAxis {
                id: PAD,
                axis: GamepadAxis::LeftY,
                value: -0.75,
            },
            InputEvent::GamepadAxis {
                id: PAD,
                axis: GamepadAxis::RightX,
                value: -0.5,
            },
            InputEvent::GamepadAxis {
                id: PAD,
                axis: GamepadAxis::RightY,
                value: -0.,
            },
            InputEvent::Key {
                code: KeyCode::Space,
                down: false,
            },
            touch(PAD - 1, TouchPhase::Down, [0., 0.]),
            touch(PAD - 1, TouchPhase::Up, [60., 80.]),
        ],
        vec![touch(4, TouchPhase::Down, [0., 0.])],
    ];
    packets.extend(vec![vec![]; 4]); // exact half-second long press
    packets.extend([
        vec![touch(5, TouchPhase::Down, [16., 0.])],
        vec![touch(5, TouchPhase::Move, [0., 32.])], // scale 2, quarter-turn
        vec![InputEvent::Focus { focused: false }],  // two Cancel records, no End required
        vec![touch(4, TouchPhase::Move, [99., 99.])], // ignored while unfocused
        vec![
            InputEvent::Focus { focused: true },
            touch(4, TouchPhase::Down, [7., 8.]),
        ],
        vec![
            touch(4, TouchPhase::Cancel, [9., 10.]),
            InputEvent::GamepadDisconnected { id: PAD },
        ],
        vec![InputEvent::GamepadConnected { id: PAD }], // reused ID has no old values/edges
        vec![
            touch(4, TouchPhase::Down, [11., 12.]),
            touch(4, TouchPhase::Up, [11., 12.]),
        ],
        vec![
            InputEvent::GamepadDisconnected { id: 7 },
            InputEvent::GamepadDisconnected { id: PAD },
        ],
    ]);
    packets
}

#[test]
fn actual_rust_input_views_match_every_typed_native_collection_and_edge() {
    let mut a = world();
    let mut b = world();
    let mut native = StaticModule::new(embedded::linked("rust-input").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest("rust-input")).unwrap();
    let mut input = InputRuntime::default();
    let bindings = actions();
    let mut observed = Vec::new();
    let mut kinds = std::collections::BTreeSet::new();
    for (index, events) in packets().iter().enumerate() {
        input.advance_validated(events, DT, &bindings).unwrap();
        let f = input.frame();
        match index {
            0 => {
                assert_eq!(f.gamepads[&PAD].values.len(), 17);
                assert_eq!(f.gamepads[&PAD].values[&GamepadButton::South], 0.);
                assert!(f.gamepads[&7].values.is_empty());
                assert_eq!(f.connected.iter().copied().collect::<Vec<_>>(), [7, PAD]);
                assert!(f.actions["jump"].pressed && f.actions["tap"].pressed);
            }
            6 => assert!(f.actions["hold"].pressed),
            9 => {
                assert!(!f.focused && f.touches.is_empty() && f.gestures.is_empty());
                assert_eq!(
                    f.touch_changes
                        .iter()
                        .map(|c| (c.id, c.phase))
                        .collect::<Vec<_>>(),
                    [(4, TouchPhase::Cancel), (5, TouchPhase::Cancel)]
                );
                assert!(f.keyboard.held.is_empty() && !f.keyboard.released.is_empty());
                assert!(f.mouse.buttons.held.is_empty() && f.mouse.position.is_none());
                assert!(f.gamepads[&PAD].values.values().all(|v| *v == 0.));
                assert_eq!(f.gamepads[&PAD].left_stick, [0., 0.]);
            }
            10 => assert!(f.touch_changes.is_empty() && f.touches.is_empty()),
            11 => assert_eq!(f.touches[&4].delta, [0., 0.]),
            13 => {
                assert!(f.gamepads[&PAD].values.is_empty());
                assert!(f.gamepads[&PAD].buttons.held.is_empty());
            }
            _ => (),
        }
        for g in &f.gestures {
            kinds.insert(match g {
                Gesture::Tap { .. } => 0,
                Gesture::LongPress { .. } => 1,
                Gesture::Swipe { .. } => 2,
                Gesture::Pinch { .. } => 3,
            });
        }
        let previous_count = observed.len();
        observed.extend(expected(f, index + 1));
        a.step().unwrap();
        b.step().unwrap();
        native.tick_with_input(&mut a, DT, &input).unwrap();
        dev.tick_with_input(&mut b, DT, &input).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        for actual in [
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries(),
        ] {
            assert_eq!(
                actual.iter().map(|r| r.message.clone()).collect::<Vec<_>>(),
                observed
            );
            assert!(
                actual
                    .iter()
                    .all(|r| r.level == incant_wasm_runtime::LogLevel::Info)
            );
            assert!(
                actual[previous_count..]
                    .iter()
                    .all(|r| r.tick == (index + 1) as u64)
            );
        }
    }
    assert_eq!(kinds, [0, 1, 2, 3].into());
}

#[test]
fn actual_rust_input_refusals_discard_numeric_and_log_prefixes_on_both_hosts() {
    for suffix in ["scratch", "callback", "stale"] {
        let name = format!("rust-input-{suffix}");
        let mut a = world();
        let mut b = world();
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let input = InputRuntime::default();
        if suffix == "stale" {
            native.tick_with_input(&mut a, DT, &input).unwrap();
            dev.tick_with_input(&mut b, DT, &input).unwrap();
            assert_eq!(
                a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
                42.
            );
        }
        let before = a.snapshot();
        assert!(
            native.tick_with_input(&mut a, DT, &input).is_err(),
            "{suffix}"
        );
        assert!(dev.tick_with_input(&mut b, DT, &input).is_err(), "{suffix}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        dev.reload(&guest("rust-input")).unwrap();
        let logs = dev.logs().unwrap().entries();
        assert_eq!(logs.len(), usize::from(suffix == "stale"));
        if suffix == "stale" {
            assert_eq!(logs[0].message, "input staged");
        }
        assert!(native.tick_with_input(&mut a, DT, &input).is_err());
    }
}
