use incant_cmd::CommandBus;
use incant_doc::Project;
use incant_script::{LogLevel, ScriptError, ScriptHost};

#[test]
fn committed_logs_preserve_order_and_survive_hot_reload_until_consumed() {
    let mut bus = CommandBus::new(Project::empty("Logs")).unwrap();
    let mut host = ScriptHost::new("exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;api.log('first\\nline');api.log('warning','warn');}};").unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    host.hot_reload("exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;api.log('after reload','debug');}};").unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    let logs = host.take_logs();
    assert_eq!(
        logs.iter().map(|l| l.message.as_str()).collect::<Vec<_>>(),
        ["first\nline", "warning", "after reload"]
    );
    assert_eq!(
        logs.iter().map(|l| l.level).collect::<Vec<_>>(),
        [LogLevel::Info, LogLevel::Warn, LogLevel::Debug]
    );
    assert_eq!(host.state()["ticks"], 2);
    assert!(host.take_logs().is_empty());
    assert!(
        bus.history().is_empty(),
        "logging is output, not a project mutation"
    );
}

#[test]
fn failed_script_transactions_never_publish_their_logs_or_state() {
    let mut bus = CommandBus::new(Project::empty("Logs")).unwrap();
    let original = bus.project().clone();
    let mut host = ScriptHost::new("exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;api.log('must not publish');api.command({op:'delete_scene',scene_id:'missing'});}};").unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
    assert!(host.take_logs().is_empty());
    assert_eq!(host.state()["ticks"], 0);
    assert_eq!(bus.project(), &original);
    for body in [
        "api.log('bad', 'fatal');",
        "api.log({secret:'not a string'});",
        "api.log('x'.repeat(4097));",
        "for(let i=0;i<65;i++)api.log('x');",
    ] {
        let mut host =
            ScriptHost::new(&format!("exports.default={{update(api){{{body}}}}};")).unwrap();
        assert!(host.tick(&mut bus, 1. / 60.).is_err());
        assert!(host.take_logs().is_empty());
    }
    let mut host =
        ScriptHost::new("exports.default={update(api){api.log('é'.repeat(2049));}};").unwrap();
    assert!(matches!(
        host.tick(&mut bus, 1. / 60.),
        Err(ScriptError::LogLimit)
    ));
}

#[test]
fn pending_log_limits_fail_before_state_commit_and_draining_allows_progress() {
    let mut bus = CommandBus::new(Project::empty("Logs")).unwrap();
    let mut host=ScriptHost::new("exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;for(let i=0;i<64;i++)api.log('x');}};").unwrap();
    for _ in 0..4 {
        host.tick(&mut bus, 1. / 60.).unwrap();
    }
    assert!(matches!(
        host.tick(&mut bus, 1. / 60.),
        Err(ScriptError::LogLimit)
    ));
    assert_eq!(host.state()["ticks"], 4);
    assert_eq!(host.take_logs().len(), 256);
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert_eq!(host.state()["ticks"], 5);
    assert_eq!(host.take_logs().len(), 64);
}
