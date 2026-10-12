use super::*;
use incant_assets_runtime::sha256;
use incant_runtime::{CookedEntity, CookedScene, StableId};
use incant_types::{Transform, Velocity};
use serde_json::json;
use std::collections::BTreeMap;

fn fixture(root: &Path) -> serde_json::Value {
    let package = root.join("package");
    fs::create_dir(&package).unwrap();
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
    let replay = json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":3,"frames":[
        {"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]}]});
    let files = [
        ("world.scene", scene.to_bytes()),
        ("clips.bin", [b"NCLIPS01".as_slice(), &[0; 4]].concat()),
        (
            "input-actions.json",
            serde_json::to_vec(&json!({
                "right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},
                "fail":{"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}
            }))
            .unwrap(),
        ),
        ("empty-input.json", serde_json::to_vec(&replay).unwrap()),
    ];
    let mut hashes = BTreeMap::new();
    for (name, bytes) in files {
        hashes.insert(name, sha256(&bytes));
        fs::write(package.join(name), bytes).unwrap();
    }
    fs::write(
        package.join("package.json"),
        serde_json::to_vec(&json!({
            "format":"incant-native-package","version":1,"camera":null,"files":hashes
        }))
        .unwrap(),
    )
    .unwrap();
    let observation = root.join("observe.json");
    fs::write(&observation, serde_json::to_vec(&json!({"format":"incant-native-observation","version":1,"entities":["00000000000000000000000001"],"checks":[{
        "name":"same native movement","tick":3,"expect":{"type":"vector","entity":"00000000000000000000000001", "property":"translation","value":[0.1,0,0],"tolerance":1e-12}
    }]})).unwrap()).unwrap();
    json!({"version":1,"package":package,"output_directory":root.join("run"),"ticks":3,"observations":observation})
}

#[test]
fn ffi_and_library_share_actual_cooked_gameplay_and_create_only_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = fixture(temp.path());
    let data = serde_json::to_vec(&request).unwrap();
    // SAFETY: this immutable byte allocation is alive throughout the call.
    let status = unsafe { incant_game_run_v1(data.as_ptr(), data.len()) };
    let result: Report =
        serde_json::from_slice(&fs::read(temp.path().join("run/result.json")).unwrap()).unwrap();
    assert_eq!(status, 0, "{result:?}");
    assert!(result.passed);
    let report = result.game.unwrap();
    assert_eq!(report["tick"], 3);
    assert_eq!(report["observations"]["passed"], true);
    assert_eq!(report["frames"], 2400);
    let pcm = fs::read(temp.path().join("run/audio.pcm")).unwrap();
    assert_eq!(pcm, vec![0; 2400 * 8]);
    assert_eq!(report["pcm_sha256"], sha256(&pcm));
    assert!(matches!(execute(&data), Err(MobileError::Io(_))));
    assert_eq!(fs::read(temp.path().join("run/audio.pcm")).unwrap(), pcm);
    request["output_directory"] = json!(temp.path().join("repeat"));
    assert_eq!(
        execute(&serde_json::to_vec(&request).unwrap())
            .unwrap()
            .game
            .unwrap(),
        report
    );
}

#[test]
fn invalid_requests_and_corrupt_packages_never_publish_success_or_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    for (key, value) in [
        ("version", json!(2)),
        ("ticks", json!(0)),
        ("ui_viewport", json!([0, 720])),
        ("package", json!("relative")),
        ("output_directory", json!("/bad\npath")),
        ("window", json!(true)),
    ] {
        let mut bad = request.clone();
        bad[key] = value;
        assert!(execute(&serde_json::to_vec(&bad).unwrap()).is_err());
        assert!(!temp.path().join("run").exists());
    }
    assert!(execute(&vec![b' '; MAX_REQUEST_BYTES + 1]).is_err());
    // Null/oversized requests return before creating a Rust slice.
    assert_eq!(unsafe { incant_game_run_v1(std::ptr::null(), 1) }, 2);
    assert_eq!(
        unsafe { incant_game_run_v1(std::ptr::dangling(), MAX_REQUEST_BYTES + 1) },
        2
    );
    fs::write(temp.path().join("package/world.scene"), b"corrupt").unwrap();
    let report = execute(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert!(!report.passed);
    assert!(report.error.unwrap().contains("checksum"));
    assert!(report.game.is_none());
    assert!(temp.path().join("run/result.json").exists());
    assert!(!temp.path().join("run/audio.pcm").exists());
}

#[test]
fn guest_failure_publishes_explicit_failure_and_no_partial_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = fixture(temp.path());
    let input = temp.path().join("failure.json");
    fs::write(&input, serde_json::to_vec(&json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":3,"frames":[
        {"tick":2,"events":[{"type":"key","code":"KeyF","down":true}]}]})).unwrap()).unwrap();
    request["input_recording"] = json!(input);
    let data = serde_json::to_vec(&request).unwrap();
    // SAFETY: the request allocation is live and immutable for this call.
    assert_eq!(unsafe { incant_game_run_v1(data.as_ptr(), data.len()) }, 1);
    let report: Report =
        serde_json::from_slice(&fs::read(temp.path().join("run/result.json")).unwrap()).unwrap();
    assert!(!report.passed);
    assert!(
        report
            .error
            .unwrap()
            .contains("accepted game input failure")
    );
    assert!(report.game.is_none());
    assert!(!temp.path().join("run/audio.pcm").exists());
}
