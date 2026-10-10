#[path = "support/play.rs"]
mod support;
use std::fs;
use support::{fixture, run, success};

#[test]
fn cli_failure_identifies_absolute_tick_after_save_restore_without_partial_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let authored = fs::read(root.join("game.incant.json")).unwrap();
    fs::write(root.join("failure.js"), "exports.default={initialState:{ticks:0},update(api,dt,s){s.ticks++;api.log('tick '+s.ticks);if(s.ticks===5)throw new TypeError('target unavailable');}};").unwrap();
    success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "3",
            "--compiled-script",
            "failure.js",
            "--save-output",
            "start.json",
        ],
    ));
    let saved = fs::read(root.join("start.json")).unwrap();
    let failed = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "3",
            "--compiled-script",
            "failure.js",
            "--load-save",
            "start.json",
            "--save-output",
            "failed.json",
            "--log-output",
            "failed.jsonl",
        ],
    );
    assert!(!failed.status.success());
    let error = String::from_utf8_lossy(&failed.stderr);
    assert!(
        error.contains("play failed at tick 5: script tick failed: TypeError: target unavailable"),
        "{error}"
    );
    assert!(failed.stdout.is_empty());
    assert!(!root.join("failed.json").exists());
    assert_eq!(fs::read(root.join("start.json")).unwrap(), saved);
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), authored);
    let lines: Vec<serde_json::Value> = fs::read_to_string(root.join("failed.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["tick"], 4);
    assert_eq!(lines[0]["message"], "tick 4");
    let legacy = run(
        root,
        &["script", "game.incant.json", "failure.js", "--ticks", "6"],
    );
    assert!(!legacy.status.success());
    assert!(
        String::from_utf8_lossy(&legacy.stderr)
            .contains("play failed at tick 5: script tick failed: TypeError: target unavailable"),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
}

#[test]
fn cli_distinguishes_invalid_initialization_from_tick_timeout() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(root.join("syntax.js"), "exports.default = ;").unwrap();
    fs::write(
        root.join("loop.js"),
        "exports.default={update(){while(true){}}};",
    )
    .unwrap();
    for file in ["syntax.js", "loop.js"] {
        for args in [
            vec![
                "play",
                "game.incant.json",
                "--ticks",
                "1",
                "--compiled-script",
                file,
            ],
            vec!["script", "game.incant.json", file, "--ticks", "1"],
        ] {
            let failed = run(root, &args);
            assert!(!failed.status.success());
            let text = String::from_utf8_lossy(&failed.stderr);
            let expected = if file == "syntax.js" {
                "script initialization failed: SyntaxError:"
            } else {
                "play failed at tick 1: script execution exceeded its 50 ms thread CPU budget"
            };
            assert!(text.contains(expected), "{text}");
            assert!(failed.stdout.is_empty());
        }
    }
}
