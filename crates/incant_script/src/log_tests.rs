use super::*;
use crate::tests::{check, id, world};

fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(&format!(
            "exports.default={{update({{chunks,commands,clock,timers,log}}){{{body}}}}};"
        ))
        .unwrap(),
        Duration::from_secs(1),
    )
    .unwrap()
}

#[test]
fn primitive_logs_preserve_order_levels_unicode_and_accepted_clock() {
    let mut host = host(
        "log('hello');log('雪🙂\\n\\t','debug');log('',undefined);log('careful','warn');log('problem','error');",
    );
    let mut world = world();
    for _ in 0..2 {
        host.tick(&mut world, 0.25).unwrap();
    }
    let logs = host.logs().unwrap();
    assert_eq!(logs.entries().len(), 10);
    assert_eq!(logs.dropped(), 0);
    for (tick, rows) in logs.entries().chunks(5).enumerate() {
        assert!(rows.iter().all(|row| row.tick == tick as u64 + 1));
        assert_eq!(
            rows.iter().map(|r| r.level).collect::<Vec<_>>(),
            vec![
                LogLevel::Info,
                LogLevel::Debug,
                LogLevel::Info,
                LogLevel::Warn,
                LogLevel::Error
            ]
        );
        assert_eq!(rows[1].message, "雪🙂\n\t");
        assert_eq!(rows[2].message, "");
    }
    let json = serde_json::to_value(logs).unwrap();
    assert_eq!(json["entries"][0]["level"], "info");
    assert!(json.get("bytes").is_none());
}

#[test]
fn caught_invalid_logs_reject_numeric_structural_and_schedule_writes() {
    for call in [
        "log()",
        "log(42)",
        "log(null)",
        "log(undefined)",
        "log(new String('x'))",
        "log('x',null)",
        "log('x','INFO')",
        "log('x','info',0)",
        "log('bad\\0')",
        "log('bad\\r')",
        "log('bad\\u001b')",
        "log('bad\\u0085')",
        r"log('\uD800')",
        r"log('\uDC00')",
        r"log('ok\uD800x')",
        r"log('x','\uDC00')",
    ] {
        let mut host = host(&format!(
            "log('pending');chunks[0].transforms[0]=7;commands.setVelocity(chunks[0].ids[0],[3,0,0]);timers.set('pending',1);try{{{call}}}catch{{}}"
        ));
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.25), Err(ScriptError::LogInput)),
            "{call}"
        );
        assert_eq!(world.snapshot(), before, "{call}");
        assert_eq!(host.schedule.tick, 0);
        assert!(host.logs.entries().is_empty());
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    }
}

#[test]
fn log_arguments_never_invoke_guest_coercion() {
    let mut host = host(
        "globalThis.coerced=false;const value={toString(){globalThis.coerced=true;return 'x'},valueOf(){globalThis.coerced=true;return 'x'}};try{log(value)}catch{};try{log('x',value)}catch{}",
    );
    assert!(matches!(
        host.tick(&mut world(), 0.25),
        Err(ScriptError::LogInput)
    ));
    check(
        &host,
        "if(globalThis.coerced)throw Error('guest coercion ran')",
    );
}

#[test]
fn utf8_message_and_update_limits_are_bounded_and_transactional() {
    for body in [
        "log('x'.repeat(1025))",
        "log('🙂'.repeat(257))",
        "log('x'.repeat(8_000_000))",
        "for(let i=0;i<65;i++){try{log('')}catch{}}",
        "for(let i=0;i<17;i++){try{log('x'.repeat(1024))}catch{}}",
    ] {
        let mut host = host(&format!("chunks[0].transforms[0]=7;try{{{body}}}catch{{}}"));
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.25), Err(ScriptError::InputLimit)),
            "{body}"
        );
        assert_eq!(world.snapshot(), before);
        assert!(host.logs.entries().is_empty());
    }
    let mut host = host("for(let i=0;i<16;i++)log('🙂'.repeat(256));for(let i=0;i<48;i++)log('');");
    host.tick(&mut world(), 0.25).unwrap();
    assert_eq!(host.logs().unwrap().entries().len(), 64);
}

#[test]
fn retained_history_caps_count_drops_without_failing_gameplay() {
    for (body, expected, count) in [
        ("for(let i=0;i<64;i++)log('row'+clock.tick)", 1024, 64),
        ("for(let i=0;i<16;i++)log('x'.repeat(1024))", 256, 16),
    ] {
        let mut host = host(body);
        let mut world = world();
        for _ in 0..20 {
            host.tick(&mut world, 0.25).unwrap();
        }
        let logs = host.logs().unwrap();
        assert_eq!(logs.entries().len(), expected);
        assert_eq!(logs.entries().last().unwrap().tick, 16);
        assert_eq!(logs.dropped(), 20 * count - expected as u64);
        assert_eq!(host.schedule.tick, 20);
    }
}

#[test]
fn exception_and_rejected_structural_batch_do_not_publish_logs() {
    for rejection in [
        "throw Error('fail')",
        "commands.setParent(chunks[0].ids[0],chunks[0].ids[0])",
        "return 1",
    ] {
        let mut host = host(&format!(
            "if(clock.tick===1){{log('committed');return;}}log('pending');chunks[0].transforms[0]=7;{rejection}"
        ));
        let mut world = world();
        host.tick(&mut world, 0.25).unwrap();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.25).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(host.logs.entries().len(), 1);
        assert_eq!(host.logs.entries()[0].message, "committed");
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    }
}

#[test]
fn retained_log_functions_never_reactivate_or_capture_later_updates() {
    let mut host = host(
        "if(globalThis.previous){let rejected=false;try{previous('stale')}catch{rejected=true}if(!rejected)throw Error('retained authority');}globalThis.previous=log;log('current');",
    );
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 0.25).unwrap();
    }
    assert_eq!(host.logs().unwrap().entries().len(), 3);
    check(
        &host,
        "let rejected=false;try{previous('outside')}catch{rejected=true}if(!rejected)throw Error('outside authority')",
    );
    assert_eq!(host.logs().unwrap().entries().len(), 3);
}

#[test]
fn rejected_input_does_not_append_logs_and_terminal_session_hides_them() {
    let scene = incant_runtime::CookedScene::new(id(99), 60, vec![]).unwrap();
    let script=CompiledScript::compile("exports.default={update({clock,log}){log('accepted');if(clock.tick===3)throw Error('terminal');}};").unwrap();
    let mut session = NativeSession::new(scene, script, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    assert!(matches!(
        session.tick(&[incant_input::InputEvent::GamepadAxis {
            id: 7,
            axis: incant_input::GamepadAxis::LeftX,
            value: 0.5
        }]),
        Err(SessionError::Input(_))
    ));
    assert_eq!(session.logs().unwrap().entries().len(), 1);
    session.tick(&[]).unwrap();
    assert_eq!(session.logs().unwrap().entries()[1].tick, 2);
    assert!(session.tick(&[]).is_err());
    assert!(matches!(session.logs(), Err(SessionError::Failed)));
}
