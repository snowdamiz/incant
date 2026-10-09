#[path = "support/play.rs"]
mod support;
use std::fs;
use support::{fixture, run, success};

#[test]
fn play_advances_isolated_simulation_and_script_state_without_authoring_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let id = fixture(root);
    let original = fs::read(root.join("game.incant.json")).unwrap();
    let result = success(run(
        root,
        &["play", "game.incant.json", "--seconds", "0.51"],
    ));
    assert_eq!(result["ticks"], 31);
    assert_eq!(result["state"]["tick"], 31);
    assert!(
        (result["state"]["entities"][&id]["translation"][0]
            .as_f64()
            .unwrap()
            - 31. / 30.)
            .abs()
            < 1e-9
    );
    assert_eq!(result["frames"], serde_json::json!([]));
    assert!(result["adapter"].is_null());
    fs::write(root.join("behavior.js"), r#"exports.default = {
        initialState: { ticks: 0 }, update(api, dt, state) {
            state.ticks++;
            for (const e of api.query('Transform')) {
                const t = e.components.Transform;
                t.translation[1] += 3 * dt;
                api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'Transform',value:t});
            }
        }
    };"#).unwrap();
    let scripted = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--compiled-script",
            "behavior.js",
        ],
    ));
    assert_eq!(scripted["script_state"]["ticks"], 60);
    assert_eq!(scripted["script_commands"], 60);
    let translation = &scripted["state"]["entities"][&id]["translation"];
    assert!((translation[0].as_f64().unwrap() - 2.).abs() < 1e-9);
    assert!((translation[1].as_f64().unwrap() - 3.).abs() < 1e-9);
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), original);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}

#[test]
fn play_rejects_unbounded_work_failed_scripts_and_existing_output_paths() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    for args in [
        vec!["--seconds", "NaN"],
        vec!["--seconds=-1"],
        vec!["--ticks", "10001"],
        vec![
            "--ticks",
            "10000",
            "--output",
            "oversized",
            "--capture-every",
            "1",
        ],
        vec![
            "--ticks",
            "7200",
            "--output",
            "oversized",
            "--width",
            "2048",
            "--height",
            "2048",
        ],
    ] {
        let mut command = vec!["play", "game.incant.json"];
        command.extend(args);
        assert!(!run(root, &command).status.success());
        assert!(!root.join("oversized").exists());
    }
    fs::create_dir(root.join("existing")).unwrap();
    fs::write(root.join("existing/report.json"), "keep prior result").unwrap();
    assert!(
        !run(root, &["play", "game.incant.json", "--output", "existing"])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(root.join("existing/report.json")).unwrap(),
        "keep prior result"
    );
    fs::write(
        root.join("failure.js"),
        "exports.default={update(){throw new Error('failure')}};",
    )
    .unwrap();
    assert!(
        !run(
            root,
            &[
                "play",
                "game.incant.json",
                "--compiled-script",
                "failure.js"
            ]
        )
        .status
        .success()
    );
    let zero = success(run(root, &["play", "game.incant.json", "--ticks", "0"]));
    assert_eq!(zero["state"]["tick"], 0);
}
