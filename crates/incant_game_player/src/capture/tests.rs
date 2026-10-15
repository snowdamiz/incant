use super::{Options, output::Output};
use std::io::Write;
fn flags(at: &str, size: &str) -> Vec<String> {
    [
        "--capture-frames",
        "new",
        "--capture-at",
        at,
        "--capture-size",
        size,
    ]
    .map(str::to_owned)
    .to_vec()
}
fn options(at: &str, size: &str) -> Options {
    Options::split_enabled(flags(at, size), true)
        .unwrap()
        .1
        .unwrap()
}

#[test]
fn selections_are_bounded_unique_and_include_initial_final_without_inference() {
    let o = options("3600,0,17", "1280x960");
    assert_eq!(o.steps, [0, 17, 3600]);
    o.admit_ticks(3600).unwrap();
    assert!(o.admit_ticks(3599).is_err());
    assert!(o.complete([0, 17, 3600].into_iter()).is_ok());
    for bad in [vec![0, 17], vec![0, 17, 17], vec![0, 3600, 17]] {
        assert!(o.complete(bad.into_iter()).is_err());
    }
    for at in [
        "",
        "0,0",
        "01,1",
        "-1",
        "1,",
        "3601",
        "18446744073709551616",
        "1.0",
        "1 2",
    ] {
        assert!(
            Options::split_enabled(flags(at, "640x480"), true).is_err(),
            "{at}"
        );
    }
    let many = (0..129)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert!(Options::split_enabled(flags(&many, "1x1"), true).is_err());
    // Four 64MiB frames exactly fit; the fifth refuses before device setup.
    assert!(Options::split_enabled(flags("0,1,2,3", "4096x4096"), true).is_ok());
    assert!(Options::split_enabled(flags("0,1,2,3,4", "4096x4096"), true).is_err());
    for size in [
        "0x1",
        "1x0",
        "4097x1",
        "1x4294967296",
        "1.0x2",
        "1x2x3",
        "+1x2",
    ] {
        assert!(
            Options::split_enabled(flags("0", size), true).is_err(),
            "{size}"
        );
    }
}

#[test]
fn feature_and_route_admission_precede_output_or_package_setup() {
    let root = tempfile::tempdir().unwrap();
    let new = root.path().join("absent");
    let mut args = flags("0,1", "640x480");
    args[1] = new.to_str().unwrap().into();
    let error = Options::split_enabled(args.clone(), false).unwrap_err();
    assert!(error.to_string().contains("frame-capture build feature"));
    assert!(!new.exists());
    for conflict in [
        "--window",
        "--gpu-offscreen",
        "--load-checkpoint",
        "--save-checkpoint",
        "--static-module",
        "--build-info",
        "--record-materials",
        "--prewarm-materials",
    ] {
        let mut bad = args.clone();
        bad.push(conflict.into());
        assert!(Options::split_enabled(bad, true).is_err());
    }
    let mut ordinary = vec!["value".into(); 8];
    ordinary.extend([
        "--cpu-render-input".into(),
        "models".into(),
        "textures".into(),
        "00".into(),
    ]);
    let expected = ordinary.clone();
    ordinary.extend(args);
    let (rest, capture) = Options::split_enabled(ordinary, true).unwrap();
    assert_eq!(rest, expected);
    let capture = capture.unwrap();
    capture.admit_route(&rest, &Default::default()).unwrap();
    assert!(
        capture
            .admit_route(&rest[..8], &Default::default())
            .is_err()
    );
    let checkpoint = crate::checkpoint::Options {
        save: Some("x".into()),
        load: None,
    };
    assert!(capture.admit_route(&rest, &checkpoint).is_err());
    let mut duplicate = flags("0", "1x1");
    duplicate.extend(["--capture-at".into(), "1".into()]);
    assert!(Options::split_enabled(duplicate, true).is_err());
    for key in ["--capture-frames", "--capture-at", "--capture-size"] {
        assert!(Options::split_enabled(vec![key.into()], true).is_err());
    }
}

#[cfg(feature = "frame-capture")]
#[test]
fn actual_player_rejects_material_modes_before_splitters_can_hide_them() {
    let root = tempfile::tempdir().unwrap();
    let frames = root.path().join("frames");
    let material = root.path().join("material.json");
    let sentinel = b"existing untrusted material data";
    std::fs::write(&material, sentinel).unwrap();
    let mut capture = flags("0", "640x480");
    capture[1] = frames.to_str().unwrap().into();
    for option in ["--record-materials", "--prewarm-materials"] {
        for material_first in [false, true] {
            let material_args = vec![option.into(), material.to_str().unwrap().into()];
            let mut args = vec![
                "--package".into(),
                root.path().join("missing-package").to_str().unwrap().into(),
            ];
            let groups = if material_first {
                [&material_args, &capture]
            } else {
                [&capture, &material_args]
            };
            args.extend(groups.into_iter().flatten().cloned());
            let error = crate::run(args, |_| panic!("no completed report")).unwrap_err();
            assert_eq!(
                error.to_string(),
                "selected-frame capture requires the offline bytecode CPU spatial route without checkpoints",
                "{option}, material_first={material_first}"
            );
            assert!(!frames.exists());
            assert_eq!(std::fs::read(&material).unwrap(), sentinel);
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn create_only_publication_retains_partial_and_latches_failures() {
    let root = tempfile::tempdir().unwrap();
    let mut o = options("0,2", "10x10");
    o.directory = root.path().join("frames");
    let mut out = Output::create(&o).unwrap();
    assert!(Output::create(&o).is_err());
    assert!(!o.directory.join("manifest.json").exists());
    let (_, hash) = out
        .publish("frame-0000.png", 100, |f| {
            f.write_all(b"png fixture")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(hash, incant_assets_runtime::sha256(b"png fixture"));
    assert!(
        out.publish("frame-0000.png", 100, |f| {
            f.write_all(b"replace")?;
            Ok(())
        })
        .is_err()
    );
    assert!(out.json("manifest.json", &true).is_err());
    assert_eq!(
        std::fs::read(o.directory.join("frame-0000.png")).unwrap(),
        b"png fixture"
    );
    assert_eq!(std::fs::read_dir(&o.directory).unwrap().count(), 2);
    let mut o2 = options("1", "1x1");
    o2.directory = root.path().join("write-failure");
    let mut out = Output::create(&o2).unwrap();
    assert!(
        out.publish("frame-0001.png", 10, |f| {
            f.write_all(b"partial")?;
            Err("injected failure".into())
        })
        .is_err()
    );
    assert!(out.json("manifest.json", &true).is_err());
    assert_eq!(std::fs::read_dir(&o2.directory).unwrap().count(), 1);
    let mut o3 = options("1", "1x1");
    o3.directory = root.path().join("byte-limit");
    let mut out = Output::create(&o3).unwrap();
    assert!(
        out.publish("frame-0001.png", 2, |f| {
            f.write_all(b"three")?;
            Ok(())
        })
        .is_err()
    );
    assert!(!o3.directory.join("frame-0001.png").exists());
}

#[test]
fn complete_manifest_is_create_only_and_never_replaces_a_collision() {
    let root = tempfile::tempdir().unwrap();
    let mut o = options("0", "1x1");
    o.directory = root.path().join("frames");
    let mut out = Output::create(&o).unwrap();
    out.json("manifest.json", &serde_json::json!({"complete":true}))
        .unwrap();
    assert!(
        out.json("manifest.json", &serde_json::json!({"complete":false}))
            .is_err()
    );
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(o.directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(v["complete"], true);
}

#[cfg(not(feature = "frame-capture"))]
#[test]
fn actual_player_refuses_capture_before_opening_absent_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("frames");
    let error = crate::run(
        vec![
            "--package".into(),
            "missing-package".into(),
            "--capture-frames".into(),
            dest.to_str().unwrap().into(),
            "--capture-at".into(),
            "0".into(),
            "--capture-size".into(),
            "640x480".into(),
        ],
        |_| panic!("no completed report"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("frame-capture build feature"));
    assert!(!dest.exists());
}
