use super::*;
use crate::tests::{check, id, world};

fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(&format!(
            "exports.default={{update({{chunks,commands,clock,timers}}){{{body}}}}};"
        ))
        .unwrap(),
        Duration::from_secs(1),
    )
    .unwrap()
}

#[test]
fn timers_follow_accepted_simulation_updates_in_id_order_and_replace_or_cancel() {
    let mut host = host(
        r#"
        if(clock.elapsed_seconds!==clock.tick/4)throw Error('simulation clock');
        const expected={1:[],2:[],3:['a','z'],4:['a'],5:[]};
        if(JSON.stringify(timers.events.map(e=>e.id))!==JSON.stringify(expected[clock.tick]))throw Error('events');
        for(const event of timers.events) {
            if(event.scheduled_tick!==clock.tick)throw Error('due tick');
            chunks[0].transforms[0]++;
        }
        if(clock.tick===1){timers.set('z',2,2);timers.set('a',10);timers.set('a',2);timers.set('removed',1);timers.cancel('removed');}
        if(clock.tick===3){timers.cancel('z');timers.set('a',1);}
    "#,
    );
    let mut world = world();
    for _ in 0..5 {
        host.tick(&mut world, 0.25).unwrap();
    }
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    assert_eq!(world.tick(), 0, "standalone host does not step physics");
    assert_eq!(host.schedule.tick, 5);
}

#[test]
fn periodic_timers_have_no_drift_and_accept_maximum_delay_without_eager_work() {
    let mut host = host(
        r#"
        if(clock.tick===1){timers.set('repeat',2,2);timers.set('far',4294967295);}
        const due=clock.tick>=3 && clock.tick%2===1;
        if(timers.events.length!==Number(due))throw Error('period');
        if(due && timers.events[0].id!=='repeat')throw Error('wrong ID');
    "#,
    );
    let mut world = world();
    for _ in 0..120 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    assert!((host.schedule.elapsed_seconds - 2.).abs() < 1e-12);
}

#[test]
fn caught_invalid_timer_calls_roll_back_all_numeric_structural_and_clock_changes() {
    for call in [
        "timers.set('',1)",
        "timers.set('a'.repeat(65),1)",
        "timers.set('a'.repeat(8_000_000),1)",
        "timers.set('é',1)",
        r"timers.set('\uD800',1)",
        r"timers.set('\uDC00',1)",
        r"timers.set('ok\uD800x',1)",
        r"timers.cancel('\uD800\uDC00')",
        "timers.set('x',0)",
        "timers.set('x',-1)",
        "timers.set('x',1.5)",
        "timers.set('x',4294967296)",
        "timers.set('x',Infinity)",
        "timers.set('x',NaN)",
        "timers.set('x','1')",
        "timers.set('x',1,0)",
        "timers.set('x',1,null)",
        "timers.set('x',1,1,1)",
        "timers.cancel()",
        "timers.cancel('x',1)",
        "timers.set({toString(){throw Error('coercion')}},1)",
    ] {
        let mut host = host(&format!(
            "chunks[0].transforms[0]=7;commands.setVelocity(chunks[0].ids[0],[3,0,0]);timers.set('valid',1);try{{{call}}}catch{{}}"
        ));
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.25), Err(ScriptError::TimerInput)),
            "{call}"
        );
        assert_eq!(world.snapshot(), before, "{call}");
        assert_eq!(host.schedule.tick, 0);
        assert!(matches!(
            host.tick(&mut world, 0.25),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn bounded_timer_storage_and_call_limits_reject_the_whole_update() {
    for body in [
        "for(let i=0;i<129;i++)timers.set('id'+i,1)",
        "for(let i=0;i<257;i++){try{timers.cancel('absent')}catch{}}",
    ] {
        let mut host = host(&format!("chunks[0].transforms[0]=7;{body}"));
        let mut world = world();
        let before = world.snapshot();
        assert!(matches!(
            host.tick(&mut world, 0.25),
            Err(ScriptError::InputLimit)
        ));
        assert_eq!(world.snapshot(), before);
        assert_eq!(host.schedule.tick, 0);
    }
    let mut host = host(
        r#"
        if(clock.tick===1){for(let i=0;i<128;i++)timers.set('id'+i,1);}
        if(clock.tick===2){
            if(timers.events.length!==128)throw Error('maximum events');
            for(let i=0;i<128;i++){timers.set('id'+i,1);timers.cancel('id'+i);}
        }
        if(clock.tick===3 && timers.events.length)throw Error('cancelled');
    "#,
    );
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 0.25).unwrap();
    }
}

#[test]
fn retained_functions_never_reactivate_and_timer_snapshots_do_not_alias_native_state() {
    let mut host = host(
        r#"
        if(globalThis.old){for(const f of old){let rejected=false;try{f()}catch{rejected=true}if(!rejected)throw Error('stale function');}}
        globalThis.old=[()=>timers.set('bad',1),()=>timers.cancel('good')];
        if(clock.tick===1){timers.set('good',1,1);globalThis.firstClock=clock;}
        if(clock.tick>1){
            if(firstClock.tick!==1 || timers.events[0]?.id!=='good')throw Error('alias');
            let readonly=false;try{timers.events[0].id='bad'}catch{readonly=true}
            if(!readonly)throw Error('event was writable');
            timers.events=[];
        }
    "#,
    );
    let mut world = world();
    for _ in 0..4 {
        host.tick(&mut world, 0.25).unwrap();
    }
    check(
        &host,
        "for(const f of old){let rejected=false;try{f()}catch{rejected=true}if(!rejected)throw Error('outside update');}",
    );
}

#[test]
fn rejected_structural_batch_does_not_commit_candidate_timer_schedule() {
    let mut host = host(
        r#"
        if(clock.tick===1){timers.set('good',1);return;}
        if(timers.events[0]?.id!=='good')throw Error('event');
        chunks[0].transforms[0]=7;timers.set('bad',1);
        commands.setParent(chunks[0].ids[0],chunks[0].ids[0]);
    "#,
    );
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.25),
        Err(ScriptError::Structural(_))
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(host.schedule.tick, 1);
    assert_eq!(host.schedule.elapsed_seconds, 0.25);
}

#[test]
fn invalid_native_input_does_not_advance_the_script_clock_or_deliver_timers() {
    let scene = incant_runtime::CookedScene::new(id(99), 60, vec![]).unwrap();
    let script = CompiledScript::compile(
        r#"exports.default={update({clock,timers}){
        if(clock.tick===1)timers.set('ready',1);
        if(clock.tick===2 && timers.events[0]?.id!=='ready')throw Error('lost event');
        if(clock.tick>2)throw Error('rejected input executed script');
    }};"#,
    )
    .unwrap();
    let mut session = NativeSession::new(scene, script, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    for _ in 0..3 {
        assert!(matches!(
            session.tick(&[incant_input::InputEvent::GamepadAxis {
                id: 7,
                axis: incant_input::GamepadAxis::LeftX,
                value: 0.5
            }]),
            Err(SessionError::Input(_))
        ));
    }
    session.tick(&[]).unwrap();
    assert_eq!(session.world().unwrap().tick(), 2);
}

#[test]
fn timer_events_and_clock_use_own_properties_without_invoking_guest_setters() {
    let host = host("");
    check(
        &host,
        r#"Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')},configurable:true});Object.defineProperty(Object.prototype,'id',{set(){throw Error('id setter')},configurable:true});Object.defineProperty(Object.prototype,'tick',{set(){throw Error('clock setter')},configurable:true});"#,
    );
    host.context.with(|ctx| {
        let timers = timers::TickTimers::default();
        let object = timers
            .expose(
                &ctx,
                schedule::Scope::Global,
                &[schedule::Event {
                    scope: schedule::Scope::Global,
                    id: "good".into(),
                    scheduled_tick: 2,
                }],
            )
            .unwrap();
        ctx.globals().set("testTimers", object).unwrap();
        ctx.globals()
            .set(
                "testClock",
                timers::clock(&ctx, &schedule::Schedule::default()).unwrap(),
            )
            .unwrap();
    });
    check(
        &host,
        "if(testTimers.events[0].id!=='good'||testTimers.events[0].scheduled_tick!==2||testClock.tick!==0)throw Error('missing data');",
    );
}
