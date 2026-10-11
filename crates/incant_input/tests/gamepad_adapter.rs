use incant_input::{
    GamepadAxis, GamepadButton, InputError, InputEvent, InputRuntime, MAX_GAMEPADS,
    gamepad::{DeviceEvent, GamepadAdapter},
};
const DT: f64 = 1. / 60.;
fn reconcile(adapter: &mut GamepadAdapter<u64>, events: &mut Vec<InputEvent>) {
    while let Some(event) = adapter.next_connection_event() {
        events.push(event);
    }
}
fn event(
    adapter: &mut GamepadAdapter<u64>,
    events: &mut Vec<InputEvent>,
    id: u64,
    event: DeviceEvent,
    active: bool,
) {
    events.extend(adapter.event(id, event, active).unwrap());
}
#[test]
fn stable_ids_and_ordered_disconnect_reconnect_release_previous_state() {
    let mut adapter = GamepadAdapter::default();
    let mut events = vec![];
    let mut input = InputRuntime::default();
    for device in [u64::MAX, 700] {
        event(
            &mut adapter,
            &mut events,
            device,
            DeviceEvent::Connected,
            true,
        );
    }
    event(
        &mut adapter,
        &mut events,
        u64::MAX,
        DeviceEvent::Button {
            button: GamepadButton::South,
            value: 1.,
        },
        true,
    );
    input.advance(&events, DT).unwrap();
    adapter.commit();
    assert!(
        input.frame().gamepads[&0]
            .buttons
            .held
            .contains(&GamepadButton::South)
    );
    assert_eq!(
        input.frame().connected.iter().copied().collect::<Vec<_>>(),
        [0, 1]
    );
    events.clear();
    event(
        &mut adapter,
        &mut events,
        u64::MAX,
        DeviceEvent::Disconnected,
        true,
    );
    event(
        &mut adapter,
        &mut events,
        u64::MAX,
        DeviceEvent::Connected,
        true,
    );
    event(
        &mut adapter,
        &mut events,
        u64::MAX,
        DeviceEvent::Connected,
        true,
    );
    assert_eq!(
        events,
        [
            InputEvent::GamepadDisconnected { id: 0 },
            InputEvent::GamepadConnected { id: 0 }
        ]
    );
    input.advance(&events, DT).unwrap();
    adapter.commit();
    assert_eq!(input.frame().gamepads.len(), 2);
    assert!(input.frame().gamepads[&0].buttons.held.is_empty());
    assert!(input.frame().connected.contains(&0));
    assert!(input.frame().disconnected.contains(&0));
}
#[test]
fn queue_discard_reconciles_both_unpublished_connect_and_disconnect() {
    let mut adapter = GamepadAdapter::default();
    let mut events = vec![];
    event(&mut adapter, &mut events, 10, DeviceEvent::Connected, true);
    events.clear(); // focus/suspend reset dropped this unpublished connection
    adapter.discard_pending();
    reconcile(&mut adapter, &mut events);
    assert_eq!(events, [InputEvent::GamepadConnected { id: 0 }]);
    let mut input = InputRuntime::default();
    input.advance(&events, DT).unwrap();
    adapter.commit();
    events.clear();
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Disconnected,
        true,
    );
    events.clear(); // published runtime still knows the pad
    adapter.discard_pending();
    reconcile(&mut adapter, &mut events);
    assert_eq!(events, [InputEvent::GamepadDisconnected { id: 0 }]);
    input.advance(&events, DT).unwrap();
    adapter.commit();
    assert!(input.frame().gamepads.is_empty());
    assert!(adapter.next_connection_event().is_none());
}
#[test]
fn focus_and_suspend_release_controls_without_replaying_paused_holds() {
    let mut adapter = GamepadAdapter::default();
    let mut events = vec![];
    let mut input = InputRuntime::default();
    event(&mut adapter, &mut events, 10, DeviceEvent::Connected, true);
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Button {
            button: GamepadButton::RightTrigger,
            value: 0.75,
        },
        true,
    );
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Axis {
            axis: GamepadAxis::LeftX,
            value: 0.9,
        },
        true,
    );
    input.advance(&events, DT).unwrap();
    adapter.commit();
    adapter.discard_pending();
    events = vec![
        InputEvent::Focus { focused: false },
        InputEvent::Focus { focused: true },
    ];
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Axis {
            axis: GamepadAxis::LeftX,
            value: -1.,
        },
        false,
    );
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Button {
            button: GamepadButton::RightTrigger,
            value: 1.,
        },
        false,
    );
    event(&mut adapter, &mut events, 20, DeviceEvent::Connected, false);
    reconcile(&mut adapter, &mut events);
    input.advance(&events, DT).unwrap();
    adapter.commit();
    assert_eq!(input.frame().gamepads[&0].left_stick, [0.; 2]);
    assert_eq!(
        input.frame().gamepads[&0].values[&GamepadButton::RightTrigger],
        0.
    );
    assert!(
        input.frame().gamepads[&0]
            .buttons
            .released
            .contains(&GamepadButton::RightTrigger)
    );
    assert!(input.frame().gamepads.contains_key(&1));
    events.clear();
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Axis {
            axis: GamepadAxis::LeftX,
            value: -0.5,
        },
        true,
    );
    input.advance(&events, DT).unwrap();
    assert_eq!(input.frame().gamepads[&0].left_stick, [-0.5, 0.]);
}
#[test]
fn paused_hotplug_reconciles_current_topology_and_never_fabricates_unknown_pads() {
    let mut adapter = GamepadAdapter::default();
    let mut events = vec![];
    event(&mut adapter, &mut events, 10, DeviceEvent::Connected, false);
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Disconnected,
        false,
    );
    event(
        &mut adapter,
        &mut events,
        900,
        DeviceEvent::Disconnected,
        true,
    );
    event(
        &mut adapter,
        &mut events,
        900,
        DeviceEvent::Axis {
            axis: GamepadAxis::RightX,
            value: 1.,
        },
        true,
    );
    reconcile(&mut adapter, &mut events);
    assert!(events.is_empty());
    event(&mut adapter, &mut events, 10, DeviceEvent::Connected, false);
    event(
        &mut adapter,
        &mut events,
        10,
        DeviceEvent::Button {
            button: GamepadButton::South,
            value: 1.,
        },
        false,
    );
    reconcile(&mut adapter, &mut events);
    assert_eq!(events, [InputEvent::GamepadConnected { id: 0 }]);
}
#[test]
fn distinct_identity_and_analog_limits_are_explicit_even_while_inactive() {
    let mut adapter = GamepadAdapter::default();
    for id in 0..MAX_GAMEPADS as u64 {
        adapter.event(id, DeviceEvent::Connected, false).unwrap();
        adapter.event(id, DeviceEvent::Disconnected, false).unwrap();
    }
    assert_eq!(
        adapter.event(999, DeviceEvent::Connected, false),
        Err(InputError::Devices)
    );
    assert_eq!(
        adapter.event(0, DeviceEvent::Connected, true).unwrap(),
        Some(InputEvent::GamepadConnected { id: 0 })
    );
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.01] {
        assert_eq!(
            adapter.event(
                0,
                DeviceEvent::Button {
                    button: GamepadButton::LeftTrigger,
                    value
                },
                false
            ),
            Err(InputError::Analog)
        );
    }
    for value in [f64::NAN, -1.01, 1.01] {
        assert_eq!(
            adapter.event(
                0,
                DeviceEvent::Axis {
                    axis: GamepadAxis::LeftY,
                    value
                },
                false
            ),
            Err(InputError::Analog)
        );
    }
}
