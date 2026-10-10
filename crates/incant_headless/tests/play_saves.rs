#[path = "support/play.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{fixture, run, success};

const SOURCE: &str = r#"exports.default={initialState:{ticks:0},update(api,dt,s){
  s.ticks++; api.log('tick '+s.ticks);
  for(const e of api.query('Transform')){
    const t=e.components.Transform;t.translation[1]+=3*dt;
    api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'Transform',value:t});
  }
}};"#;

#[test]
fn separate_processes_save_and_resume_with_continuous_state_time_and_logs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(root.join("behavior.js"), SOURCE).unwrap();
    let authored = fs::read(root.join("game.incant.json")).unwrap();
    let first = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "17",
            "--compiled-script",
            "behavior.js",
            "--save-output",
            "slots/one.json",
        ],
    ));
    assert_eq!(first["start_tick"], 0);
    let saved = fs::read(root.join("slots/one.json")).unwrap();
    let envelope: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(envelope["format"], "incant-game-save");
    assert_eq!(envelope["version"], 2);
    assert_eq!(envelope["tick"], 17);
    let resumed = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "23",
            "--compiled-script",
            "behavior.js",
            "--load-save",
            "slots/one.json",
            "--save-output",
            "slots/two.json",
            "--log-output",
            "continued.jsonl",
        ],
    ));
    let uninterrupted = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "40",
            "--compiled-script",
            "behavior.js",
        ],
    ));
    assert_eq!(resumed["start_tick"], 17);
    assert_eq!(resumed["ticks"], 23);
    assert_eq!(resumed["state"], uninterrupted["state"]);
    assert_eq!(resumed["script_state"], uninterrupted["script_state"]);
    assert_eq!(resumed["logs"][0]["tick"], 18);
    assert_eq!(resumed["logs"][0]["message"], "tick 18");
    assert_eq!(resumed["logs"][22]["tick"], 40);
    let lines: Vec<Value> = fs::read_to_string(root.join("continued.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(json!(lines), resumed["logs"]);
    assert_eq!(resumed["frames"], json!([]));
    assert!(resumed["adapter"].is_null());
    let zero = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "0",
            "--compiled-script",
            "behavior.js",
            "--load-save",
            "slots/two.json",
        ],
    ));
    assert_eq!(zero["state"], uninterrupted["state"]);
    assert_eq!(fs::read(root.join("slots/one.json")).unwrap(), saved);
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), authored);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}

#[test]
fn failed_runs_and_invalid_inputs_never_publish_or_replace_a_save() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(
        root.join("failure.js"),
        "exports.default={update(){throw Error('failure')}};",
    )
    .unwrap();
    let failed = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--compiled-script",
            "failure.js",
            "--save-output",
            "failed.json",
        ],
    );
    assert!(!failed.status.success());
    assert!(!root.join("failed.json").exists());
    success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "0",
            "--save-output",
            "valid.json",
        ],
    ));
    let original = fs::read(root.join("valid.json")).unwrap();
    let envelope: Value = serde_json::from_slice(&original).unwrap();
    for invalid in [
        "{".to_string(),
        {
            let mut value = envelope.clone();
            value["version"] = json!(999);
            value.to_string()
        },
        {
            let mut value = envelope;
            value["script_sha256"] = json!("different");
            value.to_string()
        },
    ] {
        fs::write(root.join("invalid.json"), invalid).unwrap();
        assert!(
            !run(
                root,
                &[
                    "play",
                    "game.incant.json",
                    "--load-save",
                    "invalid.json",
                    "--save-output",
                    "absent.json"
                ]
            )
            .status
            .success()
        );
        assert!(!root.join("absent.json").exists());
    }
    for path in [
        "valid.json",
        "game.incant.json",
        "game.incant.journal.jsonl",
    ] {
        let before = fs::read(root.join(path)).unwrap();
        assert!(
            !run(root, &["play", "game.incant.json", "--save-output", path])
                .status
                .success()
        );
        assert_eq!(fs::read(root.join(path)).unwrap(), before);
    }
    assert_eq!(fs::read(root.join("valid.json")).unwrap(), original);
    assert!(
        !run(
            root,
            &[
                "play",
                "game.incant.json",
                "--save-output",
                "same.json",
                "--log-output",
                "./same.json"
            ]
        )
        .status
        .success()
    );
    assert!(!root.join("same.json").exists());
    // Reserved image/report paths fail before GPU setup, even with zero ticks.
    for (folder, file) in [("frames1", "report.json"), ("frames2", "frame-000000.png")] {
        let output = format!("{folder}/{file}");
        let failed = run(
            root,
            &[
                "play",
                "game.incant.json",
                "--ticks",
                "0",
                "--output",
                folder,
                "--save-output",
                &output,
            ],
        );
        assert!(!failed.status.success());
        assert!(String::from_utf8_lossy(&failed.stderr).contains("save output conflicts"));
        assert!(!root.join(output).exists());
    }
    fs::write(root.join("not-utf8.json"), [255]).unwrap();
    assert!(
        !run(
            root,
            &["play", "game.incant.json", "--load-save", "not-utf8.json"]
        )
        .status
        .success()
    );
    let large = fs::File::create(root.join("oversized.json")).unwrap();
    large
        .set_len(incant_script::MAX_SAVE_BYTES as u64 + 1)
        .unwrap();
    let failed = run(
        root,
        &["play", "game.incant.json", "--load-save", "oversized.json"],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("exceeds its size limit"));
}
