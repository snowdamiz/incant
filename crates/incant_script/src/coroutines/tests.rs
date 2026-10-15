use crate::tests::{check, id, world};
use crate::{CompiledScript, NativeSession, ScriptHost};
use std::time::Duration;

fn host(update: &str, resume: &str) -> ScriptHost {
    ScriptHost::with_budget(CompiledScript::compile(&format!(
        "globalThis.trace=[];exports.default={{coroutines:{{step:{{resume(t,task){{{resume}}}}}}},update(t){{{update}}}}};"
    )).unwrap(), Duration::from_secs(1)).unwrap()
}
#[test]
fn coroutine_wait_order_completion_and_current_context_are_observable() {
    let mut host = host(
        "if(t.clock.tick===1){t.coroutines.start('z','step',0);t.coroutines.start('a','step',0)}",
        "trace.push(task.id+':'+t.clock.tick);t.chunks[0].transforms[0]++;if(task.state===0)return {kind:'wait',delayTicks:2,state:1};return {kind:'done'};",
    );
    let mut world = world();
    for _ in 0..5 {
        host.tick(&mut world, 0.25).unwrap();
    }
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
    check(
        &host,
        "if(trace.join(',')!=='a:2,z:2,a:4,z:4')throw Error(trace);",
    );
    assert!(host.schedule.coroutines.is_none());
}
#[test]
fn coroutine_cancel_replacement_never_resurrects_a_superseded_entry() {
    let mut host = host(
        "if(t.clock.tick===1){t.coroutines.start('a','step',0);t.coroutines.start('b','step',0)}",
        "trace.push(task.id+':'+task.state+':'+t.clock.tick); if(task.state===0){t.coroutines.cancel('b');t.coroutines.cancel(task.id);t.coroutines.start(task.id,'step',1);return {kind:'wait',delayTicks:20,state:99}}return {kind:'done'};",
    );
    for _ in 0..4 {
        host.tick(&mut world(), 0.25).unwrap();
    }
    check(
        &host,
        "if(trace.join(',')!=='a:0:2,a:1:3')throw Error(trace);",
    );
    assert!(host.schedule.coroutines.is_none());
}
#[test]
fn coroutine_state_is_owned_frozen_and_previous_tick_capabilities_stay_revoked() {
    let mut host = host(
        "if(t.clock.tick===1){globalThis.first=[7];t.coroutines.start('a','step',first);first[0]=99}",
        "if(task.state[0]!==7||!Object.isFrozen(task.state)||!Object.isFrozen(task))throw Error('state');if(globalThis.old){if(old.view.byteLength!==0)throw Error('loan');let denied=false;try{old.cancel('a')}catch{denied=true}if(!denied)throw Error('capability')}globalThis.old={view:t.chunks[0].transforms,cancel:t.coroutines.cancel};trace.push(t.clock.tick);return t.clock.tick===2?{kind:'wait',delayTicks:1,state:task.state}:{kind:'done'};",
    );
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 0.25).unwrap();
    }
    check(&host, "if(trace.join(',')!=='2,3')throw Error(trace);");
}
#[test]
fn coroutine_invalid_caught_commands_reject_numeric_timer_and_task_prefix() {
    for operation in [
        "t.coroutines.start('', 'step', null)",
        "t.coroutines.start('x', 'missing', null)",
        "t.coroutines.start('x', 'step', NaN)",
        "t.coroutines.start('x', 'step', ()=>1)",
        "t.coroutines.start('x', 'step', {get x(){return 1}})",
        "t.coroutines.start('x', 'step', new Array(2))",
        "t.coroutines.start('x', 'step', 'x'.repeat(16384))",
        "t.coroutines.start('x', 'step', Array(1024).fill(null))",
        "t.coroutines.start('x', 'step', null, t.chunks[0].ids[0])",
        "t.coroutines.start('a', 'step', null)",
        "t.coroutines.cancel('x', 1)",
        "t.coroutines.cancel()",
    ] {
        let mut host = host(
            &format!(
                "t.chunks[0].transforms[0]=99;t.timers.set('valid',1);t.coroutines.start('a','step',null);try{{{operation}}}catch{{}}"
            ),
            "return {kind:'done'}",
        );
        let mut world = world();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.25).is_err(), "{operation}");
        assert_eq!(world.snapshot(), before, "{operation}");
        assert_eq!(host.schedule.tick, 0);
        assert!(host.schedule.coroutines.is_none());
        assert!(host.tick(&mut world, 0.25).is_err());
    }
}
#[test]
fn coroutine_invalid_results_and_deferred_work_roll_back_the_resume() {
    for result in [
        "undefined",
        "{kind:'done',extra:1}",
        "{kind:'wait',delayTicks:0,state:null}",
        "{kind:'wait',delayTicks:1.5,state:null}",
        "{kind:'wait',delayTicks:4294967296,state:null}",
        "{kind:'wait',delayTicks:1,state:Infinity}",
        "{get kind(){return 'done'}}",
        "{kind:'wait',delayTicks:1,state:'x'.repeat(16384)}",
        "Promise.resolve({kind:'done'})",
        "(function*(){yield 1})()",
    ] {
        let mut host = host(
            "if(t.clock.tick===1)t.coroutines.start('a','step',null)",
            &format!("t.chunks[0].transforms[0]=99;return {result}"),
        );
        let mut world = world();
        host.tick(&mut world, 0.25).unwrap();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.25).is_err(), "{result}");
        assert_eq!(world.snapshot(), before, "{result}");
        assert_eq!(host.schedule.tick, 1);
    }
}
#[test]
fn coroutine_task_action_and_retained_state_limits_are_global_and_transactional() {
    for update in [
        "for(let i=0;i<129;i++)t.coroutines.start('t'+i,'step',null)",
        "for(let i=0;i<257;i++){try{t.coroutines.cancel('absent')}catch{}}",
        "for(let i=0;i<17;i++)t.coroutines.start('t'+i,'step','x'.repeat(16000))",
        "for(let i=0;i<128;i++)t.coroutines.start('t'+i,'step',Array(129).fill(null))",
    ] {
        let mut host = host(update, "return {kind:'done'}");
        assert!(host.tick(&mut world(), 0.25).is_err(), "{update}");
        assert!(host.schedule.coroutines.is_none());
    }
    let mut host = host(
        "if(t.clock.tick===1)for(let i=0;i<128;i++)t.coroutines.start('t'+i,'step',null)",
        "t.chunks[0].transforms[0]++;return {kind:'done'}",
    );
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    host.tick(&mut world, 0.25).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 128.);
}
#[test]
fn coroutine_proxy_reentrancy_poison_is_not_lost_and_other_failed_capabilities_stop_resume() {
    let mut host = host(
        "const data=new Proxy({}, {ownKeys(){try{t.coroutines.start('bad','missing',null)}catch{}return []}});try{t.coroutines.start('a','step',data)}catch{}",
        "return {kind:'done'}",
    );
    assert!(host.tick(&mut world(), 0.25).is_err());
    assert!(host.schedule.coroutines.is_none());
    for failure in [
        "try{t.timers.set('',1)}catch{}",
        "try{t.commands.despawn('bad')}catch{}",
        "try{t.state.set(null)}catch{}",
    ] {
        let mut host = host(
            &format!(
                "if(t.clock.tick===1)t.coroutines.start('a','step',null);if(t.clock.tick===2){{{failure}}}"
            ),
            "trace.push('must not run');return {kind:'done'}",
        );
        host.tick(&mut world(), 0.25).unwrap();
        assert!(host.tick(&mut world(), 0.25).is_err());
        check(
            &host,
            "if(trace.length)throw Error('continued after failure');",
        );
    }
}

#[test]
fn coroutine_result_inspection_traps_cannot_hide_failure_or_run_the_next_task() {
    let mut host = host(
        "if(t.clock.tick===1){t.coroutines.start('a','step',null);t.coroutines.start('b','step',null)}",
        "trace.push(task.id);t.chunks[0].transforms[0]=99;return new Proxy({kind:'done'},{ownKeys(){try{t.coroutines.start('bad','missing',null)}catch{}return ['kind']}})",
    );
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    let before = world.snapshot();
    assert!(host.tick(&mut world, 0.25).is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(host.schedule.tick, 1);
    check(
        &host,
        "if(trace.join(',')!=='a')throw Error('continued after result trap');",
    );
}
#[test]
fn coroutine_routine_catalog_is_captured_and_task_free_program_has_no_tick_queue() {
    let source = "const behavior={coroutines:{step:{resume(t){t.chunks[0].transforms[0]++;return {kind:'done'}}}},update(t){if(t.clock.tick===1){behavior.coroutines.step.resume=()=>{throw Error('replacement')};t.coroutines.start('a','step',null)}}};exports.default=behavior;";
    let mut host = ScriptHost::new(CompiledScript::compile(source).unwrap()).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    host.tick(&mut world, 0.25).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 1.);
    let mut plain=ScriptHost::new(CompiledScript::compile("exports.default={update(t){if(t.coroutines!==undefined)throw Error('extra capability')}}").unwrap()).unwrap();
    plain.tick(&mut world, 0.25).unwrap();
    assert!(!plain.coroutine_catalog.enabled());
    assert!(plain.schedule.coroutines.is_none());
}
#[test]
fn coroutine_invalid_input_does_not_advance_or_wake_tasks() {
    let code=CompiledScript::compile("exports.default={coroutines:{step:{resume(t){t.chunks[0].transforms[0]++;return {kind:'done'}}}},update(t){if(t.clock.tick===1)t.coroutines.start('a','step',null)}}").unwrap();
    let scene = incant_runtime::CookedScene::new(
        id(99),
        60,
        vec![incant_runtime::CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let mut session = NativeSession::new(scene, code, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    let before = session.world().unwrap().snapshot();
    assert!(
        session
            .tick(&[incant_input::InputEvent::GamepadAxis {
                id: 7,
                axis: incant_input::GamepadAxis::LeftX,
                value: 0.5
            }])
            .is_err()
    );
    assert!(!session.is_failed());
    assert_eq!(session.world().unwrap().snapshot(), before);
    session.tick(&[]).unwrap();
    assert_eq!(
        session
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        1.
    );
}
