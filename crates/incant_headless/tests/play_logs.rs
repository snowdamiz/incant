#[path = "support/play.rs"]
mod support;
use std::fs;
use support::{fixture, run, success};

#[test]
fn play_captures_ordered_script_logs_without_gpu_or_authored_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let project = fs::read(root.join("game.incant.json")).unwrap();
    fs::write(root.join("logs.js"), "exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;api.log('tick '+state.ticks);if(state.ticks===2)api.log('a\\nb','warn');}};").unwrap();
    let result = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "3",
            "--compiled-script",
            "logs.js",
            "--log-output",
            "logs/run.jsonl",
        ],
    ));
    assert!(result["adapter"].is_null());
    let legacy = success(run(
        root,
        &["script", "game.incant.json", "logs.js", "--ticks", "300"],
    ));
    assert_eq!(legacy["logs"].as_array().unwrap().len(), 301);
    assert_eq!(legacy["state"]["ticks"], 300);
    let records = result["logs"].as_array().unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(
        records
            .iter()
            .map(|r| r["tick"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [1, 2, 2, 3]
    );
    assert_eq!(records[0]["level"], "info");
    assert_eq!(records[2]["level"], "warn");
    assert_eq!(records[2]["message"], "a\nb");
    assert_eq!(records[3]["elapsed_seconds"], 3. / 60.);
    let log = fs::read_to_string(root.join("logs/run.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        &lines, records,
        "newlines inside a message must remain in one JSONL record"
    );
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), project);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
    assert!(
        !run(
            root,
            &["play", "game.incant.json", "--log-output", "logs/run.jsonl"]
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read_to_string(root.join("logs/run.jsonl")).unwrap(),
        log
    );
    for (i, name) in ["report.json", "frame-000000.png", "REPORT.JSON"]
        .into_iter()
        .enumerate()
    {
        let output = format!("reserved-{i}");
        let path = format!("{output}/{name}");
        let failed = run(
            root,
            &[
                "play",
                "game.incant.json",
                "--output",
                &output,
                "--log-output",
                &path,
            ],
        );
        assert!(!failed.status.success());
        assert!(String::from_utf8_lossy(&failed.stderr).contains("log output conflicts"));
        assert!(
            !root.join(path).exists(),
            "the log must not claim a reserved capture filename"
        );
    }
}

#[test]
fn failed_play_keeps_committed_logs_and_excessive_logging_fails_boundedly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(root.join("fail.js"),"exports.default={initialState:{ticks:0},update(api,dt,state){state.ticks++;api.log('tick '+state.ticks);if(state.ticks===2)throw new Error('failed');}};").unwrap();
    let failed = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "3",
            "--compiled-script",
            "fail.js",
            "--log-output",
            "failed.jsonl",
        ],
    );
    assert!(!failed.status.success());
    assert!(
        failed.stdout.is_empty(),
        "failed runs never emit a completed report"
    );
    let lines: Vec<serde_json::Value> = fs::read_to_string(root.join("failed.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["message"], "tick 1");
    fs::write(
        root.join("flood.js"),
        "exports.default={update(api){for(let i=0;i<64;i++)api.log('x');}};",
    )
    .unwrap();
    let failed = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "200",
            "--compiled-script",
            "flood.js",
            "--log-output",
            "flood.jsonl",
        ],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("script log capture exceeds"));
    let lines: Vec<serde_json::Value> = fs::read_to_string(root.join("flood.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines.len(),
        9984,
        "the overflowing tick must not publish a partial batch"
    );
    assert_eq!(lines.last().unwrap()["tick"], 156);
    fs::write(
        root.join("byte-flood.js"),
        "exports.default={update(api){for(let i=0;i<64;i++)api.log('x'.repeat(4096));}};",
    )
    .unwrap();
    let failed = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "100",
            "--compiled-script",
            "byte-flood.js",
            "--log-output",
            "byte-flood.jsonl",
        ],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("script log capture exceeds"));
    let bytes = fs::read(root.join("byte-flood.jsonl")).unwrap();
    assert!(bytes.len() > 7 * 1024 * 1024 && bytes.len() <= 8 * 1024 * 1024);
    assert_eq!(
        bytes
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .count()
            % 64,
        0
    );
}
