//! Real GPU checks, explicitly run by the desktop renderer workflow.
mod support;
use incant_render::Renderer;

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn imported_geometry_survives_sources_and_retained_versions_survive_reimport() {
    let temp = tempfile::tempdir().unwrap();
    let (mut project, mut assets, id) = support::fixture(temp.path());
    let renderer = Renderer::headless().unwrap();
    let original = renderer.prepare_scene(&project, &assets).unwrap();
    assert_eq!(original.stats().model_draw_calls, 1);
    assert_eq!(original.stats().primitive_instances, 2);
    std::fs::remove_file(temp.path().join("triangle.gltf")).unwrap();
    std::fs::remove_file(temp.path().join("triangle.bin")).unwrap();
    let first = renderer.screenshot_scene_png(&original, 320, 180).unwrap();
    let empty = renderer
        .screenshot_png(&incant_doc::Project::empty("Empty"), 320, 180)
        .unwrap();
    assert_ne!(first, empty, "indexed models must draw actual pixels");
    let mut replacement = support::model(temp.path(), 2.);
    replacement.id = id.clone();
    project.assets.insert(id.clone(), replacement);
    assets
        .sync(&project, &temp.path().join(".incant/cache"))
        .unwrap();
    let updated = renderer.prepare_scene(&project, &assets).unwrap();
    let second = renderer.screenshot_scene_png(&updated, 320, 180).unwrap();
    assert_ne!(
        first, second,
        "changed source vertices must change the GPU output"
    );
    assert_eq!(
        first,
        renderer.screenshot_scene_png(&original, 320, 180).unwrap(),
        "retained scene must keep old vertex/index buffers"
    );
    project.assets.get_mut(&id).unwrap().sha256 = "0".repeat(64);
    assert!(renderer.prepare_scene(&project, &assets).is_err());
    assert_eq!(
        second,
        renderer.screenshot_scene_png(&updated, 320, 180).unwrap(),
        "failed replacement must leave the prior scene usable"
    );
}
