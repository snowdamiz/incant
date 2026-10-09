use image::{Rgba, RgbaImage};
use incant_assets::AssetStore;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Project};
use incant_import::{ImportRequest, ImportSnapshot, SourceWatcher};
use serde_json::json;
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

fn image(root: &Path, name: &str, value: u8) {
    RgbaImage::from_pixel(1, 1, Rgba([value, 128, 255, 255]))
        .save(root.join(name))
        .unwrap();
}
fn import(bus: &mut CommandBus, root: &Path, names: &[&str]) {
    ImportSnapshot::capture(bus)
        .prepare(
            root,
            None,
            &names
                .iter()
                .map(|s| ImportRequest {
                    source: (*s).into(),
                    texture_usage: None,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
        .commit(bus, Actor::import("test"), "Import")
        .unwrap();
}
fn asset(bus: &CommandBus, path: &str) -> Asset {
    bus.project()
        .assets
        .values()
        .find(|a| a.path == path)
        .unwrap()
        .clone()
}
fn buffer(root: &Path, name: &str, x: f32) {
    let bytes: Vec<_> = [0_f32, 0., 0., x, 0., 0., 0., 1., 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    fs::write(root.join(name), bytes).unwrap();
}
fn model(root: &Path, bin: &str) {
    let document = json!({"asset":{"version":"2.0"},"buffers":[{"uri":bin,"byteLength":36}],
        "bufferViews":[{"buffer":0,"byteLength":36}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[10,1,0]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
        "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    fs::write(root.join("model.gltf"), document.to_string()).unwrap();
}

#[test]
fn stale_preparation_keeps_edits_and_does_not_consume_watch_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "good.png", 1);
    image(root, "bad.png", 2);
    let mut bus = CommandBus::new(Project::empty("Concurrent watch")).unwrap();
    import(&mut bus, root, &["good.png", "bad.png"]);
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    watcher.poll(&mut bus, Instant::now()).unwrap();
    image(root, "good.png", 7);
    fs::write(root.join("bad.png"), "unfinished source").unwrap();
    let before = bus.project().clone();
    let prepared = watcher.prepare(ImportSnapshot::capture(&bus), Instant::now());
    assert_eq!(bus.project(), &before);
    assert_eq!(watcher.diagnostics().count(), 0);
    let mut renamed = asset(&bus, "good.png");
    renamed.name = "User's concurrent name".into();
    bus.execute(
        vec![Command::UpsertAsset { asset: renamed }],
        Actor::user("editor"),
        "Rename",
        None,
    )
    .unwrap();
    let after_edit = bus.project().clone();
    let history = bus.history().len();
    assert!(matches!(
        watcher.commit(prepared, &mut bus),
        Err(incant_import::ImportError::Command(
            incant_cmd::CommandError::Conflict { .. }
        ))
    ));
    assert_eq!(bus.project(), &after_edit);
    assert_eq!(bus.history().len(), history);
    assert_eq!(watcher.diagnostics().count(), 0);
    let retry = watcher.poll(&mut bus, Instant::now()).unwrap();
    assert_eq!(retry.imports.len(), 1);
    assert_eq!(retry.diagnostics.len(), 1);
    assert_eq!(asset(&bus, "good.png").name, "User's concurrent name");
    assert_eq!(bus.history().len(), history + 1);
}

#[test]
fn prepared_cycles_cannot_be_reordered_or_published_by_another_watcher() {
    let temp = tempfile::tempdir().unwrap();
    let mut bus = CommandBus::new(Project::empty("Watch order")).unwrap();
    let mut watcher = SourceWatcher::new(temp.path(), Duration::ZERO);
    let earlier = watcher.prepare(ImportSnapshot::capture(&bus), Instant::now());
    let later = watcher.prepare(ImportSnapshot::capture(&bus), Instant::now());
    watcher.commit(later, &mut bus).unwrap();
    assert!(matches!(
        watcher.commit(earlier, &mut bus),
        Err(incant_import::ImportError::StaleWatch)
    ));
    let prepared = watcher.prepare(ImportSnapshot::capture(&bus), Instant::now());
    let mut other = SourceWatcher::new(temp.path(), Duration::ZERO);
    assert!(matches!(
        other.commit(prepared, &mut bus),
        Err(incant_import::ImportError::StaleWatch)
    ));
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
}

#[test]
fn burst_writes_form_one_transaction_and_undo_is_not_immediately_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "a.png", 1);
    image(root, "b.png", 2);
    let mut bus = CommandBus::persistent(root.join("journal"), Project::empty("Watch")).unwrap();
    import(&mut bus, root, &["a.png", "b.png"]);
    let original = bus.project().clone();
    let id = asset(&bus, "a.png").id;
    let mut runtime = AssetStore::default();
    runtime
        .sync(bus.project(), &root.join(".incant/cache"))
        .unwrap();
    let old = runtime.get(&id).unwrap().clone();
    let mut watcher = SourceWatcher::new(root, Duration::from_millis(100));
    let now = Instant::now();
    assert_eq!(watcher.poll(&mut bus, now).unwrap().pending, 0);
    image(root, "a.png", 3);
    image(root, "b.png", 4);
    assert_eq!(watcher.poll(&mut bus, now).unwrap().pending, 2);
    image(root, "a.png", 5);
    assert!(
        watcher
            .poll(&mut bus, now + Duration::from_millis(50))
            .unwrap()
            .imports
            .is_empty()
    );
    let report = watcher
        .poll(&mut bus, now + Duration::from_millis(150))
        .unwrap();
    assert_eq!(report.imports.len(), 2);
    assert_eq!(report.pending, 0);
    assert_eq!(bus.history().len(), 2);
    assert_eq!(bus.history()[1].commands.len(), 2);
    assert_eq!(bus.history()[1].actor.origin, incant_doc::Origin::Import);
    runtime
        .sync(bus.project(), &root.join(".incant/cache"))
        .unwrap();
    assert!(!Arc::ptr_eq(&old, &runtime.get(&id).unwrap()));
    assert_ne!(
        old.info().fingerprint,
        runtime.get(&id).unwrap().info().fingerprint
    );
    bus.undo().unwrap();
    let revision = bus.revision();
    assert_eq!(bus.project(), &original);
    assert!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(10))
            .unwrap()
            .imports
            .is_empty()
    );
    assert_eq!(bus.revision(), revision);
    runtime
        .sync(bus.project(), &root.join(".incant/cache"))
        .unwrap();
    assert_eq!(
        runtime.get(&id).unwrap().info().fingerprint,
        old.info().fingerprint
    );
    image(root, "a.png", 6);
    watcher
        .poll(&mut bus, now + Duration::from_secs(11))
        .unwrap();
    assert_eq!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(12))
            .unwrap()
            .imports
            .len(),
        1
    );
    let expected = bus.project().clone();
    drop(bus);
    let recovered = CommandBus::persistent(root.join("journal"), original).unwrap();
    assert_eq!(recovered.project(), &expected);
}

#[test]
fn external_same_size_edits_and_new_dependencies_are_detected_and_repaired() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    buffer(root, "first.bin", 1.);
    model(root, "first.bin");
    let mut bus = CommandBus::new(Project::empty("Model")).unwrap();
    import(&mut bus, root, &["model.gltf"]);
    let original = asset(&bus, "model.gltf");
    // Edit while closed: first poll compares actual content to cooked dependencies.
    buffer(root, "first.bin", 2.);
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    let report = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(report.imports.len(), 1);
    assert_ne!(asset(&bus, "model.gltf").sha256, original.sha256);
    // The next cook refers to a dependency that doesn't exist yet.
    model(root, "second.bin");
    assert_eq!(watcher.poll(&mut bus, now).unwrap().diagnostics.len(), 1);
    let revision = bus.revision();
    buffer(root, "second.bin", 3.);
    assert!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(1))
            .unwrap()
            .imports
            .is_empty()
    );
    assert_eq!(bus.revision(), revision);
    let repaired = watcher
        .poll(&mut bus, now + Duration::from_secs(2))
        .unwrap();
    assert_eq!(repaired.imports.len(), 1);
    assert_eq!(repaired.cleared, vec![original.id]);
    assert_eq!(watcher.diagnostics().count(), 0);
    fs::remove_file(root.join("first.bin")).unwrap();
    assert_eq!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(3))
            .unwrap()
            .pending,
        0
    );
    buffer(root, "second.bin", 4.);
    assert_eq!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(3))
            .unwrap()
            .imports
            .len(),
        1
    );
}

#[test]
fn broken_and_deleted_sources_preserve_last_good_assets_without_blocking_other_imports() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "a.png", 1);
    image(root, "b.png", 2);
    let mut bus = CommandBus::new(Project::empty("Recovery")).unwrap();
    import(&mut bus, root, &["a.png", "b.png"]);
    let original = asset(&bus, "a.png");
    let valid_bytes = fs::read(root.join("a.png")).unwrap();
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    watcher.poll(&mut bus, now).unwrap();
    fs::write(root.join("a.png"), "half-written").unwrap();
    image(root, "b.png", 5);
    let result = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(asset(&bus, "a.png"), original);
    assert_eq!(watcher.diagnostics().count(), 1);
    assert!(
        watcher
            .poll(&mut bus, now + Duration::from_secs(3))
            .unwrap()
            .diagnostics
            .is_empty()
    );
    fs::remove_file(root.join("a.png")).unwrap();
    assert_eq!(watcher.poll(&mut bus, now).unwrap().diagnostics.len(), 1);
    assert_eq!(asset(&bus, "a.png"), original);
    fs::write(root.join("a.png"), valid_bytes).unwrap();
    let revision = bus.revision();
    let restored = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(restored.cleared, vec![original.id.clone()]);
    assert_eq!(restored.pending, 0);
    assert_eq!(revision, bus.revision());
    fs::remove_file(root.join("a.png")).unwrap();
    watcher.poll(&mut bus, now).unwrap();
    bus.execute(
        vec![Command::RemoveAsset {
            asset_id: original.id.clone(),
        }],
        Actor::user("test"),
        "Remove",
        None,
    )
    .unwrap();
    assert_eq!(
        watcher.poll(&mut bus, now).unwrap().cleared,
        vec![original.id]
    );
    assert_eq!(watcher.diagnostics().count(), 0);
}

#[test]
fn cache_repair_is_a_noop_and_unrelated_files_do_not_create_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "a.png", 1);
    let mut bus = CommandBus::new(Project::empty("Cache")).unwrap();
    import(&mut bus, root, &["a.png"]);
    fs::remove_dir_all(root.join(".incant/cache/textures")).unwrap();
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    let repaired = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(repaired.imports.len(), 1);
    assert!(!repaired.imports[0].changed);
    assert!(!repaired.imports[0].cache_hit);
    image(root, "unregistered.png", 9);
    fs::write(root.join(".incant/cache/irrelevant"), b"cache write").unwrap();
    assert_eq!(watcher.poll(&mut bus, now).unwrap().pending, 0);
    assert_eq!(bus.history().len(), 1);
    // A user changes interpretation without changing the file's content. The
    // new settings need a new cook, while the user's name and ULID survive.
    let mut settings = asset(&bus, "a.png");
    settings.name = "Surface normals".into();
    settings.import_settings = Some(incant_doc::AssetImportSettings::Texture {
        usage: incant_assets::TextureUsage::Normal,
    });
    bus.execute(
        vec![Command::UpsertAsset {
            asset: settings.clone(),
        }],
        Actor::user("test"),
        "Change usage",
        None,
    )
    .unwrap();
    let reinterpreted = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(reinterpreted.imports.len(), 1);
    assert_ne!(reinterpreted.imports[0].asset.sha256, settings.sha256);
    assert_eq!(reinterpreted.imports[0].asset.name, settings.name);
    assert_eq!(reinterpreted.imports[0].asset.id, settings.id);
    let mut runtime = AssetStore::default();
    runtime
        .sync(bus.project(), &root.join(".incant/cache"))
        .unwrap();
}

#[test]
fn projects_larger_than_a_batch_make_progress_past_a_broken_source() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut bus = CommandBus::new(Project::empty("Large")).unwrap();
    let names: Vec<_> = (0..65).map(|i| format!("{i}.png")).collect();
    for name in &names {
        image(root, name, 1);
    }
    for group in names.chunks(64) {
        import(
            &mut bus,
            root,
            &group.iter().map(String::as_str).collect::<Vec<_>>(),
        );
    }
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    watcher.poll(&mut bus, now).unwrap();
    for name in &names {
        image(root, name, 2);
    }
    fs::write(root.join("0.png"), b"broken").unwrap();
    let first = watcher.poll(&mut bus, now).unwrap();
    let second = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(first.imports.len() + second.imports.len(), 64);
    assert_eq!(second.pending, 1);
    assert_eq!(watcher.diagnostics().count(), 1);
}

#[cfg(unix)]
#[test]
fn replacing_a_source_with_an_outside_symlink_cannot_import_outside_content() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let outside = tempfile::tempdir().unwrap();
    image(root, "a.png", 1);
    image(outside.path(), "private.png", 2);
    let mut bus = CommandBus::new(Project::empty("Confined")).unwrap();
    import(&mut bus, root, &["a.png"]);
    let original = bus.project().clone();
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    watcher.poll(&mut bus, now).unwrap();
    fs::remove_file(root.join("a.png")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("private.png"), root.join("a.png")).unwrap();
    let failed = watcher.poll(&mut bus, now).unwrap();
    assert_eq!(failed.diagnostics.len(), 1);
    assert!(failed.diagnostics[0].message.contains("escapes"));
    assert_eq!(bus.project(), &original);
}
