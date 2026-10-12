use super::*;
use incant_input::{InputEvent, InputRuntime, KeyCode};
const DT: f64 = 1. / 60.;
#[test]
fn failed_backend_initialization_is_a_visible_error_without_dummy_fallback() {
    assert!(matches!(
        require_backend(Err(gilrs::Error::InvalidAxisToBtn)),
        Err(Error::GamepadBackend)
    ));
}
fn axis(axis: Axis, value: f32) -> EventType {
    // The adapter receives already normalized events and never inspects the
    // opaque hardware code. Use a public synthetic code without OS discovery.
    EventType::AxisChanged(axis, value, Button::South.to_nec().unwrap())
}
fn changed(button: Button, value: f32) -> EventType {
    EventType::ButtonChanged(button, value, button.to_nec().unwrap())
}
fn batch(
    adapter: &mut GamepadAdapter<usize>,
    pending: &mut PendingInput,
    events: &[(usize, EventType)],
    active: bool,
) {
    let mut events = events.iter().copied();
    drain(adapter, pending, active, || events.next()).unwrap();
}
#[test]
fn measured_analog_values_and_all_standard_buttons_map_without_invented_edges() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, EventType::Connected),
            (
                42,
                EventType::ButtonPressed(
                    Button::LeftTrigger2,
                    Button::LeftTrigger2.to_nec().unwrap(),
                ),
            ),
            (42, changed(Button::LeftTrigger2, 0.8)),
        ],
        true,
    );
    assert_eq!(
        pending.events(),
        [
            InputEvent::GamepadConnected { id: 0 },
            InputEvent::GamepadButton {
                id: 0,
                button: GamepadButton::LeftTrigger,
                value: f64::from(0.8f32)
            }
        ]
    );
    let buttons = [
        Button::South,
        Button::East,
        Button::West,
        Button::North,
        Button::LeftTrigger,
        Button::RightTrigger,
        Button::LeftTrigger2,
        Button::RightTrigger2,
        Button::Select,
        Button::Start,
        Button::Mode,
        Button::LeftThumb,
        Button::RightThumb,
        Button::DPadUp,
        Button::DPadDown,
        Button::DPadLeft,
        Button::DPadRight,
    ];
    for native in buttons {
        translate(&mut adapter, &mut pending, 42, changed(native, 0.6), true).unwrap();
    }
    let mut input = InputRuntime::default();
    input.advance(pending.events(), DT).unwrap();
    assert_eq!(input.frame().gamepads[&0].buttons.held.len(), 17);
    assert_eq!(
        input.frame().gamepads[&0].values[&GamepadButton::LeftShoulder],
        f64::from(0.6f32)
    );
    pending.clear();
    adapter.commit();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, changed(Button::LeftTrigger2, 0.)),
            (42, EventType::Disconnected),
        ],
        true,
    );
    input.advance(pending.events(), DT).unwrap();
    assert!(input.frame().gamepads.is_empty());
    assert!(input.frame().disconnected.contains(&0));
}
#[test]
fn native_measured_trigger_and_stick_events_drive_typed_actions_and_preserve_taps() {
    let actions = serde_json::from_value(serde_json::json!({
        "fire":{"kind":"button","threshold":0.85,"bindings":[{"type":"gamepad_button","id":0,"button":"right_trigger"}]},
        "move":{"kind":"axis2","dead_zone":0.,"bindings":[{"type":"gamepad_stick","id":0,"stick":"left"}]}
    })).unwrap();
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, EventType::Connected),
            (
                42,
                EventType::ButtonPressed(
                    Button::RightTrigger2,
                    Button::RightTrigger2.to_nec().unwrap(),
                ),
            ),
            (42, changed(Button::RightTrigger2, 0.8)),
            (42, axis(Axis::LeftStickY, 0.5)),
        ],
        true,
    );
    let mut input = InputRuntime::default();
    input
        .advance_mapped(pending.events(), DT, &actions)
        .unwrap();
    assert!(
        !input.frame().actions["fire"].active,
        "analog press edges do not fabricate full depression"
    );
    assert_eq!(input.frame().actions["move"].value, [0., -0.5]);
    adapter.commit();
    pending.clear();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, changed(Button::RightTrigger2, 1.)),
            (42, changed(Button::RightTrigger2, 0.)),
        ],
        true,
    );
    input
        .advance_mapped(pending.events(), DT, &actions)
        .unwrap();
    assert!(!input.frame().actions["fire"].active);
    assert!(input.frame().actions["fire"].pressed && input.frame().actions["fire"].released);
}
#[test]
fn sticks_use_down_positive_and_dpad_reversals_release_opposite_before_pressing() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (70, EventType::Connected),
            (70, axis(Axis::LeftStickX, -0.5)),
            (70, axis(Axis::LeftStickY, 1.)),
            (70, axis(Axis::RightStickX, 1.)),
            (70, axis(Axis::RightStickY, -0.75)),
            (70, axis(Axis::DPadY, 1.)),
            (70, axis(Axis::DPadX, -1.)),
        ],
        true,
    );
    let mut input = InputRuntime::default();
    input.advance(pending.events(), DT).unwrap();
    let pad = &input.frame().gamepads[&0];
    assert_eq!(pad.left_stick, [-0.5, -1.]);
    assert_eq!(pad.right_stick, [1., 0.75]);
    assert!(pad.buttons.held.contains(&GamepadButton::DpadUp));
    assert!(pad.buttons.held.contains(&GamepadButton::DpadLeft));
    adapter.commit();
    pending.clear();
    batch(
        &mut adapter,
        &mut pending,
        &[(70, axis(Axis::DPadY, -1.)), (70, axis(Axis::DPadX, 1.))],
        true,
    );
    input.advance(pending.events(), DT).unwrap();
    let pad = &input.frame().gamepads[&0];
    assert!(pad.buttons.released.contains(&GamepadButton::DpadUp));
    assert!(pad.buttons.released.contains(&GamepadButton::DpadLeft));
    assert!(pad.buttons.pressed.contains(&GamepadButton::DpadDown));
    assert!(pad.buttons.pressed.contains(&GamepadButton::DpadRight));
}
#[test]
fn raw_poll_work_and_combined_input_queue_are_bounded_even_for_ignored_paused_events() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    let mut calls = 0;
    let result = drain(&mut adapter, &mut pending, false, || {
        calls += 1;
        Some((0, EventType::Dropped))
    });
    assert!(matches!(result, Err(Error::InputCapacity)));
    assert_eq!(calls, MAX_EVENTS_PER_TICK + 1);
    assert!(pending.events().is_empty());
    for _ in 0..MAX_EVENTS_PER_TICK {
        pending
            .push(InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            })
            .unwrap();
    }
    let mut events = [(3, EventType::Connected)].into_iter();
    assert!(matches!(
        drain(&mut adapter, &mut pending, true, || events.next()),
        Err(Error::InputCapacity)
    ));
    assert_eq!(pending.events().len(), MAX_EVENTS_PER_TICK);
}
#[test]
fn focus_reset_discards_unpublished_hotplug_then_resume_keeps_neutral_controls() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    batch(
        &mut adapter,
        &mut pending,
        &[(70, EventType::Connected), (70, changed(Button::South, 1.))],
        true,
    );
    pending.reset_focus(false);
    adapter.discard_pending();
    batch(
        &mut adapter,
        &mut pending,
        &[(70, axis(Axis::LeftStickX, 1.))],
        false,
    );
    pending.reset_focus(true);
    adapter.discard_pending();
    while let Some(event) = adapter.next_connection_event() {
        pending.push(event).unwrap();
    }
    let mut input = InputRuntime::default();
    input.advance(pending.events(), DT).unwrap();
    assert!(input.frame().gamepads[&0].buttons.held.is_empty());
    assert_eq!(input.frame().gamepads[&0].left_stick, [0.; 2]);
}
#[test]
fn malformed_native_values_are_visible_and_unknown_controls_do_not_change_keyboard() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    for native in [
        axis(Axis::LeftStickY, f32::NAN),
        changed(Button::RightTrigger2, 1.01),
    ] {
        assert!(matches!(
            translate(&mut adapter, &mut pending, 0, native, false),
            Err(Error::GamepadInput(incant_input::InputError::Analog))
        ));
    }
    pending
        .push(InputEvent::Key {
            code: KeyCode::Space,
            down: true,
        })
        .unwrap();
    batch(
        &mut adapter,
        &mut pending,
        &[(0, EventType::Dropped), (999, axis(Axis::LeftStickX, 0.5))],
        true,
    );
    let mut input = InputRuntime::default();
    input.advance(pending.events(), DT).unwrap();
    assert!(input.frame().gamepads.is_empty());
    assert!(input.frame().keyboard.held.contains(&KeyCode::Space));
}
#[test]
fn gamepad_bindings_are_accepted_only_for_bounded_player_ids_and_gestures_stay_unsupported() {
    let actions = |value| serde_json::from_value(value).unwrap();
    for id in [0, 15] {
        super::super::input::validate_actions(&actions(serde_json::json!({"move":{"kind":"axis2","bindings":[{"type":"gamepad_stick","id":id,"stick":"left"}]},"fire":{"kind":"button","bindings":[{"type":"gamepad_button","id":id,"button":"right_trigger"}]},"turn":{"kind":"axis1","bindings":[{"type":"gamepad_axis","id":id,"axis":"right_x"}]}}))).unwrap();
    }
    for binding in [
        serde_json::json!({"type":"gamepad_stick","id":16,"stick":"left"}),
        serde_json::json!({"type":"gesture","gesture":"tap"}),
    ] {
        assert!(matches!(
            super::super::input::validate_actions(&actions(
                serde_json::json!({"move":{"kind":"axis2","bindings":[binding]}})
            )),
            Err(Error::InputCapability)
        ));
    }
}

#[test]
fn restart_reannounces_real_adapter_topology_and_release_cannot_become_a_fresh_press() {
    let mut adapter = GamepadAdapter::default();
    let mut pending = PendingInput::new();
    let mut fence = crate::restart::InputFence::default();
    let mut old = InputRuntime::default();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, EventType::Connected),
            (42, changed(Button::Start, 1.)),
            (42, axis(Axis::LeftStickX, 0.7)),
        ],
        true,
    );
    old.advance(fence.filter(pending.events()).unwrap(), DT)
        .unwrap();
    adapter.commit();
    pending.clear();
    fence.replace_session();
    let mut fresh = InputRuntime::default();
    // The native adapter retains its device ID; only the game input owner is new.
    fresh
        .advance(fence.filter(pending.events()).unwrap(), DT)
        .unwrap();
    assert!(fresh.frame().gamepads.contains_key(&0));
    assert!(fresh.frame().gamepads[&0].buttons.held.is_empty());
    let mut ui = super::super::ui_input::UiInput::new();
    ui.defer_surface();
    batch(
        &mut adapter,
        &mut pending,
        &[
            (42, changed(Button::Start, 0.)),
            (42, axis(Axis::LeftStickX, 0.)),
        ],
        ui.accepts_delivery(true, true),
    );
    fresh
        .advance(fence.filter(pending.events()).unwrap(), DT)
        .unwrap();
    assert!(fresh.frame().gamepads[&0].buttons.released.is_empty());
    adapter.commit();
    pending.clear();
    batch(
        &mut adapter,
        &mut pending,
        &[(42, changed(Button::Start, 1.))],
        true,
    );
    fresh
        .advance(fence.filter(pending.events()).unwrap(), DT)
        .unwrap();
    assert!(
        fresh.frame().gamepads[&0]
            .buttons
            .pressed
            .contains(&GamepadButton::Start)
    );
}
