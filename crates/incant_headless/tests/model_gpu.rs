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
                "failure.js"
            ]
        )
        .status
        .success()
    );
    assert!(root.join("failed/frame-000000.png").exists());
    assert!(
        !root.join("failed/report.json").exists(),
        "a partial run must never claim completion"
    );
}
