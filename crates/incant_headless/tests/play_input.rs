#[path = "support/play.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{fixture, run, success};
const SOURCE: &str = r#"exports.default={initialState:{ticks:0,presses:0,taps:0},update(api,dt,s){
    s.ticks++;const input=api.input();if(input.keyboard.pressed.includes('KeyW'))s.presses++;
    s.taps+=input.gestures.filter(g=>g.type==='tap').length;
    const velocity=[input.keyboard.held.includes('KeyW')?2:0,0,input.gamepads['0']?.left_stick[0]??0];
    for(const e of api.query('Transform'))api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,
        component:'Velocity',value:{linear:velocity}});
    api.log('frame '+s.ticks+' held '+input.keyboard.held.includes('KeyW'));
}};"#;
fn clip() -> Value {
    json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":60,"frames":[
        {"tick":1,"events":[{"type":"key","code":"KeyW","down":true},{"type":"gamepad_connected","id":0},
            {"type":"gamepad_axis","id":0,"axis":"left_x","value":0.75}]},
        {"tick":24,"events":[{"type":"touch","id":1,"phase":"down","position":[10,20]}]},
        {"tick":25,"events":[{"type":"touch","id":1,"phase":"up","position":[10,20]}]},
        {"tick":40,"events":[{"type":"focus","focused":false}]}
    ]})
}

#[test]
fn headless_input_replay_survives_process_restart_without_authoring_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(root.join("behavior.js"), SOURCE).unwrap();
    fs::write(root.join("input.json"), clip().to_string()).unwrap();
    let original = fs::read(root.join("game.incant.json")).unwrap();
    let first = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "19",
            "--compiled-script",
            "behavior.js",
            "--input-replay",
            "input.json",
            "--save-output",
            "checkpoint.json",
        ],
    ));
    assert_eq!(first["input"]["keyboard"]["held"], json!(["KeyW"]));
    let continued = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "41",
            "--compiled-script",
            "behavior.js",
            "--input-replay",
            "input.json",
            "--load-save",
            "checkpoint.json",
        ],
    ));
    let whole = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--compiled-script",
            "behavior.js",
            "--input-replay",
            "input.json",
        ],
    ));
    for key in ["state", "script_state", "input"] {
        assert_eq!(continued[key], whole[key], "{key}");
    }
    assert_eq!(
        continued["logs"],
        json!(&whole["logs"].as_array().unwrap()[19..])
    );
    assert_eq!(
        continued["script_state"],
        json!({"ticks":60,"presses":1,"taps":1})
    );
    assert_eq!(continued["input"]["focused"], false);
    assert_eq!(continued["input"]["keyboard"]["held"], json!([]));
    assert_eq!(continued["frames"], json!([]));
    assert!(continued["adapter"].is_null());
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), original);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}

#[test]
fn invalid_future_events_ranges_and_oversized_files_fail_before_output_is_published() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let mut bad = clip();
    bad["frames"][3]["events"] = json!([{"type":"gamepad_disconnected","id":99}]);
    for recording in [
        bad,
        {
            let mut c = clip();
            c["tick_rate"] = json!(120);
            c
        },
        {
            let mut c = clip();
            c["version"] = json!(2);
            c
        },
        {
            let mut c = clip();
            c["start_tick"] = json!(100);
            c
        },
        json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":0,"frames":[]}),
    ] {
        fs::write(root.join("bad.json"), recording.to_string()).unwrap();
        assert!(
            !run(
                root,
                &[
                    "play",
                    "game.incant.json",
                    "--ticks",
                    "1",
                    "--input-replay",
                    "bad.json",
                    "--save-output",
                    "absent.json",
                    "--log-output",
                    "absent.jsonl"
                ]
            )
            .status
            .success()
        );
        assert!(!root.join("absent.json").exists());
        assert!(!root.join("absent.jsonl").exists());
    }
    fs::write(root.join("valid.json"), clip().to_string()).unwrap();
    assert!(
        !run(
            root,
            &[
                "play",
                "game.incant.json",
                "--ticks",
                "61",
                "--input-replay",
                "valid.json"
            ]
        )
        .status
        .success()
    );
    let file = fs::File::create(root.join("large.json")).unwrap();
    file.set_len(incant_input::MAX_RECORDING_BYTES as u64 + 1)
        .unwrap();
    let failed = run(
        root,
        &["play", "game.incant.json", "--input-replay", "large.json"],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("exceeds"));
}
