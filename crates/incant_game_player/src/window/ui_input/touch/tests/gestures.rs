use super::*;

#[test]
fn gesture_actions_and_all_four_gestures_replay_exactly() {
    let actions = actions();
    crate::window::input::validate_actions(&actions).unwrap();
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    let mut state = InputRuntime::default();
    let mut packets = vec![];
    let mut observed = vec![];
    let mut step = |events: &[(u64, NativePhase, [f64; 2])]| {
        for &(id, phase, p) in events {
            deliver(&mut input, &mut physical, None, id, phase, p);
        }
        packets.push(physical.events().to_vec());
        state
            .advance_mapped(physical.events(), 1. / 60., &actions)
            .unwrap();
        observed.push(state.frame().clone());
        physical.clear();
        input.accepted();
    };
    step(&[
        (9, NativePhase::Started, [0., 0.]),
        (9, NativePhase::Ended, [2., 3.]),
    ]);
    step(&[(9, NativePhase::Started, [0., 0.])]);
    step(&[
        (9, NativePhase::Moved, [60., 0.]),
        (9, NativePhase::Ended, [70., 0.]),
    ]);
    step(&[(9, NativePhase::Started, [0., 0.])]);
    for _ in 0..31 {
        step(&[]);
    }
    step(&[(9, NativePhase::Ended, [0., 0.])]);
    step(&[
        (9, NativePhase::Started, [0., 0.]),
        (10, NativePhase::Started, [10., 0.]),
    ]);
    step(&[(10, NativePhase::Moved, [20., 0.])]);
    step(&[
        (9, NativePhase::Ended, [0., 0.]),
        (10, NativePhase::Ended, [20., 0.]),
    ]);
    let all = observed
        .iter()
        .flat_map(|f| &f.gestures)
        .collect::<Vec<_>>();
    assert_eq!(
        all.iter()
            .filter(|g| matches!(g, Gesture::Tap { .. }))
            .count(),
        1
    );
    assert_eq!(
        all.iter()
            .filter(|g| matches!(g, Gesture::Swipe { .. }))
            .count(),
        1
    );
    assert_eq!(
        all.iter()
            .filter(|g| matches!(g, Gesture::LongPress { .. }))
            .count(),
        1
    );
    assert_eq!(
        all.iter()
            .filter(|g| matches!(g, Gesture::Pinch { .. }))
            .count(),
        1
    );
    for name in actions.keys() {
        assert!(observed.iter().any(|f| f.actions[name].pressed));
    }
    let text = serde_json::json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":packets.len(),
        "frames": packets.iter().enumerate().map(|(i, events)| serde_json::json!({"tick":i+1,"events":events})).collect::<Vec<_>>() }).to_string();
    let replay =
        crate::window::replay::Replay::parse(&text, 60, Some(packets.len() as u64)).unwrap();
    let mut replayed = InputRuntime::default();
    for (i, expected) in observed.iter().enumerate() {
        replayed
            .advance_mapped(replay.events(i as u64).unwrap(), 1. / 60., &actions)
            .unwrap();
        assert_eq!(replayed.frame(), expected);
    }
}

#[test]
fn unsupported_bindings_still_refuse_before_devices() {
    let mut actions = actions();
    for binding in [
        InputBinding::MouseButton {
            button: MouseButton::Back,
            scale: [1., 0.],
        },
        InputBinding::GamepadAxis {
            id: incant_input::MAX_GAMEPADS as u32,
            axis: incant_input::GamepadAxis::LeftX,
            scale: [1., 0.],
        },
    ] {
        actions.insert(
            "invalid".into(),
            InputAction::Button {
                bindings: vec![binding],
                threshold: 0.5,
            },
        );
        assert!(matches!(
            crate::window::input::validate_actions(&actions),
            Err(Error::InputCapability)
        ));
    }
}
