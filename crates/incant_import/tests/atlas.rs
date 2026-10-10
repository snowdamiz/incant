use image::{Rgba, RgbaImage};
use incant_assets::{AssetStore, RuntimeAssetData};
use incant_cmd::{Actor, CommandBus};
use incant_doc::Project;
use incant_import::{ImportRequest, ImportSnapshot, SourceWatcher};
use serde_json::json;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
fn atlas_dependency_reimports_share_history_and_restore_without_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    RgbaImage::from_pixel(3, 2, Rgba([255u8, 0, 0, 255]))
        .save(root.join("first.png"))
        .unwrap();
    RgbaImage::from_pixel(4, 4, Rgba([0u8, 255, 0, 255]))
        .save(root.join("second.png"))
        .unwrap();
    let descriptor = json!({"version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
        "frames":{"00000000000000000000000001":{"name":"First","path":"first.png"},
                  "00000000000000000000000002":{"name":"Second","path":"second.png"}}});
    fs::write(root.join("sheet.atlas.json"), descriptor.to_string()).unwrap();
    let seed = Project::empty("Atlas watcher");
    let journal = root.join("history.jsonl");
    let mut bus = CommandBus::persistent(&journal, seed.clone()).unwrap();
    let result = ImportSnapshot::capture(&bus)
        .prepare(
            root,
            None,
            &[ImportRequest {
                source: "sheet.atlas.json".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("atlas"), "Import atlas")
        .unwrap();
    let first = result.imports[0].asset.clone();
    let before = bus.project().canonical_text().unwrap();
    let mut store = AssetStore::default();
    store.sync_project(bus.project(), root).unwrap();
    let retained = store.get(&first.id).unwrap();
    let RuntimeAssetData::Texture(original) = retained.data() else {
        panic!("atlas must load as a texture")
    };
    assert_eq!(original.metadata.all_dependencies().len(), 3);
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    assert!(
        watcher
            .poll(&mut bus, Instant::now())
            .unwrap()
            .imports
            .is_empty()
    );
    RgbaImage::from_pixel(17, 13, Rgba([0u8, 0, 255, 255]))
        .save(root.join("first.png"))
        .unwrap();
    let report = watcher.poll(&mut bus, Instant::now()).unwrap();
    assert_eq!(report.imports.len(), 1);
    assert!(report.diagnostics.is_empty());
    let second = bus.project().assets[&first.id].clone();
    assert_ne!(first.sha256, second.sha256);
    let after = bus.project().canonical_text().unwrap();
    assert_eq!(bus.history().len(), 2);
    assert_eq!(bus.history()[1].actor.origin, incant_doc::Origin::Import);
    store.sync_project(bus.project(), root).unwrap();
    let current = store.get(&first.id).unwrap();
    let RuntimeAssetData::Texture(current) = current.data() else {
        panic!("atlas must load as a texture")
    };
    assert_eq!(
        &current.metadata.atlas.as_ref().unwrap().frames["00000000000000000000000001"].region[2..],
        &[17, 13]
    );
    assert_eq!(
        &original.metadata.atlas.as_ref().unwrap().frames["00000000000000000000000001"].region[2..],
        &[3, 2]
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    assert!(
        watcher
            .poll(&mut bus, Instant::now())
            .unwrap()
            .imports
            .is_empty()
    );
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    fs::write(root.join("first.png"), b"incomplete source").unwrap();
    let report = watcher.poll(&mut bus, Instant::now()).unwrap();
    assert!(report.imports.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    for name in ["sheet.atlas.json", "first.png", "second.png"] {
        fs::remove_file(root.join(name)).unwrap();
    }
    drop(bus);
    let recovered = CommandBus::persistent(&journal, seed).unwrap();
    assert_eq!(recovered.project().canonical_text().unwrap(), after);
    let mut cold = AssetStore::default();
    cold.sync_project(recovered.project(), root).unwrap();
    assert_eq!(
        cold.get(&first.id).unwrap().info().fingerprint,
        second.sha256
    );
}
