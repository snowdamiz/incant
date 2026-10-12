use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::{Value, json};
use std::{fs, path::Path};
fn run(root: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn setup(root: &Path) -> (String, String) {
    let mut project = Project::empty("Native play");
    project.settings.tick_rate = 10;
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let mut entity = Entity::new("Mover");
    let eid = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[2.,0.,0.]}));
    bus.execute(
        vec![
            Command::SetPhysicsMode {
                mode: incant_runtime::PHYSICS_VARIANT.mode(),
            },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity,
            },
            Command::SetInputActions {
                actions: serde_json::from_value(
                    json!({"right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]}}),
                )
                .unwrap(),
            },
        ],
        Actor::user("test"),
        "Native fixture",
        None,
    )
    .unwrap();
    fs::write(
        root.join("game.incant.json"),
        bus.project().canonical_text().unwrap(),
    )
    .unwrap();
    fs::write(root.join("game.journal.jsonl"), "must not be opened").unwrap();
    fs::write(root.join("replay.json"),json!({"format":"incant-input","version":1,"tick_rate":10,"start_tick":0,"ticks":5,"frames":[{"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]}]}).to_string()).unwrap();
    fs::write(root.join("observe.json"),json!({"format":"incant-native-observation","version":1,"entities":[eid],"checks":[{"name":"move","tick":5,"expect":{"type":"vector","entity":eid,"property":"translation","value":[1.,0.,0.],"tolerance":1e-10}},{"name":"input","tick":1,"expect":{"type":"action","name":"right","active":true,"pressed":true}},{"name":"input later","tick":5,"expect":{"type":"action","name":"right","active":true,"pressed":false}}]}).to_string()).unwrap();
    (sid, eid)
}
fn args() -> Vec<&'static str> {
    vec![
        "native-play",
        "game.incant.json",
        "--ticks",
        "5",
        "--input-replay",
        "replay.json",
        "--observe",
        "observe.json",
        "--sample-rate",
        "44117",
    ]
}
fn unchanged(root: &Path, before: &[u8]) {
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), before);
    assert_eq!(
        fs::read_to_string(root.join("game.journal.jsonl")).unwrap(),
        "must not be opened"
    );
    assert!(!root.join(".incant").exists());
}
#[test]
fn native_project_motion_and_selected_observations_repeat_without_authoring_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (sid, eid) = setup(root);
    let before = fs::read(root.join("game.incant.json")).unwrap();
    let mut command = args();
    command.extend(["--pcm-output", "first.pcm"]);
    let first = success(run(root, &command));
    *command.last_mut().unwrap() = "second.pcm";
    let second = success(run(root, &command));
    assert_eq!(first, second);
    assert_eq!(first["runtime"], "native");
    assert_eq!(first["script_source_parsing"], false);
    assert_eq!(first["scene"], sid);
    assert_eq!(first["tick"], 5);
    assert_eq!(first["frames"], 22058);
    assert_eq!(first["logs"]["entries"].as_array().unwrap().len(), 0);
    assert_eq!(
        first["observations"]["entities"][&eid]["velocity"],
        json!([2., 0., 0.])
    );
    assert_eq!(first["gpu_submitted"], false);
    assert_eq!(first["cpu_render_input"], false);
    let pcm = fs::read(root.join("first.pcm")).unwrap();
    assert_eq!(pcm.len(), 22058 * 8);
    assert!(pcm.iter().all(|b| *b == 0));
    assert_eq!(pcm, fs::read(root.join("second.pcm")).unwrap());
    unchanged(root, &before);
}
#[test]
fn false_assertion_reports_completed_state_and_startup_failure_creates_no_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    let before = fs::read(root.join("game.incant.json")).unwrap();
    let path = root.join("observe.json");
    let mut plan: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    plan["checks"][0]["expect"]["value"] = json!([9., 0., 0.]);
    fs::write(path, plan.to_string()).unwrap();
    let mut command = args();
    command.extend(["--pcm-output", "false.pcm"]);
    let output = run(root, &command);
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["passed"], false);
    assert_eq!(report["tick"], 5);
    assert!(root.join("false.pcm").exists());
    fs::write(root.join("replay.json"), "{}").unwrap();
    *command.last_mut().unwrap() = "failed.pcm";
    let output = run(root, &command);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!root.join("failed.pcm").exists());
    assert_eq!(
        fs::read_dir(root)
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp"))
            .count(),
        0
    );
    unchanged(root, &before);
}
#[test]
fn startup_bounds_unknown_capabilities_and_existing_outputs_fail_explicitly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    let before = fs::read(root.join("game.incant.json")).unwrap();
    for extra in [
        vec!["--seconds", "NaN"],
        vec!["--seconds", "0"],
        vec!["--seconds", "-1"],
        vec!["--ticks", "0"],
        vec!["--ticks", "36001"],
        vec!["--sample-rate", "1"],
        vec!["--scene", "missing"],
        vec!["--camera", "00000000000000000000000001"],
        vec!["--load-save", "old.json"],
        vec!["--compiled-script", "untrusted.js"],
        vec!["--script-sha256", "wrong"],
    ] {
        let mut command = vec![
            "native-play",
            "game.incant.json",
            "--pcm-output",
            "absent.pcm",
        ];
        command.extend(extra);
        let output = run(root, &command);
        assert!(!output.status.success(), "{command:?}");
        assert!(output.stdout.is_empty());
        assert!(!root.join("absent.pcm").exists());
    }
    fs::write(root.join("keep.pcm"), b"keep").unwrap();
    let output = run(
        root,
        &[
            "native-play",
            "game.incant.json",
            "--pcm-output",
            "keep.pcm",
        ],
    );
    assert!(!output.status.success());
    assert_eq!(fs::read(root.join("keep.pcm")).unwrap(), b"keep");
    unchanged(root, &before);
}
#[test]
fn seconds_round_up_and_scene_selection_is_explicit_for_multi_scene_projects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (sid, _) = setup(root);
    let report = success(run(
        root,
        &["native-play", "game.incant.json", "--seconds", "0.21"],
    ));
    assert_eq!(report["tick"], 3);
    let report = success(run(root, &["native-play", "game.incant.json"]));
    assert_eq!(report["tick"], 10);
    let mut project: Project =
        serde_json::from_slice(&fs::read(root.join("game.incant.json")).unwrap()).unwrap();
    let scene = Scene::new("Second");
    project.scenes.insert(scene.id.clone(), scene);
    fs::write(
        root.join("multi.incant.json"),
        project.canonical_text().unwrap(),
    )
    .unwrap();
    let output = run(root, &["native-play", "multi.incant.json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        success(run(
            root,
            &["native-play", "multi.incant.json", "--scene", &sid]
        ))["scene"],
        sid
    );
}

#[path = "../../incant_assets/tests/support/audio.rs"]
mod wave;

fn edit(root: &Path, commands: Vec<Command>) {
    let path = root.join("game.incant.json");
    let project = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut bus = CommandBus::new(project).unwrap();
    bus.execute(commands, Actor::user("test"), "fixture edit", None)
        .unwrap();
    fs::write(path, bus.project().canonical_text().unwrap()).unwrap();
}

#[test]
fn foreign_physics_mode_still_refuses_without_output_or_authoring_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    let mode = match incant_runtime::PHYSICS_VARIANT.mode() {
        incant_doc::PhysicsMode::Deterministic => incant_doc::PhysicsMode::Fast,
        incant_doc::PhysicsMode::Fast => incant_doc::PhysicsMode::Deterministic,
    };
    edit(root, vec![Command::SetPhysicsMode { mode }]);
    let before = fs::read(root.join("game.incant.json")).unwrap();
    let mut command = args();
    command.extend(["--pcm-output", "refused.pcm"]);
    let output = run(root, &command);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!(
            "project physics mode {mode} requires a matching process; compiled kernel is {}",
            incant_runtime::PHYSICS_VARIANT
        ))
    );
    assert!(!root.join("refused.pcm").exists());
    unchanged(root, &before);
}

#[test]
fn native_camera_is_required_for_spatial_content_and_cpu_host_is_reported() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (sid, eid) = setup(root);
    edit(
        root,
        vec![Command::SetComponent {
            scene_id: sid,
            entity_id: eid.clone(),
            component: "Camera".into(),
            value: json!({"fov_degrees":60,"near":0.1,"far":100.,"projection":{"kind":"perspective"}}),
        }],
    );
    let before = fs::read(root.join("game.incant.json")).unwrap();
    let output = run(root, &["native-play", "game.incant.json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--camera"));
    let report = success(run(
        root,
        &["native-play", "game.incant.json", "--camera", &eid],
    ));
    assert_eq!(report["cpu_render_input"], true);
    assert_eq!(report["gpu_submitted"], false);
    unchanged(root, &before);
}

#[test]
fn source_free_streaming_and_static_audio_repeat_then_corruption_refuses_output() {
    use incant_import::{ImportRequest, ImportSnapshot};
    for streaming in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let (sid, eid) = setup(root);
        fs::write(
            root.join("tone.wav"),
            wave::wav(1, 48000, &vec![[8000; 2]; 48000]),
        )
        .unwrap();
        let path = root.join("game.incant.json");
        let mut bus =
            CommandBus::new(Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap())
                .unwrap();
        let imported = ImportSnapshot::capture(&bus)
            .prepare(
                root,
                None,
                &[ImportRequest {
                    source: "tone.wav".into(),
                    texture_usage: None,
                }],
            )
            .unwrap()
            .commit(&mut bus, Actor::import("test"), "Import audio")
            .unwrap();
        bus.execute(vec![Command::SetComponent{scene_id:sid,entity_id:eid,component:"AudioSource".into(),value:json!({"clip":imported.imports[0].asset.id,"bus":null,"gain_db":0.,"pan":0.,"rate":1.,"start_seconds":0.,"playing":true,"looping":true,"streaming":streaming,"spatial":null})}],Actor::user("test"),"Audio",None).unwrap();
        fs::write(&path, bus.project().canonical_text().unwrap()).unwrap();
        fs::remove_file(root.join("tone.wav")).unwrap();
        let before = fs::read(&path).unwrap();
        let a = success(run(
            root,
            &[
                "native-play",
                "game.incant.json",
                "--ticks",
                "5",
                "--pcm-output",
                "a.pcm",
            ],
        ));
        let b = success(run(
            root,
            &[
                "native-play",
                "game.incant.json",
                "--ticks",
                "5",
                "--pcm-output",
                "b.pcm",
            ],
        ));
        assert_eq!(a, b);
        let pcm = fs::read(root.join("a.pcm")).unwrap();
        assert_eq!(pcm.len(), 24000 * 8);
        assert_eq!(pcm, fs::read(root.join("b.pcm")).unwrap());
        let mut nonzero = 0;
        for pair in pcm.as_chunks::<8>().0 {
            let left = f32::from_le_bytes(pair[..4].try_into().unwrap());
            let right = f32::from_le_bytes(pair[4..].try_into().unwrap());
            assert!(left.is_finite() && right.is_finite());
            assert!((left - right).abs() < 1e-6);
            if left.abs() > 0.1 {
                nonzero += 1;
            }
        }
        assert!(nonzero > 23000);
        for cache in fs::read_dir(root.join(".incant/cache/audio")).unwrap() {
            let cache = cache.unwrap().path();
            if cache.is_file() {
                fs::write(cache, b"corrupt").unwrap();
            }
        }
        let output = run(
            root,
            &[
                "native-play",
                "game.incant.json",
                "--pcm-output",
                "absent.pcm",
            ],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!root.join("absent.pcm").exists());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn oversized_plans_and_short_recordings_are_rejected_before_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    fs::write(
        root.join("large.json"),
        vec![b' '; incant_observe::MAX_PLAN_BYTES + 1],
    )
    .unwrap();
    for extra in [
        vec!["--observe", "large.json"],
        vec!["--input-replay", "replay.json"],
    ] {
        let mut command = vec![
            "native-play",
            "game.incant.json",
            "--ticks",
            "10",
            "--pcm-output",
            "absent.pcm",
        ];
        command.extend(extra);
        let output = run(root, &command);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!root.join("absent.pcm").exists());
    }
}

#[test]
fn runtime_overflow_after_accepted_ticks_discards_partial_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (sid, eid) = setup(root);
    edit(
        root,
        vec![
            Command::SetComponent {
                scene_id: sid.clone(),
                entity_id: eid.clone(),
                component: "Transform".into(),
                value: json!(Transform {
                    translation: [1.6e308, 0., 0.],
                    ..Default::default()
                }),
            },
            Command::SetComponent {
                scene_id: sid,
                entity_id: eid,
                component: "Velocity".into(),
                value: json!({"linear":[1e308,0.,0.]}),
            },
        ],
    );
    let before = fs::read(root.join("game.incant.json")).unwrap();
    // One tick is finite, so the later failure happens after real PCM is staged.
    assert_eq!(
        success(run(
            root,
            &["native-play", "game.incant.json", "--ticks", "1"]
        ))["tick"],
        1
    );
    let output = run(
        root,
        &[
            "native-play",
            "game.incant.json",
            "--ticks",
            "5",
            "--pcm-output",
            "absent.pcm",
        ],
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!root.join("absent.pcm").exists());
    assert!(!fs::read_dir(root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tmp")
    }));
    unchanged(root, &before);
}
