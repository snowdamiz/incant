use incant_cmd::{Actor, CommandBus};
use incant_doc::Project;
use incant_import::{ImportRequest, ImportSnapshot};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../incant_assets/tests/fixtures/fbx")
}
fn request(source: &str) -> ImportRequest {
    ImportRequest {
        source: source.into(),
        texture_usage: None,
    }
}
#[test]
fn fbx_import_reimport_refusal_and_history_are_one_existing_transaction() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let original = fs::read_to_string(fixtures().join("maya_cube_7500_ascii.fbx")).unwrap();
    fs::write(root.join("cube.FBX"), &original).unwrap();
    let initial = Project::empty("FBX");
    let journal = root.join("history.jsonl");
    let mut bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
    let prepare = |bus: &CommandBus| {
        ImportSnapshot::capture(bus)
            .prepare(root, None, &[request("cube.FBX")])
            .unwrap()
    };
    let staged = prepare(&bus);
    assert_eq!(bus.project(), &initial);
    assert!(bus.history().is_empty());
    let imported = staged
        .commit(&mut bus, Actor::import("fbx-test"), "Import FBX")
        .unwrap();
    let first = imported.imports[0].asset.clone();
    assert_eq!(first.kind, "model");
    let unchanged = prepare(&bus);
    assert!(unchanged.outcomes()[0].cache_hit);
    assert!(!unchanged.outcomes()[0].changed);
    unchanged
        .commit(&mut bus, Actor::import("fbx-test"), "Unchanged")
        .unwrap();
    assert_eq!(bus.history().len(), 1);
    fs::write(root.join("cube.FBX"), original.replace("-0.5", "-0.75")).unwrap();
    let update = prepare(&bus);
    assert_eq!(update.outcomes()[0].asset.id, first.id);
    assert_ne!(update.outcomes()[0].asset.sha256, first.sha256);
    update
        .commit(&mut bus, Actor::import("fbx-test"), "Reimport FBX")
        .unwrap();
    let accepted = bus.project().clone();
    bus.undo().unwrap();
    assert_eq!(bus.project().assets[&first.id], first);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &accepted);
    let before_journal = fs::read(&journal).unwrap();
    let before_revision = bus.revision();
    fs::copy(
        fixtures().join("maya_textured_cube_7500_ascii.fbx"),
        root.join("bad.fbx"),
    )
    .unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("cube.FBX"), request("bad.fbx")])
            .is_err()
    );
    assert_eq!(bus.project(), &accepted);
    assert_eq!(bus.revision(), before_revision);
    assert_eq!(fs::read(&journal).unwrap(), before_journal);
    fs::write(root.join("cube.FBX"), b"truncated FBX").unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("cube.FBX")])
            .is_err()
    );
    assert_eq!(bus.project(), &accepted);
    fs::remove_file(root.join("cube.FBX")).unwrap();
    fs::remove_file(root.join("bad.fbx")).unwrap();
    let asset = &bus.project().assets[&first.id];
    let cooked =
        incant_assets::load_model(&root.join(".incant/cache/models"), &asset.sha256).unwrap();
    assert_eq!(cooked.meshes.len(), 1);
    drop(bus);
    let recovered = CommandBus::persistent(&journal, initial).unwrap();
    assert_eq!(recovered.project(), &accepted);
}
