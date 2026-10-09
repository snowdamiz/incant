use image::{DynamicImage, ImageFormat, RgbaImage};
use incant_assets::{AssetStore, RuntimeAssetData, TextureUsage, cook_texture};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, AssetImportSettings, Project, new_id};
use std::{fs, io::Cursor, path::Path, sync::Arc};

fn cook(root: &Path, name: &str, pixel: [u8; 4], usage: TextureUsage) -> Asset {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::from_raw(1, 1, pixel.to_vec()).unwrap())
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    fs::write(root.join(name), bytes.into_inner()).unwrap();
    let cooked = cook_texture(root, Path::new(name), &root.join("cache/textures"), usage).unwrap();
    Asset {
        id: new_id(),
        name: name.into(),
        path: name.into(),
        kind: "texture".into(),
        sha256: cooked.metadata.fingerprint,
        import_settings: Some(AssetImportSettings::Texture { usage }),
    }
}
fn edit(bus: &mut CommandBus, asset: &Asset) {
    bus.execute(
        vec![Command::UpsertAsset {
            asset: asset.clone(),
        }],
        Actor::import("texture"),
        "Import",
        None,
    )
    .unwrap();
}
fn pixel(asset: &incant_assets::RuntimeAsset) -> &[u8] {
    let RuntimeAssetData::Texture(texture) = asset.data() else {
        panic!("expected texture")
    };
    &texture.texture.levels[0]
}

#[test]
fn retained_assets_are_not_reused_as_authorization_for_another_project_scope() {
    let temp = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let root = temp.path();
    let asset = cook(root, "paint.png", [255, 0, 0, 255], TextureUsage::Color);
    fs::create_dir(root.join(".incant")).unwrap();
    fs::rename(root.join("cache"), root.join(".incant/cache")).unwrap();
    let mut bus = CommandBus::new(Project::empty("Scoped cache")).unwrap();
    edit(&mut bus, &asset);
    let mut store = AssetStore::default();
    store.sync_project(bus.project(), root).unwrap();
    let trusted = store.get(&asset.id).unwrap();
    // Same IDs and fingerprints do not grant the second root access to the first.
    assert!(store.sync_project(bus.project(), other.path()).is_err());
    assert!(Arc::ptr_eq(&trusted, &store.get(&asset.id).unwrap()));
    fs::rename(root.join(".incant/cache"), root.join("held-cache")).unwrap();
    assert!(
        store
            .sync_project(bus.project(), root)
            .unwrap()
            .loaded
            .is_empty()
    );
    let mut different_project = bus.project().clone();
    different_project.id = new_id();
    assert!(store.sync_project(&different_project, root).is_err());
    assert!(Arc::ptr_eq(&trusted, &store.get(&asset.id).unwrap()));
}

#[test]
#[cfg(unix)]
fn project_runtime_rejects_redirected_cache_directories_and_retains_trusted_versions() {
    use std::os::unix::fs::symlink;
    for (component, target) in [
        (".incant", ""),
        (".incant/cache", "cache"),
        (".incant/cache/textures", "cache/textures"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = temp.path();
        let original = cook(root, "paint.png", [255, 0, 0, 255], TextureUsage::Color);
        fs::create_dir(root.join(".incant")).unwrap();
        fs::rename(root.join("cache"), root.join(".incant/cache")).unwrap();
        fs::remove_file(root.join("paint.png")).unwrap();
        let mut bus = CommandBus::new(Project::empty("Scoped cache")).unwrap();
        edit(&mut bus, &original);
        let mut store = AssetStore::default();
        store.sync_project(bus.project(), root).unwrap();
        let before = store.snapshot();
        let retained = store.get(&original.id).unwrap();
        assert_eq!(pixel(&retained), [255, 0, 0, 255]);
        let mut replacement = cook(
            outside.path(),
            "paint.png",
            [0, 0, 255, 255],
            TextureUsage::Color,
        );
        replacement.id = original.id.clone();
        edit(&mut bus, &replacement);
        fs::rename(root.join(component), root.join("held-cache")).unwrap();
        symlink(outside.path().join(target), root.join(component)).unwrap();
        assert!(
            store.sync_project(bus.project(), root).is_err(),
            "redirected {component} must fail"
        );
        assert_eq!(store.snapshot(), before);
        assert!(Arc::ptr_eq(&retained, &store.get(&original.id).unwrap()));
        // The replacement really is valid; only the unauthorized project redirect
        // prevents loading. Explicit caller-granted caches remain supported.
        let mut explicit = AssetStore::default();
        explicit
            .sync(bus.project(), &outside.path().join("cache"))
            .unwrap();
        assert_eq!(
            pixel(&explicit.get(&original.id).unwrap()),
            [0, 0, 255, 255]
        );
    }
}

#[test]
fn live_versions_survive_reimport_and_undo_without_source_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    let mut bus = CommandBus::new(Project::empty("runtime")).unwrap();
    let original = cook(root, "paint.png", [255, 0, 0, 255], TextureUsage::Color);
    edit(&mut bus, &original);
    let mut store = AssetStore::default();
    fs::remove_file(root.join("paint.png")).unwrap();
    assert_eq!(
        store.sync(bus.project(), &cache).unwrap().loaded,
        std::slice::from_ref(&original.id)
    );
    let old = store.get(&original.id).unwrap();
    assert_eq!(pixel(&old), [255, 0, 0, 255]);
    assert!(store.sync(bus.project(), &cache).unwrap().loaded.is_empty());
    assert!(Arc::ptr_eq(&old, &store.get(&original.id).unwrap()));
    let mut replacement = cook(root, "paint.png", [0, 0, 255, 255], TextureUsage::Color);
    replacement.id = original.id.clone();
    edit(&mut bus, &replacement);
    store.sync(bus.project(), &cache).unwrap();
    let new = store.get(&original.id).unwrap();
    assert!(new.info().generation > old.info().generation);
    assert_eq!(pixel(&new), [0, 0, 255, 255]);
    assert_eq!(pixel(&old), [255, 0, 0, 255]);
    bus.undo().unwrap();
    store.sync(bus.project(), &cache).unwrap();
    let undone = store.get(&original.id).unwrap();
    assert!(undone.info().generation > new.info().generation);
    assert_eq!(pixel(&undone), [255, 0, 0, 255]);
    bus.redo().unwrap();
    store.sync(bus.project(), &cache).unwrap();
    assert_eq!(pixel(&store.get(&original.id).unwrap()), [0, 0, 255, 255]);
}

#[test]
fn failed_batches_keep_previous_versions_and_removals_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut bus = CommandBus::new(Project::empty("runtime")).unwrap();
    let original = cook(root, "paint.png", [255, 0, 0, 255], TextureUsage::Color);
    edit(&mut bus, &original);
    let mut store = AssetStore::default();
    store.sync(bus.project(), &root.join("cache")).unwrap();
    let before = store.snapshot();
    let retained = store.get(&original.id).unwrap();
    let mut missing = original.clone();
    missing.sha256 = "0".repeat(64);
    edit(&mut bus, &missing);
    assert!(store.sync(bus.project(), &root.join("cache")).is_err());
    assert_eq!(store.snapshot(), before);
    assert!(Arc::ptr_eq(&retained, &store.get(&original.id).unwrap()));
    bus.undo().unwrap();
    let mut added = original.clone();
    added.id = "00000000000000000000000001".into();
    missing.id = "00000000000000000000000002".into();
    bus.execute(
        vec![
            Command::RemoveAsset {
                asset_id: original.id.clone(),
            },
            Command::UpsertAsset { asset: added },
            Command::UpsertAsset { asset: missing },
        ],
        Actor::import("texture"),
        "Batch reimport",
        None,
    )
    .unwrap();
    // One new version loads successfully, but a later failure must preserve
    // both the old entry and the generation counter, including staged removals.
    assert!(store.sync(bus.project(), &root.join("cache")).is_err());
    assert_eq!(store.snapshot(), before);
    bus.undo().unwrap();
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
        store
            .sync(bus.project(), &root.join("cache"))
            .unwrap()
            .removed,
        std::slice::from_ref(&original.id)
    );
    assert!(store.get(&original.id).is_none());
    assert_eq!(pixel(&retained), [255, 0, 0, 255]);
    bus.undo().unwrap();
    store.sync(bus.project(), &root.join("cache")).unwrap();
    assert_eq!(
        store.get(&original.id).unwrap().info().generation,
        retained.info().generation + 1
    );
}

#[test]
fn payload_budget_and_import_settings_are_enforced_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let first = cook(root, "data.png", [255, 128, 128, 255], TextureUsage::Normal);
    let mut second = cook(root, "other.png", [0, 0, 0, 255], TextureUsage::Color);
    let mut bus = CommandBus::new(Project::empty("runtime")).unwrap();
    edit(&mut bus, &first);
    let mut store = AssetStore::with_budget(4);
    store.sync(bus.project(), &root.join("cache")).unwrap();
    let before = store.snapshot();
    edit(&mut bus, &second);
    assert!(store.sync(bus.project(), &root.join("cache")).is_err());
    assert_eq!(store.snapshot(), before);
    bus.undo().unwrap();
    second = first.clone();
    second.import_settings = Some(AssetImportSettings::Texture {
        usage: TextureUsage::Color,
    });
    edit(&mut bus, &second);
    assert!(store.sync(bus.project(), &root.join("cache")).is_err());
    assert_eq!(store.snapshot(), before);
    bus.undo().unwrap();
    second = first.clone();
    second.name = "Renamed".into();
    edit(&mut bus, &second);
    assert!(
        store
            .sync(bus.project(), &root.join("cache"))
            .unwrap()
            .loaded
            .is_empty()
    );
    assert_eq!(store.snapshot(), before);
}

#[test]
fn unknown_runtime_kinds_and_corrupt_replacements_fail_explicitly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut asset = cook(root, "paint.png", [255, 0, 0, 255], TextureUsage::Color);
    let mut bus = CommandBus::new(Project::empty("runtime")).unwrap();
    edit(&mut bus, &asset);
    let mut store = AssetStore::default();
    store.sync(bus.project(), &root.join("cache")).unwrap();
    let before = store.snapshot();
    asset.kind = "future_audio".into();
    asset.import_settings = None;
    edit(&mut bus, &asset);
    let error = store.sync(bus.project(), &root.join("cache")).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported asset feature: runtime asset kind future_audio")
    );
    assert_eq!(store.snapshot(), before);
    bus.undo().unwrap();
    let mut replacement = cook(root, "paint.png", [0, 0, 255, 255], TextureUsage::Color);
    replacement.id = asset.id;
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.join("cache/textures")
                .join(format!("{}.json", replacement.sha256)),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("cache/textures").join(format!(
            "{}.ktx2",
            manifest["content_sha256"].as_str().unwrap()
        )),
        b"corrupt",
    )
    .unwrap();
    edit(&mut bus, &replacement);
    assert!(store.sync(bus.project(), &root.join("cache")).is_err());
    assert_eq!(store.snapshot(), before);
}
