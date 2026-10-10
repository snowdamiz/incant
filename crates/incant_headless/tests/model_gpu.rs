#[path = "support/play.rs"]
mod support;
use std::fs;
use support::{fixture, run, success};

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn headless_play_records_current_simulation_frames_and_publishes_a_complete_report() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let authored = fs::read(root.join("game.incant.json")).unwrap();
    fs::write(
        root.join("logging.js"),
        "exports.default={update(api){api.log('simulated');}};",
    )
    .unwrap();
    let report = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--capture-every",
            "37",
            "--output",
            "frames",
            "--log-output",
            "frames/game.jsonl",
            "--compiled-script",
            "logging.js",
            "--width",
            "320",
            "--height",
            "180",
        ],
    ));
    let ticks: Vec<_> = report["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["tick"].as_u64().unwrap())
        .collect();
    assert_eq!(ticks, [0, 37, 60]);
    assert_eq!(report["completed"], true);
    assert_eq!(report["state"]["tick"], 60);
    assert_eq!(report["logs"].as_array().unwrap().len(), 60);
    let records: Vec<serde_json::Value> = fs::read_to_string(root.join("frames/game.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(serde_json::json!(records), report["logs"]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(root.join("frames/report.json")).unwrap()
        )
        .unwrap(),
        report
    );
    let first = fs::read(root.join("frames/frame-000000.png")).unwrap();
    let last = fs::read(root.join("frames/frame-000060.png")).unwrap();
    assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_ne!(
        first, last,
        "captured pixels must follow simulated positions"
    );
    success(run(
        root,
        &[
            "screenshot",
            "game.incant.json",
            "authored.png",
            "--width",
            "320",
            "--height",
            "180",
        ],
    ));
    assert_eq!(first, fs::read(root.join("authored.png")).unwrap());
    // Resume at a nonzero tick and compare actual pixels to uninterrupted play.
    success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "37",
            "--compiled-script",
            "logging.js",
            "--save-output",
            "checkpoint.json",
        ],
    ));
    let resumed = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "23",
            "--compiled-script",
            "logging.js",
            "--load-save",
            "checkpoint.json",
            "--output",
            "resumed",
            "--width",
            "320",
            "--height",
            "180",
        ],
    ));
    assert_eq!(resumed["start_tick"], 37);
    assert_eq!(resumed["state"], report["state"]);
    assert_eq!(
        resumed["logs"],
        serde_json::json!(&report["logs"].as_array().unwrap()[37..])
    );
    assert_eq!(resumed["frames"][0]["tick"], 37);
    assert_eq!(resumed["frames"][1]["tick"], 60);
    for tick in [37, 60] {
        let name = format!("frame-{tick:06}.png");
        assert_eq!(
            fs::read(root.join("resumed").join(&name)).unwrap(),
            fs::read(root.join("frames").join(name)).unwrap()
        );
    }
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), authored);
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
                "--output",
                "failed",
                "--compiled-script",
                "failure.js",
                "--save-output",
                "failed-save.json"
            ]
        )
        .status
        .success()
    );
    assert!(root.join("failed/frame-000000.png").exists());
    assert!(!root.join("failed-save.json").exists());
    assert!(
        !root.join("failed/report.json").exists(),
        "a partial run must never claim completion"
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn camera_capture_tracks_simulated_camera_motion_and_rejects_unknown_selection() {
    use incant_doc::{Camera, Entity, Project, Transform};
    use serde_json::json;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cube = fixture(root);
    let path = root.join("game.incant.json");
    let mut project = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let entities = &mut project.scenes.values_mut().next().unwrap().entities;
    entities
        .get_mut(&cube)
        .unwrap()
        .components
        .remove("Velocity");
    let mut camera = Entity::new("Moving camera");
    let id = camera.id.clone();
    camera.components.insert(
        "Camera".into(),
        json!(Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 100.
        }),
    );
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 0., 5.],
            ..Default::default()
        }),
    );
    camera
        .components
        .insert("Velocity".into(), json!({"linear":[2,0,0]}));
    entities.insert(id.clone(), camera);
    fs::write(&path, project.canonical_text().unwrap()).unwrap();
    let authored = fs::read(&path).unwrap();
    let journal = fs::read(root.join("game.incant.journal.jsonl")).unwrap();
    let report = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--camera",
            &id,
            "--ticks",
            "60",
            "--output",
            "camera-frames",
            "--width",
            "320",
            "--height",
            "180",
        ],
    ));
    assert_eq!(report["camera"], id);
    let first = fs::read(root.join("camera-frames/frame-000000.png")).unwrap();
    let last = fs::read(root.join("camera-frames/frame-000060.png")).unwrap();
    assert_ne!(
        first, last,
        "camera velocity must affect the frame while geometry stays still"
    );
    let screenshot = success(run(
        root,
        &[
            "screenshot",
            "game.incant.json",
            "camera.png",
            "--camera",
            &id,
            "--width",
            "320",
            "--height",
            "180",
        ],
    ));
    assert_eq!(screenshot["camera"], id);
    assert_eq!(first, fs::read(root.join("camera.png")).unwrap());
    assert_eq!(authored, fs::read(&path).unwrap());
    assert_eq!(
        journal,
        fs::read(root.join("game.incant.journal.jsonl")).unwrap()
    );
    assert!(
        !run(
            root,
            &[
                "screenshot",
                "game.incant.json",
                "missing.png",
                "--camera",
                &cube
            ]
        )
        .status
        .success()
    );
    assert!(!root.join("missing.png").exists());
    assert!(
        !run(
            root,
            &[
                "play",
                "game.incant.json",
                "--camera",
                &cube,
                "--output",
                "missing-frames"
            ]
        )
        .status
        .success()
    );
    assert!(!root.join("missing-frames/report.json").exists());
    assert!(
        !run(root, &["play", "game.incant.json", "--camera", &id])
            .status
            .success()
    );
}
