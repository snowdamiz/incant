use super::*;
use crate::CompiledScript;
use crate::tests::{check as check_script, id};
use incant_input::*;
use incant_runtime::{CookedEntity, CookedScene};
use incant_types::{Transform, Velocity};
use serde_json::json;

fn check(vm: &SessionVm, source: &str) {
    match vm {
        SessionVm::Bytecode(host) => check_script(host, source),
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(_) => unreachable!(),
        #[cfg(feature = "static-module")]
        SessionVm::Static(_) => panic!("this regression expects a bytecode session"),
    }
}

fn scene(rate: u32) -> CookedScene {
    CookedScene::new(
        id(99),
        rate,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        }],
    )
    .unwrap()
}
fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,dt,input,commands}}){{{body}}}}};"
    ))
    .unwrap()
}
fn actions() -> InputActions {
    serde_json::from_value(
        json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]},
        "tap":{"kind":"button","bindings":[{"type":"gesture","gesture":"tap"}]}}),
    )
    .unwrap()
}
fn key(down: bool) -> InputEvent {
    InputEvent::Key {
        code: KeyCode::Space,
        down,
    }
}

#[test]
fn numeric_runtime_failure_is_terminal_after_input_acceptance() {
    let mut bad = scene(1).entities()[0].clone();
    bad.transform.translation[0] = 1e308;
    bad.velocity.as_mut().unwrap().linear[0] = 1e308;
    let scene = CookedScene::new(id(99), 1, vec![bad]).unwrap();
    let mut session =
        NativeSession::new(scene, code("throw Error('script must not run')"), actions()).unwrap();
    assert!(
        matches!(session.tick(&[key(true)]), Err(SessionError::Runtime(
        RuntimeError::Scene(incant_runtime::SceneError::Component(entity))
    )) if entity == id(1))
    );
    assert!(session.is_failed());
    assert!(matches!(session.world(), Err(SessionError::Failed)));
    assert!(matches!(session.input(), Err(SessionError::Failed)));
    assert!(matches!(session.tick(&[]), Err(SessionError::Failed)));
}

#[test]
fn fixed_rate_native_step_precedes_script_and_retained_input_is_owned() {
    for rate in [1, 30, 60, 120, 240] {
        let script = code(
            r#"
            globalThis.count=(globalThis.count||0)+1;
            const c=chunks[0];
            if(Math.abs(c.transforms[0]-count*dt)>1e-12) throw Error('simulation order/clock');
            if(count===1){globalThis.previous=input;globalThis.view=c.transforms;
                if(!input.actions.jump.pressed || !input.actions.jump.released || input.actions.jump.active) throw Error('subtick');
                if(input.keyboard.pressed[0]!=='Space') throw Error('raw key');
            } else {
                if(!previous.actions.jump.pressed || !previous.actions.jump.released) throw Error('snapshot mutated');
                if(input.actions.jump.pressed || input.keyboard.pressed.length) throw Error('stale edge');
                if(view.buffer.byteLength!==0) throw Error('retained loan');
            }
            c.transforms[1]+=input.actions.jump.pressed?3:0;
        "#,
        );
        let mut session = NativeSession::new(scene(rate), script, actions()).unwrap();
        assert_eq!(session.world().unwrap().tick_rate(), rate);
        session.tick(&[key(true), key(false)]).unwrap();
        session.tick(&[]).unwrap();
        assert_eq!(session.world().unwrap().tick(), 2);
        assert_eq!(
            session
                .world()
                .unwrap()
                .inspect(id(1))
                .unwrap()
                .transform
                .translation,
            [2. / f64::from(rate), 3., 0.]
        );
    }
}

#[test]
fn invalid_suffix_preserves_edges_gesture_clock_world_and_vm_then_retry_works() {
    let mut session = NativeSession::new(
        scene(60),
        code("globalThis.calls=(globalThis.calls||0)+1;"),
        actions(),
    )
    .unwrap();
    session
        .tick(&[
            key(true),
            InputEvent::Touch {
                id: 7,
                phase: TouchPhase::Down,
                position: [1., 2.],
            },
        ])
        .unwrap();
    let input = session.input().unwrap().clone();
    let before = session.world().unwrap().snapshot();
    for _ in 0..40 {
        assert!(matches!(
            session.tick(&[
                key(false),
                InputEvent::GamepadAxis {
                    id: 99,
                    axis: GamepadAxis::LeftX,
                    value: 0.5
                }
            ]),
            Err(SessionError::Input(_))
        ));
        assert_eq!(session.input().unwrap(), &input);
        assert_eq!(session.world().unwrap().snapshot(), before);
        assert_eq!(session.world().unwrap().tick(), 1);
        assert!(!session.is_failed());
    }
    session
        .tick(&[
            key(false),
            InputEvent::Touch {
                id: 7,
                phase: TouchPhase::Up,
                position: [1., 2.],
            },
        ])
        .unwrap();
    assert!(session.input().unwrap().actions["jump"].released);
    assert!(session.input().unwrap().actions["tap"].pressed);
    assert!(matches!(
        session.input().unwrap().gestures[0],
        Gesture::Tap { .. }
    ));
    check(
        &session.host,
        "if(calls!==2) throw Error('invalid packets invoked VM');",
    );
    assert!(matches!(
        session.tick(&vec![key(true); MAX_EVENTS_PER_TICK + 1]),
        Err(SessionError::Input(_))
    ));
    assert_eq!(session.world().unwrap().tick(), 2);
}

#[test]
fn rebind_validates_atomically_and_seeds_held_controls_without_press() {
    let mut session = NativeSession::new(scene(60), code(""), actions()).unwrap();
    session.tick(&[key(true)]).unwrap();
    let before = session.input().unwrap().clone();
    let invalid =
        serde_json::from_value(json!({"bad name":{"kind":"button","bindings":[]}})).unwrap();
    assert!(matches!(
        session.rebind(invalid),
        Err(SessionError::Input(_))
    ));
    assert_eq!(session.input().unwrap(), &before);
    let mut next = actions();
    let jump = next.remove("jump").unwrap();
    next.insert("new_jump".into(), jump);
    session.rebind(next).unwrap();
    let action = &session.input().unwrap().actions["new_jump"];
    assert!(action.active && !action.pressed && !action.released);
    assert!(
        session
            .input()
            .unwrap()
            .keyboard
            .pressed
            .contains(&KeyCode::Space)
    );
    assert_eq!(session.world().unwrap().tick(), 1);
    session.tick(&[key(false)]).unwrap();
    assert!(session.input().unwrap().actions["new_jump"].released);
}

#[test]
fn accepted_packet_script_failure_revokes_loans_and_makes_entire_session_unusable() {
    for failure in [
        "throw Error('fail');",
        "c.transforms[0]=NaN;",
        "void new Promise(()=>{});",
        "return 1;",
    ] {
        let mut session = NativeSession::new(
            scene(60),
            code(&format!(
                "const c=chunks[0];globalThis.retained=c.transforms;c.transforms[0]=99;{failure}"
            )),
            actions(),
        )
        .unwrap();
        assert!(matches!(
            session.tick(&[key(true)]),
            Err(SessionError::Script(_))
        ));
        assert!(session.is_failed());
        assert!(matches!(session.world(), Err(SessionError::Failed)));
        assert!(matches!(session.input(), Err(SessionError::Failed)));
        assert!(matches!(session.tick(&[]), Err(SessionError::Failed)));
        assert!(matches!(
            session.rebind(actions()),
            Err(SessionError::Failed)
        ));
        // Internal observation only: public API never releases this partial state.
        assert_eq!(session.world.tick(), 1);
        assert_eq!(
            session.world.inspect(id(1)).unwrap().transform.translation,
            [1. / 60., 0., 0.]
        );
        check(
            &session.host,
            "if(retained.buffer.byteLength!==0) throw Error('live failed loan');",
        );
    }
}
