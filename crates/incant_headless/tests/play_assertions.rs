#[path = "support/play.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{fixture, run, success};

fn plan(checks: Value) -> Value {
    json!({"format":"incant-play-assertions","version":1,"checks":checks})
}
fn write(root: &std::path::Path, value: &Value) {
    fs::write(root.join("assertions.json"), value.to_string()).unwrap();
}

#[test]
fn checks_observe_initial_intermediate_and_final_ticks_without_authoring_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let id = fixture(root);
    let original = fs::read(root.join("game.incant.json")).unwrap();
    write(
        root,
        &plan(json!([
            {"name":"arrived","tick":60,"path":format!("/state/entities/{id}/translation/0"),"expect":{"type":"approx","value":2,"tolerance":1e-12}},
            {"name":"initial","tick":0,"path":"/state/tick","expect":{"type":"equals","value":0}},
            {"name":"midpoint","tick":30,"path":format!("/state/entities/{id}/translation/0"),"expect":{"type":"range","min":0.99,"max":1.01}},
            {"name":"no phantom","tick":60,"path":"/state/entities/missing","expect":{"type":"exists","exists":false}},
            {"name":"neutral controls","tick":60,"path":"/input/keyboard/held","expect":{"type":"equals","value":[]}}
        ])),
    );
    let report = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--assertions",
            "assertions.json",
            "--save-output",
            "good.json",
        ],
    ));
    assert_eq!(report["completed"], true);
    assert_eq!(report["passed"], true);
    let results = report["assertions"].as_array().unwrap();
    assert_eq!(
        results
            .iter()
            .map(|x| x["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "initial",
            "midpoint",
            "arrived",
            "no phantom",
            "neutral controls"
        ]
    );
    assert!(results.iter().all(|r| r["passed"] == true));
    assert_eq!(results[3]["actual"], Value::Null);
    assert!(fs::metadata(root.join("good.json")).is_ok());
    assert!(report["adapter"].is_null());
    assert_eq!(report["frames"], json!([]));
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), original);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}

#[test]
fn failed_gameplay_exits_nonzero_with_all_diagnostics_and_never_publishes_a_save() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fixture(root);
    fs::write(root.join("script.js"), "exports.default={initialState:{value:null,text:'é'.repeat(3000),'a/b':{'~key':7}},update(api,dt,s){api.log('completed');}};").unwrap();
    write(
        root,
        &plan(json!([
            {"name":"initial failure","tick":0,"path":"/state/tick","expect":{"type":"equals","value":99}},
            {"name":"missing differs from null","tick":1,"path":"/script_state/missing","expect":{"type":"equals","value":null}},
            {"name":"null exists","tick":1,"path":"/script_state/value","expect":{"type":"exists","exists":true}},
            {"name":"null equality","tick":1,"path":"/script_state/value","expect":{"type":"equals","value":null}},
            {"name":"numeric type required","tick":2,"path":"/script_state/text","expect":{"type":"range","min":0,"max":1}},
            {"name":"escaped path","tick":2,"path":"/script_state/a~1b/~0key","expect":{"type":"equals","value":7}}
        ])),
    );
    let output = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "2",
            "--compiled-script",
            "script.js",
            "--assertions",
            "assertions.json",
            "--log-output",
            "completed.jsonl",
            "--save-output",
            "absent.json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("gameplay assertions failed"));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["completed"], true);
    assert_eq!(report["passed"], false);
    assert_eq!(report["state"]["tick"], 2);
    assert!(report.get("save_output").is_none());
    assert!(!root.join("absent.json").exists());
    let checks = report["assertions"].as_array().unwrap();
    assert_eq!(
        checks
            .iter()
            .map(|c| c["passed"].as_bool().unwrap())
            .collect::<Vec<_>>(),
        [false, false, true, true, false, true]
    );
    assert!(checks[1]["actual"].is_null());
    assert_eq!(checks[2]["actual"]["json"], "null");
    assert_eq!(checks[4]["actual"]["truncated"], true);
    assert!(checks[4]["actual"]["json"].as_str().unwrap().len() <= 1027);
    assert_eq!(
        fs::read_to_string(root.join("completed.jsonl"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn invalid_assertion_plans_fail_before_creating_outputs_or_running_gameplay() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fixture(root);
    let valid = plan(
        json!([{"name":"check","tick":1,"path":"/state/tick","expect":{"type":"equals","value":1}}]),
    );
    for (path, value) in [
        ("/version", json!(2)),
        ("/format", json!("other")),
        ("/checks", json!([])),
        ("/checks/0/tick", json!(3)),
        ("/checks/0/name", json!(" ")),
        ("/checks/0/path", json!("/state/bad~2")),
        ("/checks/0/path", json!("/files")),
        (
            "/checks/0/expect",
            json!({"type":"approx","value":0,"tolerance":-1}),
        ),
        ("/checks/0/expect", json!({"type":"range","min":2,"max":1})),
        (
            "/checks/0/expect",
            json!({"type":"shell","command":"never"}),
        ),
        (
            "/checks/0/expect",
            json!({"type":"equals","value":1,"unexpected":true}),
        ),
        ("/checks", json!([valid["checks"][0], valid["checks"][0]])),
        ("/checks", json!(vec![valid["checks"][0].clone(); 257])),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(path).unwrap() = value;
        write(root, &bad);
        let output = run(
            root,
            &[
                "play",
                "game.incant.json",
                "--ticks",
                "2",
                "--assertions",
                "assertions.json",
                "--output",
                "absent-frames",
                "--log-output",
                "absent.log",
                "--save-output",
                "absent.save",
            ],
        );
        assert!(!output.status.success(), "{path}");
        assert!(output.stdout.is_empty());
        assert!(!root.join("absent-frames").exists());
        assert!(!root.join("absent.log").exists());
        assert!(!root.join("absent.save").exists());
    }
    let file = fs::File::create(root.join("assertions.json")).unwrap();
    file.set_len(256 * 1024 + 1).unwrap();
    let output = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "0",
            "--assertions",
            "assertions.json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds"));
}

#[test]
fn restored_checkpoint_assertions_use_absolute_ticks_and_reconstructed_input() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fixture(root);
    fs::write(root.join("input.json"),json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":60,"frames":[
        {"tick":1,"events":[{"type":"key","code":"KeyW","down":true}]},
        {"tick":40,"events":[{"type":"key","code":"KeyW","down":false}]}]}).to_string()).unwrap();
    success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "30",
            "--input-replay",
            "input.json",
            "--save-output",
            "checkpoint.json",
        ],
    ));
    write(
        root,
        &plan(json!([
            {"name":"saved clock","tick":30,"path":"/state/tick","expect":{"type":"equals","value":30}},
            {"name":"held restored","tick":30,"path":"/input/keyboard/held","expect":{"type":"equals","value":["KeyW"]}},
            {"name":"released once","tick":40,"path":"/input/keyboard/released","expect":{"type":"equals","value":["KeyW"]}},
            {"name":"not sticky","tick":41,"path":"/input/keyboard/released","expect":{"type":"equals","value":[]}},
            {"name":"finished","tick":60,"path":"/state/tick","expect":{"type":"equals","value":60}}
        ])),
    );
    let report = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "30",
            "--load-save",
            "checkpoint.json",
            "--input-replay",
            "input.json",
            "--assertions",
            "assertions.json",
        ],
    ));
    assert_eq!(report["passed"], true);
    assert_eq!(report["assertions"].as_array().unwrap().len(), 5);
    assert_eq!(report["start_tick"], 30);
}

#[test]
fn json_numeric_equality_accepts_integral_floats_without_rounding_large_integers() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(root.join("numbers.js"), "exports.default={initialState:{whole:2,big:9007199254740992,nested:{n:[-2,0,2]}},update(){}};").unwrap();
    write(
        root,
        &plan(json!([
            {"name":"integral float","tick":0,"path":"/script_state/whole","expect":{"type":"equals","value":2.0}},
            {"name":"recursive numbers","tick":0,"path":"/script_state/nested","expect":{"type":"equals","value":{"n":[-2.0,-0.0,2.0]}}},
            {"name":"distinct large integer","tick":0,"path":"/script_state/big","expect":{"type":"equals","value":9007199254740993_u64}}
        ])),
    );
    let output = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "0",
            "--compiled-script",
            "numbers.js",
            "--assertions",
            "assertions.json",
        ],
    );
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["passed"].as_bool().unwrap())
            .collect::<Vec<_>>(),
        [true, true, false]
    );
}
