use super::*;
use incant_input::*;
use serde_json::json;

fn touch(id: u32, phase: TouchPhase, position: [f64; 2]) -> InputEvent {
    InputEvent::Touch {
        id,
        phase,
        position,
    }
}
fn assert_snapshot(host: &ScriptHost, input: &InputRuntime) {
    let actual = host.context.with(|ctx| {
        let snapshot = input::expose(&ctx, input.frame()).unwrap();
        ctx.globals().set("snapshot", snapshot).unwrap();
        ctx.eval::<String, _>("JSON.stringify(snapshot)").unwrap()
    });
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Number(n) => {
                *n = serde_json::Number::from_f64(n.as_f64().unwrap()).unwrap()
            }
            serde_json::Value::Array(v) => v.iter_mut().for_each(normalize),
            serde_json::Value::Object(v) => v.values_mut().for_each(normalize),
            _ => (),
        }
    }
    let mut actual = serde_json::from_str::<serde_json::Value>(&actual).unwrap();
    let mut expected = json!(input.frame());
    normalize(&mut actual);
    normalize(&mut expected);
    assert_eq!(actual, expected);
}
fn host() -> ScriptHost {
    ScriptHost::new(CompiledScript::compile("exports.default={update(){}};").unwrap()).unwrap()
}

#[test]
fn direct_snapshot_matches_shared_schema_for_devices_actions_and_every_gesture() {
    let host = host();
    let mut runtime = InputRuntime::default();
    let bindings = ValidatedActions::new(
        serde_json::from_value(json!({
            "__proto__":{"kind":"button","bindings":[{"type":"key","code":"Space"}]},
            "move":{"kind":"axis2","bindings":[{"type":"gamepad_stick","id":7,"stick":"left"}]},
            "axis":{"kind":"axis1","bindings":[{"type":"gamepad_axis","id":7,"axis":"right_x"}]},
            "tap":{"kind":"button","bindings":[{"type":"gesture","gesture":"tap"}]}
        }))
        .unwrap(),
    )
    .unwrap();
    runtime.map_validated(&bindings);
    assert_snapshot(&host, &runtime);
    let mut events = vec![
        InputEvent::Key {
            code: KeyCode::Space,
            down: true,
        },
        InputEvent::Key {
            code: KeyCode::Space,
            down: false,
        },
        InputEvent::PointerPosition {
            position: [1.5, 2.25],
        },
        InputEvent::PointerMotion { delta: [3., -4.] },
        InputEvent::Wheel {
            delta: [1., 2.],
            unit: WheelUnit::Pixels,
        },
        InputEvent::Wheel {
            delta: [3., 4.],
            unit: WheelUnit::Lines,
        },
        InputEvent::GamepadConnected { id: 7 },
        InputEvent::GamepadAxis {
            id: 7,
            axis: GamepadAxis::LeftY,
            value: -0.5,
        },
        InputEvent::GamepadAxis {
            id: 7,
            axis: GamepadAxis::RightX,
            value: 0.75,
        },
    ];
    for button in [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ] {
        events.push(InputEvent::MouseButton { button, down: true });
    }
    for button in [
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
    ] {
        events.push(InputEvent::GamepadButton {
            id: 7,
            button,
            value: 0.75,
        });
    }
    events.extend([
        touch(1, TouchPhase::Down, [5., 6.]),
        touch(1, TouchPhase::Up, [5., 6.]),
    ]);
    runtime
        .advance_validated(&events, 0.125, &bindings)
        .unwrap();
    assert!(matches!(runtime.frame().gestures[0], Gesture::Tap { .. }));
    assert_snapshot(&host, &runtime);
    runtime
        .advance_validated(
            &[
                touch(1, TouchPhase::Down, [0., 0.]),
                touch(1, TouchPhase::Up, [64., 0.]),
            ],
            0.125,
            &bindings,
        )
        .unwrap();
    assert!(matches!(runtime.frame().gestures[0], Gesture::Swipe { .. }));
    assert_snapshot(&host, &runtime);
    runtime
        .advance_validated(&[touch(1, TouchPhase::Down, [0., 0.])], 0.125, &bindings)
        .unwrap();
    for _ in 0..4 {
        runtime.advance_validated(&[], 0.125, &bindings).unwrap();
    }
    assert!(matches!(
        runtime.frame().gestures[0],
        Gesture::LongPress { .. }
    ));
    assert_snapshot(&host, &runtime);
    runtime
        .advance_validated(&[touch(2, TouchPhase::Down, [16., 0.])], 0.125, &bindings)
        .unwrap();
    runtime
        .advance_validated(&[touch(2, TouchPhase::Move, [0., 32.])], 0.125, &bindings)
        .unwrap();
    assert!(matches!(runtime.frame().gestures[0], Gesture::Pinch { .. }));
    assert_snapshot(&host, &runtime);
    runtime
        .advance_validated(
            &[
                InputEvent::GamepadDisconnected { id: 7 },
                InputEvent::Focus { focused: false },
            ],
            0.125,
            &bindings,
        )
        .unwrap();
    assert_snapshot(&host, &runtime);
}

#[test]
fn snapshot_construction_does_not_call_guest_prototype_accessors() {
    let host = host();
    let mut input = InputRuntime::default();
    input
        .advance(
            &[InputEvent::Key {
                code: KeyCode::KeyA,
                down: true,
            }],
            0.125,
        )
        .unwrap();
    host.context.with(|ctx| {
        ctx.eval::<(),_>(r#"
            globalThis.accesses=0;
            for(const name of ['focused','actions','held','keyboard','0'])
                Object.defineProperty(Object.prototype,name,{get(){++accesses;throw Error('get');},set(){++accesses;throw Error('set');}});
            Object.defineProperty(Array.prototype,'0',{get(){++accesses;throw Error('array get');},set(){++accesses;throw Error('array set');}});
        "#).unwrap();
    });
    host.context.with(|ctx| {
        let snapshot = input::expose(&ctx, input.frame()).unwrap();
        ctx.globals().set("snapshot", snapshot).unwrap();
    });
    tests::check(
        &host,
        "if(accesses!==0 || snapshot.keyboard.held[0]!=='KeyA') throw Error('guest accessor ran');",
    );
}

#[test]
fn expired_snapshot_budget_never_publishes_component_views() {
    let mut host = ScriptHost::new(CompiledScript::compile(r#"
        globalThis.published=0;
        Object.defineProperty(Object.prototype,'transforms',{set(){++published;throw Error('loan created');}});
        exports.default={update(){throw Error('callback ran');}};
    "#).unwrap()).unwrap();
    *host.deadline.lock().unwrap() = budget::ExecutionBudget::new(Duration::ZERO).unwrap();
    let mut world = tests::world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick_with_input(&mut world, 0.125, &InputRuntime::default()),
        Err(ScriptError::ExecutionDeadline { .. })
    ));
    assert_eq!(world.snapshot(), before);
    *host.deadline.lock().unwrap() = budget::ExecutionBudget::new(Duration::from_secs(1)).unwrap();
    tests::check(
        &host,
        "if(published!==0) throw Error('deadline allowed publication');",
    );
    assert!(matches!(
        host.tick(&mut world, 0.125),
        Err(ScriptError::FailedSession)
    ));
}

#[test]
fn maximum_validated_packet_exposes_bounded_raw_and_gesture_arrays() {
    let mut runtime = InputRuntime::default();
    let events: Vec<_> = (0..MAX_EVENTS_PER_TICK / 2)
        .flat_map(|id| {
            [
                touch(id as u32, TouchPhase::Down, [0., 0.]),
                touch(id as u32, TouchPhase::Up, [0., 0.]),
            ]
        })
        .collect();
    runtime.advance(&events, 0.125).unwrap();
    let mut host = ScriptHost::new(
        CompiledScript::compile(
            r#"
        exports.default={update({input}){
            if(input.touch_changes.length!==1024 || input.gestures.length!==512 ||
               input.gestures[511].type!=='tap' || input.gestures[511].id!==511 ||
               Object.keys(input.touches).length!==0) throw Error('bounded packet lost events');
        }};
    "#,
        )
        .unwrap(),
    )
    .unwrap();
    host.tick_with_input(&mut tests::world(), 0.125, &runtime)
        .unwrap();
}
