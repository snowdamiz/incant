#[path = "support/play.rs"]
mod support;
use serde_json::Value;
use std::fs;
use support::{fixture, run, success};

#[test]
fn failed_timer_callback_withholds_report_and_save_but_preserves_prior_logs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let authored = fs::read(root.join("game.incant.json")).unwrap();
    fs::write(root.join("timer.js"), r#"exports.default={initialState:{events:0},
      update(api){api.log('tick '+api.clock().tick);if(api.clock().tick===1)api.setTimer({id:'fail',delay_ticks:2})},
      async onTimer(api,e,s){s.events++;api.log('must not publish');await Promise.resolve();}}"#).unwrap();
    let prefix = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "2",
            "--compiled-script",
            "timer.js",
            "--save-output",
            "prior.save.json",
        ],
    ));
    assert_eq!(prefix["script_state"]["events"], 0);
    let prior_save = fs::read(root.join("prior.save.json")).unwrap();
    let output = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "10",
            "--compiled-script",
            "timer.js",
            "--save-output",
            "failed.save.json",
            "--log-output",
            "prior.jsonl",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("async/Promise"));
    assert!(output.stdout.is_empty());
    assert!(!root.join("failed.save.json").exists());
    assert_eq!(fs::read(root.join("prior.save.json")).unwrap(), prior_save);
    let logs: Vec<Value> = fs::read_to_string(root.join("prior.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0]["message"], "tick 1");
    assert_eq!(logs[1]["message"], "tick 2");
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), authored);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}
