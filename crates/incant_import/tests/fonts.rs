use incant_assets::{AssetStore, RuntimeAssetData};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Origin, Project};
use incant_import::{ImportDetails, ImportRequest, ImportSnapshot, SourceWatcher};
use incant_text::{FontSource, TextEngine, TextStyle, Wrap};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const HEBREW: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansHebrew.ttf");
fn request(source: &str) -> ImportRequest {
    ImportRequest {
        source: source.into(),
        texture_usage: None,
    }
}
fn style() -> TextStyle {
    TextStyle {
        font_size: 20.,
        line_height: 30.,
        width: 800.,
        wrap: Wrap::WordOrGlyph,
    }
}
fn source(store: &AssetStore, id: &str) -> FontSource {
    let asset = store.get(id).unwrap();
    let RuntimeAssetData::Font(font) = asset.data() else {
        panic!("font missing")
    };
    FontSource {
        id: id.parse().unwrap(),
        bytes: font.bytes().to_vec(),
    }
}
#[test]
fn public_font_batch_reopens_cooked_only_and_shapes_multilingual_text_after_source_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for (path, bytes) in [
        ("latin.ttf", LATIN),
        ("arabic.ttf", ARABIC),
        ("hebrew.otf", HEBREW),
    ] {
        fs::write(root.join(path), bytes).unwrap();
    }
    let initial = Project::empty("Fonts");
    let journal = root.join("history.jsonl");
    let mut bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
    let prepared = ImportSnapshot::capture(&bus)
        .prepare(
            root,
            None,
            &[
                request("latin.ttf"),
                request("arabic.ttf"),
                request("hebrew.otf"),
            ],
        )
        .unwrap();
    assert_eq!(bus.project(), &initial);
    assert!(bus.history().is_empty());
    let committed = prepared
        .commit(
            &mut bus,
            Actor::agent("test-model", "test-conversation"),
            "Import packaged fonts",
        )
        .unwrap();
    let ids: Vec<_> = committed
        .imports
        .iter()
        .map(|i| i.asset.id.clone())
        .collect();
    assert_eq!(bus.history().len(), 1);
    assert_eq!(bus.history()[0].commands.len(), 3);
    assert_eq!(bus.history()[0].actor.origin, Origin::Agent);
    for import in &committed.imports {
        assert_eq!(import.asset.kind, "font");
        assert_eq!(import.asset.import_settings, None);
        let ImportDetails::Font {
            bytes,
            dependencies,
        } = &import.details
        else {
            panic!()
        };
        assert!(*bytes > 0);
        assert_eq!(dependencies.len(), 1);
    }
    let imported = bus.project().clone();
    bus.undo().unwrap();
    assert_eq!(bus.project(), &initial);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &imported);
    drop(bus);
    let recovered = CommandBus::persistent(&journal, initial).unwrap();
    assert_eq!(recovered.project(), &imported);
    for path in ["latin.ttf", "arabic.ttf", "hebrew.otf"] {
        fs::remove_file(root.join(path)).unwrap();
    }
    let mut store = AssetStore::default();
    assert_eq!(
        store
            .sync_project(recovered.project(), root)
            .unwrap()
            .loaded
            .len(),
        3
    );
    let mut engine = TextEngine::new(ids.iter().map(|id| source(&store, id)).collect()).unwrap();
    let shaped = engine
        .shape("abc\r\nשלום\nسلام ffi e\u{301}", style())
        .unwrap();
    assert_eq!(shaped.lines().len(), 3);
    assert!(shaped.missing_clusters().is_empty());
    let mut explicit = AssetStore::default();
    assert_eq!(
        explicit
            .sync(recovered.project(), &root.join(".incant/cache"))
            .unwrap()
            .loaded
            .len(),
        3
    );
    assert!(
        AssetStore::with_budget(0)
            .sync_project(recovered.project(), root)
            .is_err()
    );
}
#[test]
fn font_reimport_watch_undo_and_bad_replacement_preserve_ids_names_and_retained_versions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("font.ttf"), LATIN).unwrap();
    let mut bus = CommandBus::new(Project::empty("Fonts")).unwrap();
    let first = ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("font.ttf")])
        .unwrap()
        .commit(&mut bus, Actor::import("font-test"), "Import font")
        .unwrap();
    let id = first.imports[0].asset.id.clone();
    let first_key = first.imports[0].asset.sha256.clone();
    let mut named = first.imports[0].asset.clone();
    named.name = "User name".into();
    bus.execute(
        vec![Command::UpsertAsset { asset: named }],
        Actor::user("font-test"),
        "Rename",
        None,
    )
    .unwrap();
    let mut store = AssetStore::default();
    store.sync_project(bus.project(), root).unwrap();
    let retained = store.get(&id).unwrap();
    let start = Instant::now();
    let mut watcher = SourceWatcher::new(root, Duration::from_millis(10));
    assert!(watcher.poll(&mut bus, start).unwrap().imports.is_empty());
    fs::write(root.join("font.ttf"), ARABIC).unwrap();
    let reimport = ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("font.ttf")])
        .unwrap()
        .commit(&mut bus, Actor::import("font-test"), "Reimport font")
        .unwrap();
    assert_eq!(reimport.imports[0].asset.id, id);
    assert_eq!(reimport.imports[0].asset.name, "User name");
    assert_ne!(reimport.imports[0].asset.sha256, first_key);
    store.sync_project(bus.project(), root).unwrap();
    let RuntimeAssetData::Font(old) = retained.data() else {
        panic!()
    };
    assert_eq!(old.bytes(), LATIN);
    bus.undo().unwrap();
    store.sync_project(bus.project(), root).unwrap();
    assert_eq!(source(&store, &id).bytes, LATIN);
    // Watch detects the actual source change and uses the same font cooker/ID.
    assert_eq!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(20))
            .unwrap()
            .pending,
        1
    );
    let watched = watcher
        .poll(&mut bus, start + Duration::from_millis(40))
        .unwrap();
    assert_eq!(watched.imports.len(), 1);
    assert_eq!(watched.imports[0].asset.id, id);
    assert_eq!(
        watched.imports[0].asset.sha256,
        reimport.imports[0].asset.sha256
    );
    bus.undo().unwrap();
    assert!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(80))
            .unwrap()
            .imports
            .is_empty()
    );
    store.sync_project(bus.project(), root).unwrap();
    let current = store.get(&id).unwrap();
    fs::write(root.join("font.ttf"), b"bad").unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("font.ttf")])
            .is_err()
    );
    assert_eq!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(100))
            .unwrap()
            .pending,
        1
    );
    assert_eq!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(120))
            .unwrap()
            .diagnostics
            .len(),
        1
    );
    assert_eq!(bus.project().assets[&id].sha256, first_key);
    // A changed document pointing to a broken cache cannot displace the store.
    let mut broken = bus.project().clone();
    broken.assets.get_mut(&id).unwrap().sha256 = "f".repeat(64);
    assert!(store.sync_project(&broken, root).is_err());
    assert!(Arc::ptr_eq(&current, &store.get(&id).unwrap()));
    fs::write(root.join("font.ttf"), HEBREW).unwrap();
    watcher
        .poll(&mut bus, start + Duration::from_millis(140))
        .unwrap();
    let recovered = watcher
        .poll(&mut bus, start + Duration::from_millis(160))
        .unwrap();
    assert_eq!(recovered.imports.len(), 1);
    assert_eq!(recovered.cleared, vec![id.clone()]);
    store.sync_project(bus.project(), root).unwrap();
    assert_eq!(source(&store, &id).bytes, HEBREW);
}
#[test]
fn mixed_bad_font_batch_stale_preparation_and_bad_options_are_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("font.ttf"), LATIN).unwrap();
    fs::write(root.join("bad.otf"), b"bad").unwrap();
    image::RgbaImage::new(1, 1)
        .save(root.join("image.png"))
        .unwrap();
    let mut bus = CommandBus::new(Project::empty("Fonts")).unwrap();
    let before = bus.project().clone();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(
                root,
                None,
                &[
                    request("image.png"),
                    request("font.ttf"),
                    request("bad.otf")
                ]
            )
            .is_err()
    );
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    let mut wrong = request("font.ttf");
    wrong.texture_usage = Some(incant_assets::TextureUsage::Color);
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[wrong])
            .is_err()
    );
    let stale = ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("font.ttf")])
        .unwrap();
    ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("image.png")])
        .unwrap()
        .commit(&mut bus, Actor::import("test"), "Import image")
        .unwrap();
    let changed = bus.project().clone();
    assert!(
        stale
            .commit(&mut bus, Actor::import("test"), "Stale font")
            .is_err()
    );
    assert_eq!(bus.project(), &changed);
    assert_eq!(bus.history().len(), 1);
    let mut watcher = SourceWatcher::new(root, Duration::ZERO);
    let now = Instant::now();
    let stale_watch = watcher.prepare(ImportSnapshot::capture(&bus), now);
    bus.undo().unwrap();
    let undone = bus.project().clone();
    assert!(watcher.commit(stale_watch, &mut bus).is_err());
    assert_eq!(bus.project(), &undone);
    assert_eq!(bus.redo_history().count(), 1);
}
#[test]
#[cfg(unix)]
fn font_import_and_runtime_refuse_project_cache_and_source_symlink_escape() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(outside.path().join("font.ttf"), LATIN).unwrap();
    symlink(outside.path().join("font.ttf"), root.join("font.ttf")).unwrap();
    let mut bus = CommandBus::new(Project::empty("Fonts")).unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("font.ttf")])
            .is_err()
    );
    fs::remove_file(root.join("font.ttf")).unwrap();
    fs::write(root.join("font.ttf"), LATIN).unwrap();
    ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("font.ttf")])
        .unwrap()
        .commit(&mut bus, Actor::import("test"), "Font")
        .unwrap();
    let mut store = AssetStore::default();
    store.sync_project(bus.project(), root).unwrap();
    let id = bus.project().assets.keys().next().unwrap();
    let retained = store.get(id).unwrap();
    fs::rename(
        root.join(".incant/cache/fonts"),
        outside.path().join("fonts"),
    )
    .unwrap();
    symlink(
        outside.path().join("fonts"),
        root.join(".incant/cache/fonts"),
    )
    .unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("font.ttf")])
            .is_err()
    );
    assert!(
        AssetStore::default()
            .sync_project(bus.project(), root)
            .is_err()
    );
    assert!(Arc::ptr_eq(&retained, &store.get(id).unwrap()));
}
