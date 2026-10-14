//! Real standalone process/file continuation through the ordinary player CLI.
#![cfg(feature = "native-checkpoints")]
use incant_runtime::{CookedEntity, CookedPhysics, CookedPhysics2D, CookedScene, StableId};
use incant_types::{Collider, Collider2D, ColliderShape, ColliderShape2D, RigidBody, Transform};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn fixture(root: &Path) {
    let scene = CookedScene::new(
        id(99),
        144,
        (2..=5)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                velocity: None,
                transform: Transform {
                    translation: [0., if n % 2 == 0 { -0.5 } else { 1. }, 0.],
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap()
    .with_physics(
        (2..=3)
            .map(|n| CookedPhysics {
                entity: id(n),
                body: (n == 3).then(RigidBody::default),
                angular_velocity: None,
                collider: Collider {
                    shape: ColliderShape::Box {
                        half_extents: if n == 3 { [0.4; 3] } else { [4., 0.5, 4.] },
                    },
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap()
    .with_physics_2d(
        (4..=5)
            .map(|n| CookedPhysics2D {
                entity: id(n),
                body: (n == 5).then(RigidBody::default),
                angular_velocity: None,
                collider: Collider2D {
                    shape: ColliderShape2D::Box {
                        half_extents: if n == 5 { [0.4; 2] } else { [4., 0.5] },
                    },
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap();
    fs::write(root.join("scene"), scene.to_bytes()).unwrap();
    fs::write(
        root.join("clips"),
        [b"NCLIPS01".as_slice(), &[0; 4]].concat(),
    )
    .unwrap();
    fs::write(root.join("actions"), b"{}").unwrap();
    fs::write(root.join("input"), serde_json::to_vec(&json!({
        "format":"incant-input", "version":1,"start_tick":0,"tick_rate":144,"ticks":104,"frames":[]
    })).unwrap()).unwrap();
}
fn run(root: &Path, stem: &str, ticks: u64, load: Option<&str>, save: &str, rate: u32) -> Output {
    // Evidence collectors may retain the matching CLI beside the immediate test
    // executable before building a different numerical feature configuration.
    let bin = std::env::var_os("INCANT_TEST_PLAYER")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_incant_game_player").into());
    let mut cmd = Command::new(bin);
    for path in [
        root.join("scene"),
        root.to_path_buf(),
        root.join("clips"),
        root.join("input"),
        root.join("actions"),
    ] {
        cmd.arg(path);
    }
    cmd.arg(ticks.to_string())
        .arg(rate.to_string())
        .arg(root.join(format!("{stem}.pcm")));
    if let Some(load) = load {
        cmd.arg("--load-checkpoint").arg(root.join(load));
    }
    cmd.arg("--save-checkpoint").arg(root.join(save));
    cmd.output().unwrap()
}
fn success(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn separate_processes_resume_warm_physics_and_fractional_pcm_at_the_saved_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let full = success(run(root, "full", 104, None, "full.save", 44117));
    let first = success(run(root, "first", 67, None, "first.save", 44117));
    let resumed = success(run(
        root,
        "resumed",
        37,
        Some("first.save"),
        "resumed.save",
        44117,
    ));
    assert_eq!(full["tick"], 104);
    assert_eq!(first["tick"], 67);
    assert_eq!(resumed["tick"], 104);
    assert_eq!(
        first["frames"].as_u64().unwrap() + resumed["frames"].as_u64().unwrap(),
        full["frames"].as_u64().unwrap()
    );
    let expected = fs::read(root.join("full.pcm")).unwrap();
    let mut joined = fs::read(root.join("first.pcm")).unwrap();
    joined.extend(fs::read(root.join("resumed.pcm")).unwrap());
    assert_eq!(joined, expected);
    assert_eq!(
        fs::read(root.join("full.save")).unwrap(),
        fs::read(root.join("resumed.save")).unwrap()
    );
}

#[test]
fn malformed_or_mismatched_files_publish_no_new_output_and_never_overwrite_a_save() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    success(run(root, "first", 13, None, "first.save", 44117));
    let original = fs::read(root.join("first.save")).unwrap();
    let mut bad = original.clone();
    *bad.last_mut().unwrap() ^= 1;
    fs::write(root.join("corrupt-input.save"), &bad).unwrap();
    for (stem, input, rate) in [
        ("corrupt", "corrupt-input.save", 44117),
        ("rate", "first.save", 48000),
    ] {
        let out = run(root, stem, 3, Some(input), &format!("{stem}.save"), rate);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!root.join(format!("{stem}.pcm")).exists());
        assert!(!root.join(format!("{stem}.save")).exists());
    }
    assert_eq!(fs::read(root.join("corrupt-input.save")).unwrap(), bad);
    let duplicate = run(
        root,
        "duplicate",
        3,
        Some("first.save"),
        "first.save",
        44117,
    );
    assert!(!duplicate.status.success());
    assert_eq!(fs::read(root.join("first.save")).unwrap(), original);
    assert!(!fs::read_dir(root).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".incant-")
    }));
}
