//! Actual parser/cooked-scene refusal; must return before any native setup.
#[cfg(all(feature = "native-minimum-duration", target_os = "macos"))]
#[test]
fn minimum_duration_scene_refusal_precedes_log_window_audio_and_gpu() {
    use incant_runtime::{CookedEntity, CookedScene, StableId};
    let root = tempfile::tempdir().unwrap();
    let scene_path = root.path().join("scene");
    let log_path = root.path().join("state.jsonl");
    let scene = CookedScene::new(
        StableId(99_u128.to_be_bytes()),
        30,
        vec![CookedEntity {
            id: StableId(1_u128.to_be_bytes()),
            parent: None,
            transform: incant_types::Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    std::fs::write(&scene_path, scene.to_bytes()).unwrap();
    let args = vec![
        "--window".into(),
        scene_path.to_str().unwrap().into(),
        "missing-cache".into(),
        "missing-clips".into(),
        "missing-actions".into(),
        "missing-models".into(),
        "missing-textures".into(),
        "0123456789abcdef0123456789abcdef".into(),
        "--stop-after".into(),
        "1".into(),
        "--window-state-log".into(),
        log_path.to_str().unwrap().into(),
        "--metal-present-mode".into(),
        "minimum-duration60".into(),
    ];
    let mut reports = Vec::new();
    let error = crate::run(args, |s| reports.push(s.to_owned())).unwrap_err();
    assert_eq!(
        error.downcast_ref::<crate::pacing::Error>(),
        Some(&crate::pacing::Error::SceneRate)
    );
    assert!(!log_path.exists());
    assert!(reports.is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
