use incant_cmd::CommandBus;
use incant_doc::Project;
use incant_script::{ScriptError, ScriptHost};
use std::time::{Duration, Instant};

fn js_message(error: ScriptError, expected_phase: &str) -> String {
    match error {
        ScriptError::Javascript { phase, message } => {
            assert_eq!(phase, expected_phase);
            message
        }
        error => panic!("expected JavaScript diagnostic, got {error}"),
    }
}

#[test]
fn syntax_and_gameplay_exceptions_identify_phase_without_committing_failed_ticks() {
    let error = ScriptHost::new("exports.default = ;").err().unwrap();
    assert!(js_message(error, "initialization").starts_with("SyntaxError:"));
    let mut host = ScriptHost::new(
        "exports.default={initialState:{n:0},update(api,dt,s){s.n++;api.log('discard');throw new TypeError('missing target');}};",
    ).unwrap();
    let mut bus = CommandBus::new(Project::empty("diagnostics")).unwrap();
    let before = bus.project().clone();
    assert_eq!(
        js_message(host.tick(&mut bus, 1. / 60.).unwrap_err(), "tick"),
        "TypeError: missing target"
    );
    assert_eq!(host.state()["n"], 0);
    assert_eq!(host.clock().tick, 0);
    assert!(host.take_logs().is_empty());
    assert_eq!(bus.project(), &before);
    // Failed hot reload keeps the previous program and state usable.
    let error = host
        .hot_reload("throw new Error('invalid reload');")
        .unwrap_err();
    assert_eq!(js_message(error, "initialization"), "Error: invalid reload");
    assert_eq!(
        js_message(host.tick(&mut bus, 1. / 60.).unwrap_err(), "tick"),
        "TypeError: missing target"
    );
}

#[test]
fn untrusted_exception_text_is_bounded_and_arbitrary_objects_are_not_stringified() {
    let mut bus = CommandBus::new(Project::empty("diagnostics")).unwrap();
    for (expression, prefix) in [
        (
            "'first\\x1b[31m\\x00\\r\\n\\t'+'é'.repeat(2000)",
            "first[31m\n\t",
        ),
        ("new Error('é'.repeat(2000))", "Error: é"),
    ] {
        let mut host = ScriptHost::new(&format!(
            "exports.default={{update(){{throw {expression};}}}};"
        ))
        .unwrap();
        let message = js_message(host.tick(&mut bus, 1. / 60.).unwrap_err(), "tick");
        assert!(message.starts_with(prefix), "{message}");
        assert!(message.ends_with('…'));
        assert!(message.len() <= 1027);
        assert!(!message.contains('\x1b') && !message.contains('\0') && !message.contains('\r'));
    }
    let mut host = ScriptHost::new(
        "let converted=false;exports.default={update(){if(converted)throw 'toString ran';throw {toString(){converted=true;return 'secret';}};}};",
    ).unwrap();
    for _ in 0..2 {
        assert_eq!(
            js_message(host.tick(&mut bus, 1. / 60.).unwrap_err(), "tick"),
            "non-string JavaScript exception"
        );
    }
}

#[test]
fn hostile_error_getters_cannot_escape_the_deadline_or_poison_later_calls() {
    let mut bus = CommandBus::new(Project::empty("diagnostics")).unwrap();
    let mut host = ScriptHost::new(
        "let first=true;exports.default={update(){if(!first)return;first=false;let e=new Error();Object.defineProperty(e,'message',{get(){throw new Error('getter failed');}});throw e;}};",
    ).unwrap();
    assert_eq!(
        js_message(host.tick(&mut bus, 1. / 60.).unwrap_err(), "tick"),
        "Error"
    );
    host.tick(&mut bus, 1. / 60.).unwrap();
    let mut host = ScriptHost::with_budget(
        "exports.default={update(){let e=new Error();Object.defineProperty(e,'message',{get(){while(true){}}});throw e;}};",
        Duration::from_millis(50),
    ).unwrap();
    let start = Instant::now();
    assert!(matches!(
        host.tick(&mut bus, 1. / 60.),
        Err(ScriptError::ExecutionDeadline { budget_ms: 50 })
    ));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(host.clock().tick, 0);
}
