use incant_runtime::{CookedEntity, CookedScene, StableId};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn copied_player_runs_scene_and_structural_behavior_without_script_arguments() {
    let temporary = Temporary(std::env::temp_dir().join(format!(
            "incant-embedded-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir(&temporary.0).unwrap();
    let binary = temporary.0.join(if cfg!(windows) {
        "player.exe"
    } else {
        "player"
    });
    fs::copy(env!("CARGO_BIN_EXE_incant_script_player"), &binary).unwrap();
    let scene = CookedScene::new(
        StableId(999u128.to_be_bytes()),
        60,
        vec![CookedEntity {
            id: StableId(1u128.to_be_bytes()),
            parent: None,
            transform: incant_types::Transform::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .to_bytes();
    let scene_path = temporary.0.join("world.scene");
    fs::write(&scene_path, &scene).unwrap();
    // A neighboring supplied executable cache must have no influence. The
    // process is launched with a scene/tick count only, never a script path.
    let hostile = temporary.0.join("behavior.qjs");
    fs::write(&hostile, [0xff; 1024]).unwrap();
    let execute = |args: &[&str]| {
        Command::new(&binary)
            .args(args)
            .current_dir(&temporary.0)
            .output()
            .unwrap()
    };
    let info = execute(&["--build-info"]);
    assert!(info.status.success());
    assert!(
        String::from_utf8(info.stdout)
            .unwrap()
            .contains("source_parsing=disabled")
    );
    for (ticks, entities) in [(0, 1), (1, 2), (3, 2), (4, 1), (60, 1)] {
        let args = ["world.scene", &ticks.to_string()];
        let first = execute(&args);
        let second = execute(&args);
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        assert_eq!(first.stdout, second.stdout);
        let text = String::from_utf8(first.stdout).unwrap();
        assert!(text.starts_with(&format!("tick={ticks} entities={entities}\n")));
        let position = text.lines().nth(1).unwrap().rsplit_once('[').unwrap().1;
        let x: f64 = position.split(',').next().unwrap().parse().unwrap();
        assert!((x - f64::from(ticks) / 60.).abs() < 1e-12);
    }
    assert_eq!(fs::read(&scene_path).unwrap(), scene);
    let extra = execute(&["world.scene", "1", "behavior.qjs"]);
    assert!(!extra.status.success() && extra.stdout.is_empty());
    fs::write(&scene_path, b"damaged scene").unwrap();
    let damaged = execute(&["world.scene", "1"]);
    assert!(!damaged.status.success() && damaged.stdout.is_empty());
}

#[test]
fn recording_boundary_rejects_incompatible_unbounded_or_conflicting_inputs_before_play() {
    let temporary = Temporary(std::env::temp_dir().join(format!(
        "incant-input-player-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    fs::create_dir(&temporary.0).unwrap();
    let scene = CookedScene::new(StableId(999u128.to_be_bytes()), 60, vec![])
        .unwrap()
        .to_bytes();
    fs::write(temporary.0.join("world.scene"), scene).unwrap();
    fs::write(temporary.0.join("actions.json"), "{}").unwrap();
    let original = serde_json::json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":4,"frames":[
        {"tick":1,"events":[{"type":"key","code":"Space","down":true}]},
        {"tick":2,"events":[{"type":"key","code":"Space","down":false}]}]});
    let execute = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_incant_script_player"))
            .args(["world.scene", "4"])
            .args(extra)
            .current_dir(&temporary.0)
            .output()
            .unwrap()
    };
    let args = [
        "--input-recording",
        "input.json",
        "--input-actions",
        "actions.json",
    ];
    for (field, value) in [("tick_rate", 30), ("start_tick", 1), ("ticks", 2)] {
        let mut invalid = original.clone();
        invalid[field] = value.into();
        fs::write(temporary.0.join("input.json"), invalid.to_string()).unwrap();
        let result = execute(&args);
        assert!(
            !result.status.success() && result.stdout.is_empty(),
            "{field}"
        );
    }
    let mut invalid = original.clone();
    invalid["frames"][1]["events"] =
        serde_json::json!([{"type":"gamepad_axis","id":7,"axis":"left_x","value":0.5}]);
    fs::write(temporary.0.join("input.json"), invalid.to_string()).unwrap();
    assert!(!execute(&args).status.success());
    fs::write(temporary.0.join("input.json"), original.to_string()).unwrap();
    let result = execute(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .starts_with("tick=4 entities=0\n")
    );
    fs::write(
        temporary.0.join("actions.json"),
        r#"{"bad name":{"kind":"button","bindings":[]}}"#,
    )
    .unwrap();
    let result = execute(&args);
    assert!(!result.status.success() && result.stdout.is_empty());
    fs::write(temporary.0.join("actions.json"), vec![b' '; 256 * 1024 + 1]).unwrap();
    let result = execute(&args);
    assert!(!result.status.success() && result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("input file exceeds player limit")
    );
    fs::write(temporary.0.join("actions.json"), "{}").unwrap();
    fs::write(
        temporary.0.join("input.json"),
        vec![b' '; incant_input::MAX_RECORDING_BYTES + 1],
    )
    .unwrap();
    let result = execute(&args);
    assert!(!result.status.success() && result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("input file exceeds player limit")
    );
    for extra in [
        vec!["--input-recording", "input.json"],
        vec![
            "--input-recording",
            "input.json",
            "--input-recording",
            "input.json",
        ],
        vec![
            "--input-recording",
            "input.json",
            "--input-actions",
            "actions.json",
            "--event",
            "key",
        ],
    ] {
        let result = execute(&extra);
        assert!(!result.status.success() && result.stdout.is_empty());
        assert!(String::from_utf8(result.stderr).unwrap().contains("usage:"));
    }
}
