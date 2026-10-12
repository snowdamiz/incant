//! Real embedded bytecode and offline clock checks, without a device or GPU.
use super::*;
use incant_runtime::{CookedEntity, CookedScene, StableId};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn scene(rate: u32) -> CookedScene {
    CookedScene::new(
        StableId([9; 16]),
        rate,
        vec![CookedEntity {
            id: StableId(1_u128.to_be_bytes()),
            parent: None,
            transform: Default::default(),
            velocity: Some(incant_types::Velocity { linear: [0.; 3] }),
        }],
    )
    .unwrap()
}
fn args(root: &Path, wrong_observation: bool, late_failure: bool) -> Vec<String> {
    fs::write(root.join("world"), scene(60).to_bytes()).unwrap();
    fs::write(
        root.join("clips"),
        [b"NCLIPS01".as_slice(), &[0; 4]].concat(),
    )
    .unwrap();
    fs::write(
        root.join("actions"),
        serde_json::to_vec(&json!({
            "right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},
            "fail":{"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut frames = vec![json!({"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]})];
    if late_failure {
        frames.push(json!({"tick":1251,"events":[{"type":"key","code":"KeyF","down":true}]}));
    }
    fs::write(root.join("input"), serde_json::to_vec(&json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":1251,"frames":frames})).unwrap()).unwrap();
    fs::write(root.join("observe"), serde_json::to_vec(&json!({"format":"incant-native-observation","version":1,"entities":["00000000000000000000000001"],"checks":[{
        "name":"complete movement", "tick":1251,"expect":{"type":"vector","entity":"00000000000000000000000001","property":"translation","value":[if wrong_observation {0.}else{62.5},0.,0.],"tolerance":1e-9}
    }]})).unwrap()).unwrap();
    vec![
        root.join("world"),
        root.to_path_buf(),
        root.join("clips"),
        root.join("input"),
        root.join("actions"),
    ]
    .into_iter()
    .map(|p| p.to_str().unwrap().to_owned())
    .chain([
        "1251".into(),
        "48000".into(),
        root.join("audio.pcm").to_str().unwrap().into(),
        "--observe".into(),
        root.join("observe").to_str().unwrap().into(),
    ])
    .collect()
}
fn no_staging(root: &Path) {
    assert!(!fs::read_dir(root).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".incant-pcm-")
    }));
}
#[test]
fn complete_player_exceeds_previous_limit_and_preserves_diagnostic_pcm_and_no_clobber() {
    let root = tempfile::tempdir().unwrap();
    let args = args(root.path(), false, false);
    let mut reports = vec![];
    crate::run(args.clone(), |r| reports.push(r.to_owned())).unwrap();
    let report: Value = serde_json::from_str(&reports[0]).unwrap();
    assert_eq!(report["tick"], 1251);
    assert_eq!(report["frames"], 1_000_800);
    assert_eq!(report["observations"]["passed"], true);
    let data = fs::read(root.path().join("audio.pcm")).unwrap();
    assert_eq!(data.len(), 1_000_800 * 8);
    assert!(data.iter().all(|v| *v == 0));
    assert_eq!(report["pcm_sha256"], sha256(&data));
    reports.clear();
    assert!(crate::run(args, |r| reports.push(r.to_owned())).is_err());
    assert!(reports.is_empty());
    assert_eq!(fs::read(root.path().join("audio.pcm")).unwrap(), data);
    no_staging(root.path());
    let wrong = tempfile::tempdir().unwrap();
    let err = crate::run(self::args(wrong.path(), true, false), |r| {
        reports.push(r.to_owned())
    })
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "native observation assertions failed (see completed report)"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&reports[0]).unwrap()["observations"]["passed"],
        false
    );
    assert_eq!(fs::read(wrong.path().join("audio.pcm")).unwrap(), data);
    no_staging(wrong.path());
}
#[test]
fn late_real_guest_failure_discards_one_million_staged_frames() {
    let root = tempfile::tempdir().unwrap();
    let mut reports = vec![];
    let err = crate::run(args(root.path(), false, true), |r| {
        reports.push(r.to_owned())
    })
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "script tick failed: Error: accepted game input failure"
    );
    assert!(reports.is_empty());
    assert!(!root.path().join("audio.pcm").exists());
    no_staging(root.path());
}
#[test]
fn checkpoint_fractional_phase_streams_exact_continuation() {
    use incant_game_runtime::OfflineGameSession;
    let initial = scene(144);
    let code=incant_script::CompiledScript::compile("exports.default={persistence:{version:1},update(t){const n=Number(t.state.get()||0)+1;t.state.set(n);t.chunks[0].transforms[0]=n;}} ").unwrap();
    let mut full = OfflineGameSession::new(
        initial.clone(),
        code.clone(),
        Default::default(),
        vec![],
        44117,
    )
    .unwrap();
    let mut first = Stream::new(Vec::new(), 13000);
    let mut tail = Stream::new(Vec::new(), 13000);
    let mut buffer = vec![0.; 44117_usize.div_ceil(144) * 2];
    for _ in 0..13 {
        let n = full.next_output_frames().unwrap() * 2;
        full.tick(&[], &mut buffer[..n]).unwrap();
        first.append(&buffer[..n]).unwrap();
    }
    let save = full.checkpoint_bytes().unwrap();
    let mut resumed = OfflineGameSession::from_checkpoint_bytes(
        &save,
        &initial,
        code,
        Default::default(),
        vec![],
        44117,
    )
    .unwrap();
    let mut actual = Stream::new(Vec::new(), 13000);
    for _ in 0..27 {
        let n = full.next_output_frames().unwrap() * 2;
        full.tick(&[], &mut buffer[..n]).unwrap();
        tail.append(&buffer[..n]).unwrap();
        let n = resumed.next_output_frames().unwrap() * 2;
        resumed.tick(&[], &mut buffer[..n]).unwrap();
        actual.append(&buffer[..n]).unwrap();
    }
    assert_eq!(tail.writer, actual.writer);
    assert_eq!(
        tail.frames,
        crate::checkpoint::frame_range(13, 27, 44117, 144)
            .unwrap()
            .1 as u64
    );
    assert_eq!(first.frames + tail.frames, 40 * 44117 / 144);
    assert_eq!(
        full.checkpoint_bytes().unwrap(),
        resumed.checkpoint_bytes().unwrap()
    );
}
