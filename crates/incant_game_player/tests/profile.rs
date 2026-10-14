//! Actual ordinary CLI execution, not a profiler mock. No GPU/audio device.
#![cfg(feature = "profile")]
use incant_runtime::{CookedEntity, CookedScene, StableId};
use incant_types::{Transform, Velocity};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
fn fixture(root: &Path, fail: bool) -> Vec<String> {
    let scene = CookedScene::new(
        StableId(99_u128.to_be_bytes()),
        60,
        vec![CookedEntity {
            id: StableId(1_u128.to_be_bytes()),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        }],
    )
    .unwrap();
    fs::write(root.join("scene"), scene.to_bytes()).unwrap();
    fs::write(
        root.join("clips"),
        [b"NCLIPS01".as_slice(), &[0; 4]].concat(),
    )
    .unwrap();
    let mut frames = vec![json!({"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]})];
    if fail {
        frames.push(json!({"tick":2,"events":[{"type":"key","code":"KeyF","down":true}]}));
    }
    fs::write(root.join("input"),serde_json::to_vec(&json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":3,"frames":frames})).unwrap()).unwrap();
    fs::write(root.join("actions"),serde_json::to_vec(&json!({"right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},"fail":{"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}})).unwrap()).unwrap();
    vec![
        root.join("scene").to_str().unwrap().into(),
        root.to_str().unwrap().into(),
        root.join("clips").to_str().unwrap().into(),
        root.join("input").to_str().unwrap().into(),
        root.join("actions").to_str().unwrap().into(),
        "3".into(),
        "48000".into(),
        root.join("out").to_str().unwrap().into(),
    ]
}
#[test]
fn profiler_reports_real_native_phases_and_preserves_default_gameplay_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let mut reports = Vec::new();
    let mut pcm = Vec::new();
    for (index, mode) in [None, Some("maintained"), Some("detailed")]
        .into_iter()
        .enumerate()
    {
        let root = temp.path().join(index.to_string());
        fs::create_dir(&root).unwrap();
        let mut args = fixture(&root, false);
        if mode.is_some() {
            args.push("--profile".into());
            if mode == Some("detailed") {
                args.push("--profile-heap-detail".into());
            }
        }
        let result = Command::new(env!("CARGO_BIN_EXE_incant_game_player"))
            .args(&args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let mut report: Value = serde_json::from_slice(&result.stdout).unwrap();
        if let Some(heap_mode) = mode {
            // Retain the actual child stdout for protocol-consumer qualification.
            // This CI-profile fixture is not shipping/10k-entity performance proof.
            println!(
                "CLI_PROFILE_REPORT={}",
                String::from_utf8_lossy(&result.stdout).trim()
            );
            let profile = report.as_object_mut().unwrap().remove("profile").unwrap();
            assert_eq!(profile["version"], 3);
            assert_eq!(profile["frame_wall"]["version"], 1);
            assert_eq!(profile["frame_wall"]["attempted_frames"], 3);
            assert_eq!(profile["frame_wall"]["successful_frames"], 3);
            assert_eq!(profile["frame_wall"]["invalid_frames"], 0);
            for name in ["player.frame", "script.tick"] {
                let row = profile["frame_wall"]["phases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["name"] == name)
                    .unwrap();
                assert_eq!(row["samples"], 3);
                assert_eq!(row["missing_frames"], 0);
                assert_eq!(row["invalid_frames"], 0);
                assert_eq!(row["overflow_samples"], 0);
                assert_eq!(
                    row["counts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_u64().unwrap())
                        .sum::<u64>(),
                    3
                );
                let max = profile["zones"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["name"] == name)
                    .unwrap()["totals"]["max_wall_ns"]
                    .as_u64()
                    .unwrap();
                assert!(row["p95_lower_ns"].as_u64().unwrap() <= max);
                assert!(row["p95_upper_ns"].as_u64().unwrap() >= max);
            }
            assert_eq!(profile["completed_frames"], 3);
            assert_eq!(profile["failed_frames"], 0);
            assert_eq!(profile["quickjs_heap"]["samples"], 3);
            assert_eq!(profile["quickjs_heap"]["mode"], heap_mode);
            assert_eq!(profile["quickjs_heap"]["invalid_samples"], 0);
            assert_eq!(
                profile["quickjs_heap"]["latest"]["estimated_used_bytes"].is_null(),
                heap_mode == "maintained"
            );
            assert_eq!(profile["rust_allocations"]["installed"], true);
            assert_eq!(profile["tracy_process_transport"], false);
            for name in [
                "player.frame",
                "world.step",
                "ecs.integrate",
                "ecs.globals",
                "ecs.flipbook",
                "ecs.animation",
                "native.tick",
                "native.input",
                "script.tick",
                "script.source",
                "script.apply",
                "game.audio_sync",
                "game.audio_render",
                "game.host_sync",
            ] {
                let row = profile["zones"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|z| z["name"] == name)
                    .unwrap_or_else(|| panic!("missing {name}: {profile}"));
                assert_eq!(
                    row["totals"]["calls"],
                    if name == "ecs.globals" { 4 } else { 3 },
                    "{name}"
                );
            }
        } else {
            assert!(report.get("profile").is_none());
        }
        reports.push(report);
        pcm.push(fs::read(root.join("out")).unwrap());
    }
    assert_eq!(reports[0], reports[1]);
    assert_eq!(pcm[0], pcm[1]);
    assert_eq!(reports[0], reports[2]);
    assert_eq!(pcm[0], pcm[2]);
    let root = temp.path().join("failure");
    fs::create_dir(&root).unwrap();
    let mut args = fixture(&root, true);
    args.push("--profile".into());
    let result = Command::new(env!("CARGO_BIN_EXE_incant_game_player"))
        .args(&args)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!root.join("out").exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("accepted game input failure"));
}
