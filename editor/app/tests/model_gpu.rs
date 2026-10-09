//! Uses the real renderer to exercise the editor's revision/recovery controller.
#[path = "../src/render_scene.rs"]
mod render_scene;
#[path = "../../../crates/incant_render/tests/support/mod.rs"]
mod support;

#[test]
#[ignore = "requires a native GPU; run by desktop and macOS workflows"]
fn editor_publishes_complete_revisions_and_recovers_after_failed_replacements() {
    let temp = tempfile::tempdir().unwrap();
    let (mut project, _, id) = support::fixture(temp.path());
    let renderer = incant_render::Renderer::headless().unwrap();
    let mut runtime = render_scene::SceneRuntime::new(&renderer).unwrap();
    assert!(runtime.needs_update(0));
    runtime
        .update(&renderer, &project, 0, Some(temp.path()))
        .unwrap();
    assert!(!runtime.needs_update(0));
    let original = renderer
        .screenshot_scene_png(&runtime.scene, 320, 180)
        .unwrap();
    let saved = project.assets[&id].clone();
    project.assets.get_mut(&id).unwrap().sha256 = "0".repeat(64);
    assert!(
        runtime
            .update(&renderer, &project, 1, Some(temp.path()))
            .is_err()
    );
    assert_eq!(
        original,
        renderer
            .screenshot_scene_png(&runtime.scene, 320, 180)
            .unwrap()
    );
    assert!(runtime.needs_update(2));
    project.assets.insert(id.clone(), saved);
    runtime
        .update(&renderer, &project, 2, Some(temp.path()))
        .unwrap();
    assert!(!runtime.needs_update(2));

    let mut replacement = support::model(temp.path(), 2.);
    replacement.id = id.clone();
    project.assets.insert(id, replacement);
    runtime
        .update(&renderer, &project, 3, Some(temp.path()))
        .unwrap();
    assert_ne!(
        original,
        renderer
            .screenshot_scene_png(&runtime.scene, 320, 180)
            .unwrap()
    );
    assert!(!runtime.needs_update(3));
    assert!(runtime.needs_update(4));
}
