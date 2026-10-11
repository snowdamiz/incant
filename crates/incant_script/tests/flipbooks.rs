#![cfg(feature = "compiler")]
#[path = "../../incant_runtime/tests/support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_script::{CompiledScript, ScriptError, ScriptHost};
use std::time::Duration;
const ONE: &str = "00000000000000000000000001";
fn host(source: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(source).unwrap(),
        Duration::from_secs(2),
    )
    .unwrap()
}
#[test]
fn bulk_snapshot_is_committed_owned_and_aligned_across_native_ticks_and_controls() {
    let mut host = host(
        r#"
        let tick=0, old, loan;
        exports.default={update({chunks,flipbooks,commands}) {
            const i=flipbooks.ids.indexOf('00000000000000000000000001');
            if(i<0 || flipbooks.ids.length!==2 || flipbooks.frames.length!==2 || flipbooks.flags.length!==2)throw Error('alignment');
            if(loan && loan.byteLength!==0)throw Error('retained loan');
            loan=chunks[0].transforms;
            const expected=[[0,1],[0,0],[2,0],[2,1],[0,1],[1,1]][tick];
            if(flipbooks.frames[i]!==expected[0] || flipbooks.flags[i]!==expected[1])throw Error('snapshot state '+tick);
            if(tick===0) { commands.controlFlipbook(flipbooks.ids[i],'pause'); old=flipbooks; }
            if(tick===1) { if(old.frames[i]!==0 || old.flags[i]!==1)throw Error('old mutated'); old.frames[i]=88; try{old.ids[i]='bad';}catch(_){} commands.controlFlipbook(flipbooks.ids[i],'seek',2); }
            if(tick===2) commands.controlFlipbook(flipbooks.ids[i],'play');
            if(tick===3) commands.controlFlipbook(flipbooks.ids[i],'restart');
            if(flipbooks.frames[i]!==expected[0] || flipbooks.flags[i]!==expected[1])throw Error('queued readback');
            tick++;
        }};
    "#,
    );
    let mut world = NativeWorld::new(scene(2));
    for step_count in [1, 1, 1, 1, 1, 3] {
        for _ in 0..step_count {
            world.step().unwrap();
        }
        host.tick(&mut world, 1. / 64.).unwrap();
    }
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 1);
}
#[test]
fn caught_expired_control_never_reactivates_and_does_not_poison_current_tick() {
    let mut host = host(
        r#"
        let old, tick=0;
        exports.default={update({commands}) {
            if(old) {let threw=false;try{old('00000000000000000000000001','seek',99);}catch(_){threw=true;}if(!threw)throw Error('expired accepted');}
            if(tick===0) {old=commands.controlFlipbook;commands.controlFlipbook('00000000000000000000000001','pause');}
            else commands.controlFlipbook('00000000000000000000000001','seek',tick%3);
            tick++;
        }};
    "#,
    );
    let mut world = NativeWorld::new(scene(1));
    for frame in [0, 1, 2, 0, 1] {
        host.tick(&mut world, 1. / 64.).unwrap();
        assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), frame);
    }
}
#[test]
fn caught_invalid_active_control_restores_numeric_and_all_ordered_commands() {
    for args in [
        "'pause',1",
        "'seek'",
        "'seek',-1",
        "'seek',0.5",
        "'seek',NaN",
        "'seek',65536",
        "'seek','1'",
        "'unknown'",
        "{toString(){throw Error('coercion');}}",
        "'\\ud800'",
    ] {
        let source = format!(
            r#"exports.default={{update({{chunks,commands}}){{
            chunks[0].transforms[0]=55;
            commands.controlFlipbook('{ONE}','pause');
            commands.setVelocity('{ONE}',[7,0,0]);
            try{{commands.controlFlipbook('{ONE}',{args});}}catch(_){{}}
        }}}};"#
        );
        let mut host = host(&source);
        let mut world = NativeWorld::new(scene(1));
        let before = world.snapshot();
        let state = world.flipbook_state(id(1));
        assert!(
            matches!(
                host.tick(&mut world, 1. / 64.),
                Err(ScriptError::CommandInput)
            ),
            "{args}"
        );
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.flipbook_state(id(1)), state);
        assert!(matches!(
            host.tick(&mut world, 1. / 64.),
            Err(ScriptError::FailedSession)
        ));
    }
}
#[test]
fn final_native_control_rejection_and_callback_failure_restore_whole_transaction() {
    for tail in [
        format!("commands.controlFlipbook('{ONE}','seek',3);"),
        format!("commands.despawn('{ONE}');"),
        "throw Error('late');".into(),
        "return 1;".into(),
        "void new Promise(()=>{});".into(),
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{chunks,commands}}){{chunks[0].transforms[0]=55;commands.controlFlipbook('{ONE}','pause');{tail}}}}};"#
        ));
        let mut world = NativeWorld::new(scene(1));
        world.step().unwrap();
        let before = world.snapshot();
        let state = world.flipbook_state(id(1));
        assert!(host.tick(&mut world, 1. / 64.).is_err(), "{tail}");
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.flipbook_state(id(1)), state);
    }
}
#[test]
fn bulk_snapshot_bypasses_prototype_setters_and_handles_many_entities_once() {
    let mut host = host(
        r#"
        let once=false;
        exports.default={update({flipbooks,commands}) {
            if(!once) {Object.defineProperty(Object.prototype,'frames',{set(){throw Error('object setter');}, configurable:true});once=true;}
            if(flipbooks.ids.length!==4096 || flipbooks.frames.length!==4096 || flipbooks.flags.length!==4096)throw Error('bulk count');
            const unique=new Set(flipbooks.ids);if(unique.size!==4096)throw Error('duplicate');
            for(let i=0;i<flipbooks.ids.length;i++) if(flipbooks.frames[i]!==0 || flipbooks.flags[i]!==1)throw Error('payload');
            commands.controlFlipbook(flipbooks.ids[4095],'seek',0);
        }};
    "#,
    );
    let mut world = NativeWorld::new(scene(4096));
    host.tick(&mut world, 1. / 64.).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
}
#[test]
fn accepted_input_clock_is_retryable_before_script_and_terminal_after_control_failure() {
    use incant_input::{GamepadAxis, InputAction, InputBinding, InputEvent, KeyCode};
    use incant_script::{NativeSession, SessionError};
    let actions = [
        (
            "pause".into(),
            InputAction::Button {
                bindings: vec![InputBinding::Key {
                    code: KeyCode::KeyP,
                    scale: [1., 0.],
                }],
                threshold: 0.5,
            },
        ),
        (
            "invalid".into(),
            InputAction::Button {
                bindings: vec![InputBinding::Key {
                    code: KeyCode::KeyI,
                    scale: [1., 0.],
                }],
                threshold: 0.5,
            },
        ),
    ]
    .into();
    let code = CompiledScript::compile(
        r#"exports.default={update({input,commands,flipbooks}){
        if(input.actions.pause.pressed)commands.controlFlipbook(flipbooks.ids[0],'pause');
        if(input.actions.invalid.pressed)commands.controlFlipbook(flipbooks.ids[0],'seek',3);
    }};"#,
    )
    .unwrap();
    let mut game = NativeSession::new(scene(1), code, actions).unwrap();
    assert!(matches!(
        game.tick(&[
            InputEvent::Key {
                code: KeyCode::KeyP,
                down: true
            },
            InputEvent::GamepadAxis {
                id: 99,
                axis: GamepadAxis::LeftX,
                value: 0.5
            }
        ]),
        Err(SessionError::Input(_))
    ));
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(
        game.world()
            .unwrap()
            .flipbook_state(id(1))
            .unwrap()
            .playing()
    );
    game.tick(&[InputEvent::Key {
        code: KeyCode::KeyP,
        down: true,
    }])
    .unwrap();
    assert!(game.input().unwrap().actions["pause"].pressed);
    assert_eq!(game.world().unwrap().tick(), 1);
    assert!(
        !game
            .world()
            .unwrap()
            .flipbook_state(id(1))
            .unwrap()
            .playing()
    );
    assert!(
        game.tick(&[InputEvent::Key {
            code: KeyCode::KeyI,
            down: true
        }])
        .is_err()
    );
    assert!(game.is_failed());
    assert!(game.world().is_err());
    assert!(game.input().is_err());
    assert!(game.logs().is_err());
}
#[test]
fn caught_flipbook_command_quota_is_atomic_with_the_shared_queue() {
    let mut host = host(&format!(
        r#"exports.default={{update({{chunks,commands}}){{
        chunks[0].transforms[0]=7;
        try{{for(let i=0;i<{};i++)commands.controlFlipbook('{ONE}','pause');}}catch(_){{}}
    }}}};"#,
        MAX_FRAME_COMMANDS + 1
    ));
    let mut world = NativeWorld::new(scene(1));
    let before = world.snapshot();
    let state = world.flipbook_state(id(1));
    assert!(matches!(
        host.tick(&mut world, 1. / 64.),
        Err(ScriptError::InputLimit)
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.flipbook_state(id(1)), state);
}
#[test]
fn snapshot_tracks_committed_despawn_and_an_empty_world_without_stale_rows() {
    let mut host = host(
        r#"let turn=0;exports.default={update({flipbooks,commands}){
        if(flipbooks.ids.length!==2-turn)throw Error('stale membership');
        if(turn<2)commands.despawn(flipbooks.ids[0]);turn++;
    }};"#,
    );
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..3 {
        host.tick(&mut world, 1. / 64.).unwrap();
    }
    assert_eq!(world.entity_count(), 0);
}
#[test]
fn finished_once_state_is_bulk_observable_and_restart_clears_completion() {
    let mut host = host(
        r#"exports.default={update({flipbooks,commands}){
        const i=flipbooks.ids.indexOf('00000000000000000000000002');
        if(flipbooks.frames[i]!==2 || flipbooks.flags[i]!==2)throw Error('once completion');
        commands.controlFlipbook(flipbooks.ids[i],'restart');
    }};"#,
    );
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..12 {
        world.step().unwrap();
    }
    host.tick(&mut world, 1. / 64.).unwrap();
    let state = world.flipbook_state(id(2)).unwrap();
    assert_eq!(state.frame(), 0);
    assert!(state.playing());
    assert!(!state.completed());
}
#[test]
fn retained_bulk_copies_are_limited_by_vm_memory_and_allocation_failure_keeps_world() {
    let mut host = host(
        r#"const saved=[];exports.default={update({flipbooks,chunks}){saved.push(flipbooks);chunks[0].transforms[0]+=1;}};"#,
    );
    let mut world = NativeWorld::new(scene(4096));
    let mut failed = false;
    for _ in 0..512 {
        let before = world.snapshot();
        match host.tick(&mut world, 1. / 64.) {
            Ok(()) => {}
            Err(error) => {
                assert!(
                    matches!(
                        error,
                        ScriptError::MemoryAllocation | ScriptError::Javascript { .. }
                    ),
                    "{error:?}"
                );
                assert_eq!(world.snapshot(), before);
                assert!(matches!(
                    host.tick(&mut world, 1. / 64.),
                    Err(ScriptError::FailedSession)
                ));
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "retained snapshots escaped the VM memory cap");
}
