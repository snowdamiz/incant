use incant_cmd::CommandBus;
use incant_doc::Project;
use incant_script::{PlaySession, ScriptError, ScriptHost};
use serde_json::{Value, json};

#[test]
fn timers_run_before_update_in_stable_order_with_repeat_replace_cancel_and_isolated_data() {
    let source = r#"exports.default={initialState:{events:[],values:[],updates:[],isolated:true},
      update(api,dt,s){
        const clock=api.clock();s.updates.push(clock.tick);clock.tick=999;
        if(api.clock().tick===999)s.isolated=false;
        if(api.clock().tick===1){
          api.setTimer({id:'b',delay_ticks:2});api.setTimer({id:'a',delay_ticks:2});
          const payload={n:7};api.setTimer({id:'c',delay_ticks:1,interval_ticks:2,payload});payload.n=99;
        }
      },onTimer(api,event,s){
        if(s.updates.length!==api.clock().tick-1)s.isolated=false;
        s.values.push(event.payload?.n??null);s.events.push([event.id,event.scheduled_tick,event.payload]);
        if(event.id==='a')api.cancelTimer('b');
        if(event.id==='c'&&api.clock().tick===4)api.setTimer({id:'c',delay_ticks:1,payload:{n:8}});
        if(event.payload)event.payload.n=-1;
      }};"#;
    let project = Project::empty("Timers");
    let mut play = PlaySession::new(&project, source).unwrap();
    for _ in 0..7 {
        play.tick().unwrap();
    }
    let state = play.host.state();
    // The script retains and mutates its event payload; later delivery still
    // receives the original timer payload, not the previous callback's object.
    assert_eq!(
        state["events"],
        json!([["c",2,{"n":-1}],["a",3,null],["c",4,{"n":-1}],["c",5,{"n":-1}]])
    );
    assert_eq!(state["values"], json!([7, null, 7, 8]));
    assert_eq!(state["updates"], json!([1, 2, 3, 4, 5, 6, 7]));
    assert_eq!(state["isolated"], true);
    assert_eq!(play.host.clock().tick, 7);
    let elapsed = play.snapshot().elapsed_seconds;
    assert_eq!(play.host.clock().elapsed_seconds, elapsed);
}

#[test]
fn timer_callback_failure_preserves_deadline_state_commands_logs_and_clock_for_retry() {
    let project = Project::empty("Atomic timers");
    let mut bus = CommandBus::new(project).unwrap();
    let source = r#"exports.default={initialState:{events:0},update(api){if(api.clock().tick===1)api.setTimer({id:'one',delay_ticks:1})},
      onTimer(api,e,s){s.events++;api.log('not committed');api.command({op:'delete_scene',scene_id:'absent'})}}"#;
    let mut host = ScriptHost::new(source).unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    let before = bus.project().canonical_text().unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
    assert_eq!(host.state(), &json!({"events":0}));
    assert_eq!(host.clock().tick, 1);
    assert!(host.take_logs().is_empty());
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    let fixed = r#"exports.default={initialState:{events:0},update(){},onTimer(api,e,s){s.events++;api.log(e.id)}}"#;
    host.hot_reload(fixed).unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert_eq!(host.clock().tick, 2);
    assert_eq!(host.state(), &json!({"events":1}));
    assert_eq!(host.take_logs()[0].message, "one");
}

#[test]
fn timer_schedule_and_clock_survive_save_reload_with_exact_future_callbacks() {
    let source = r#"exports.default={initialState:{events:[],ticks:0},update(api,dt,s){s.ticks++;
      if(api.clock().tick===1){api.setTimer({id:'repeat',delay_ticks:2,interval_ticks:3,payload:{x:1}});api.setTimer({id:'later',delay_ticks:10});}
      },onTimer(api,e,s){s.events.push([e.id,api.clock().tick,e.payload]);api.log(e.id+':'+e.scheduled_tick)}}"#;
    let project = Project::empty("Saved timers");
    let mut whole = PlaySession::new(&project, source).unwrap();
    for _ in 0..5 {
        whole.tick().unwrap();
        whole.host.take_logs();
    }
    let text = whole.save_text().unwrap();
    let mut restored = PlaySession::from_save(&project, source, &text).unwrap();
    for _ in 0..10 {
        whole.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(whole.host.state(), restored.host.state());
        assert_eq!(whole.save_text().unwrap(), restored.save_text().unwrap());
        assert_eq!(
            json!(whole.host.take_logs()),
            json!(restored.host.take_logs())
        );
    }
    assert_eq!(
        whole.host.state()["events"],
        json!([["repeat",3,{"x":1}],["repeat",6,{"x":1}],["repeat",9,{"x":1}],["later",11,null],["repeat",12,{"x":1}],["repeat",15,{"x":1}]])
    );
    let mut legacy: Value = serde_json::from_str(&text).unwrap();
    legacy["version"] = json!(1);
    legacy.as_object_mut().unwrap().remove("schedule");
    let mut legacy = PlaySession::from_save(&project, source, &legacy.to_string()).unwrap();
    assert_eq!(legacy.host.clock().tick, 5);
    legacy.tick().unwrap();
    assert_eq!(legacy.host.clock().tick, 6);
    assert_eq!(legacy.host.state()["events"].as_array().unwrap().len(), 1);
}

#[test]
fn malformed_schedules_and_handler_removal_are_rejected_without_losing_pending_work() {
    let project = Project::empty("Timer validation");
    let source =
        r#"exports.default={update(api){api.setTimer({id:'one',delay_ticks:5})},onTimer(){}}"#;
    let mut play = PlaySession::new(&project, source).unwrap();
    play.tick().unwrap();
    let before = play.save_text().unwrap();
    assert!(matches!(
        play.host.hot_reload("exports.default={update(){}}"),
        Err(ScriptError::TimerHandler)
    ));
    assert_eq!(play.save_text().unwrap(), before);
    let valid: Value = serde_json::from_str(&before).unwrap();
    for (pointer, value) in [
        ("/schedule", json!(null)),
        ("/schedule/clock/tick", json!(0)),
        ("/schedule/timers/one/due_tick", json!(1)),
        ("/schedule/timers/one/interval_ticks", json!(0)),
        ("/schedule/timers/one/payload", json!("x".repeat(17000))),
        ("/schedule/timers/one/due_tick", json!(9007199254740991_u64)),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            PlaySession::from_save(&project, source, &bad.to_string()).is_err(),
            "{pointer}"
        );
    }
    assert_eq!(play.save_text().unwrap(), before);
}

#[test]
fn invalid_timer_requests_and_limits_cannot_commit_prefix_state_or_project_changes() {
    for body in [
        "api.setTimer({id:'',delay_ticks:1})",
        "api.setTimer({id:'bad space',delay_ticks:1})",
        "api.setTimer({id:'a',delay_ticks:0})",
        "api.setTimer({id:'a',delay_ticks:1.5})",
        "api.setTimer({id:'a',delay_ticks:4294967296})",
        "api.setTimer({id:'a',delay_ticks:1,interval_ticks:0})",
        "api.setTimer({id:'a',delay_ticks:1,payload:'x'.repeat(17000)})",
        "for(let i=0;i<129;i++)api.setTimer({id:'a'+i,delay_ticks:1})",
        "for(let i=0;i<257;i++)api.cancelTimer('a')",
        "for(let i=0;i<8;i++)api.setTimer({id:'a'+i,delay_ticks:1,payload:'x'.repeat(10000)})",
    ] {
        let mut bus = CommandBus::new(Project::empty("Rejected timers")).unwrap();
        let before = bus.project().canonical_text().unwrap();
        let mut host=ScriptHost::new(&format!("exports.default={{initialState:{{n:0}},update(api,dt,s){{s.n++;api.log('prefix');{body}}},onTimer(){{}}}};")).unwrap();
        assert!(host.tick(&mut bus, 1. / 60.).is_err(), "{body}");
        assert_eq!(host.state(), &json!({"n":0}));
        assert_eq!(host.clock().tick, 0);
        assert!(host.take_logs().is_empty());
        assert_eq!(bus.project().canonical_text().unwrap(), before);
    }
}

#[test]
fn promises_and_generators_fail_explicitly_before_committing_synchronous_prefixes() {
    for update in [
        "async update(api,dt,s){s.n++;api.log('prefix');await Promise.resolve();s.n++}",
        "*update(api,dt,s){s.n++;yield 1;s.n++}",
        "update(api,dt,s){s.n++;return {then(){}}}",
        "update(api,dt,s){s.n++;return {next(){}}}",
    ] {
        let mut bus = CommandBus::new(Project::empty("Async rejection")).unwrap();
        let mut host = ScriptHost::new(&format!(
            "exports.default={{initialState:{{n:0}},{update}}}"
        ))
        .unwrap();
        assert!(
            matches!(
                host.tick(&mut bus, 1. / 60.),
                Err(ScriptError::UnsupportedAsync)
            ),
            "{update}"
        );
        assert_eq!(host.state(), &json!({"n":0}));
        assert_eq!(host.clock().tick, 0);
        assert!(host.take_logs().is_empty());
    }
}

#[test]
fn async_timer_callback_cannot_consume_pending_work_or_commit_its_prefix() {
    let mut bus = CommandBus::new(Project::empty("Async timer")).unwrap();
    let mut host = ScriptHost::new(
        r#"exports.default={initialState:{events:0},
      update(api){if(api.clock().tick===1)api.setTimer({id:'pending',delay_ticks:1})},
      async onTimer(api,e,s){s.events++;api.command({op:'set_memory',section:'x',text:'prefix'});
        api.log('prefix');await Promise.resolve();s.events++;}}"#,
    )
    .unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    let before = bus.project().clone();
    assert!(matches!(
        host.tick(&mut bus, 1. / 60.),
        Err(ScriptError::UnsupportedAsync)
    ));
    assert_eq!(bus.project(), &before);
    assert_eq!(host.clock().tick, 1);
    assert_eq!(host.state(), &json!({"events":0}));
    assert!(host.take_logs().is_empty());
    host.hot_reload(
        r#"exports.default={initialState:{events:0},update(){},
      onTimer(api,e,s){s.events++;api.log(e.id)}}"#,
    )
    .unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert_eq!(host.state(), &json!({"events":1}));
    assert_eq!(host.take_logs()[0].message, "pending");
}
