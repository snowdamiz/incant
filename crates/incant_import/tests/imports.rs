use image::{Rgba, RgbaImage};
use incant_assets::{AssetStore, TextureUsage};
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{AssetImportSettings, Origin, Project, Scene, new_id};
use incant_import::{ImportError, ImportRequest, ImportSnapshot, MAX_BATCH_IMPORTS};
use std::{fs, path::Path};

fn image(root: &Path, name: &str, value: u8) {
    RgbaImage::from_pixel(1, 1, Rgba([value, 128, 255, 255]))
        .save(root.join(name))
        .unwrap();
}
fn request(name: &str) -> ImportRequest {
    ImportRequest {
        source: name.into(),
        texture_usage: None,
    }
}
fn prepare(bus: &CommandBus, root: &Path, names: &[&str]) -> incant_import::PreparedImports {
    ImportSnapshot::capture(bus)
        .prepare(
            root,
            None,
            &names.iter().map(|name| request(name)).collect::<Vec<_>>(),
        )
        .unwrap()
}

#[test]
fn batch_is_one_reversible_persistent_transaction_and_loads_without_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "a.png", 10);
    image(root, "b.png", 20);
    let original = Project::empty("Batch");
    let journal = root.join("history.jsonl");
    let mut bus = CommandBus::persistent(&journal, original.clone()).unwrap();
    let snapshot = ImportSnapshot::capture(&bus);
    let source_root = root.to_path_buf();
    // The preparation owns a snapshot and can run without borrowing/locking the bus.
    let staged = std::thread::spawn(move || {
        snapshot.prepare(&source_root, None, &[request("a.png"), request("b.png")])
    })
    .join()
    .unwrap()
    .unwrap();
    assert_eq!(bus.project(), &original);
    assert!(bus.history().is_empty());
    let committed = staged
        .commit(
            &mut bus,
            Actor::agent("test-model", "test-conversation"),
            "Import two images",
        )
        .unwrap();
    assert_eq!(committed.imports.len(), 2);
    assert_eq!(bus.history().len(), 1);
    assert_eq!(bus.history()[0].commands.len(), 2);
    assert_eq!(bus.history()[0].actor.origin, Origin::Agent);
    let imported = bus.project().clone();
    bus.undo().unwrap();
    assert_eq!(bus.project(), &original);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &imported);
    drop(bus);
    let recovered = CommandBus::persistent(&journal, original).unwrap();
    assert_eq!(recovered.project(), &imported);
    fs::remove_file(root.join("a.png")).unwrap();
    fs::remove_file(root.join("b.png")).unwrap();
    let mut runtime = AssetStore::default();
    assert_eq!(
        runtime
            .sync(recovered.project(), &root.join(".incant/cache"))
            .unwrap()
            .loaded
            .len(),
        2
    );
}

#[test]
fn cache_rebuild_and_reimport_preserve_identity_name_and_usage() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "normal.png", 10);
    let mut bus = CommandBus::new(Project::empty("Reimport")).unwrap();
    let staged = ImportSnapshot::capture(&bus)
        .prepare(
            root,
            None,
            &[ImportRequest {
                source: "normal.png".into(),
                texture_usage: Some(TextureUsage::Normal),
            }],
        )
        .unwrap();
    staged
        .commit(&mut bus, Actor::import("test"), "Import")
        .unwrap();
    let mut renamed = bus.project().assets.values().next().unwrap().clone();
    renamed.name = "User's surface".into();
    bus.execute(
        vec![Command::UpsertAsset {
            asset: renamed.clone(),
        }],
        Actor::user("test"),
        "Rename",
        None,
    )
    .unwrap();
    fs::remove_dir_all(root.join(".incant/cache/textures")).unwrap();
    let staged = prepare(&bus, root, &["normal.png"]);
    assert_eq!(staged.outcomes()[0].asset, renamed);
    assert!(!staged.outcomes()[0].cache_hit);
    let revision = bus.revision();
    staged
        .commit(&mut bus, Actor::import("test"), "Rebuild")
        .unwrap();
    assert_eq!(bus.revision(), revision);
    assert_eq!(bus.history().len(), 2);
    image(root, "normal.png", 100);
    let staged = prepare(&bus, root, &["normal.png"]);
    let changed = &staged.outcomes()[0];
    assert_eq!(changed.asset.id, renamed.id);
    assert_eq!(changed.asset.name, renamed.name);
    assert_eq!(
        changed.asset.import_settings,
        Some(AssetImportSettings::Texture {
            usage: TextureUsage::Normal
        })
    );
    assert_ne!(changed.asset.sha256, renamed.sha256);
    staged
        .commit(&mut bus, Actor::import("test"), "Reimport")
        .unwrap();
    bus.undo().unwrap();
    assert_eq!(bus.project().assets[&renamed.id], renamed);
}

#[test]
fn a_failed_batch_or_rejected_actor_never_partially_edits_the_document() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "good.png", 5);
    fs::write(root.join("bad.png"), b"not an image").unwrap();
    let mut bus = CommandBus::new(Project::empty("Atomic")).unwrap();
    let before = bus.project().clone();
    let result = ImportSnapshot::capture(&bus).prepare(
        root,
        None,
        &[request("good.png"), request("bad.png")],
    );
    assert!(matches!(result, Err(ImportError::Cook { path, .. }) if path == "bad.png"));
    assert_eq!(bus.project(), &before);
    assert!(bus.history().is_empty());
    let prepared = prepare(&bus, root, &["good.png"]);
    assert!(
        prepared
            .commit(&mut bus, Actor::agent("", ""), "Invalid provenance")
            .is_err()
    );
    assert_eq!(bus.project(), &before);
    assert!(bus.history().is_empty());
}

#[test]
fn preparation_rejects_stale_revisions_and_other_documents_even_for_noops() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root, "a.png", 5);
    let mut bus = CommandBus::new(Project::empty("Original")).unwrap();
    let staged = prepare(&bus, root, &["a.png"]);
    bus.execute(
        vec![Command::CreateScene {
            scene: Scene::new("Concurrent"),
        }],
        Actor::user("test"),
        "Add scene",
        None,
    )
    .unwrap();
    assert!(matches!(
        staged.commit(&mut bus, Actor::import("test"), "Import"),
        Err(ImportError::Command(CommandError::Conflict { .. }))
    ));
    assert!(bus.project().assets.is_empty());
    assert_eq!(bus.project().scenes.len(), 1);
    prepare(&bus, root, &["a.png"])
        .commit(&mut bus, Actor::import("test"), "Retry")
        .unwrap();
    let unchanged = prepare(&bus, root, &["a.png"]);
    assert!(!unchanged.outcomes()[0].changed);
    bus.undo().unwrap();
    assert!(matches!(
        unchanged.commit(&mut bus, Actor::import("test"), "No-op"),
        Err(ImportError::Command(CommandError::Conflict { .. }))
    ));
    let different = prepare(&bus, root, &["a.png"]);
    let mut other = CommandBus::new(Project::empty("Other")).unwrap();
    assert!(matches!(
        different.commit(&mut other, Actor::import("test"), "Wrong project"),
        Err(ImportError::DifferentProject)
    ));
    let initial = Project::empty("Same ID");
    let fresh = CommandBus::new(initial.clone()).unwrap();
    let staged = prepare(&fresh, root, &["a.png"]);
    let mut changed = initial;
    changed.name = "Different contents, same identity and revision".into();
    let mut other = CommandBus::new(changed).unwrap();
    assert!(matches!(
        staged.commit(&mut other, Actor::import("test"), "Wrong snapshot"),
        Err(ImportError::ChangedProject)
    ));
}

#[test]
fn malformed_or_ambiguous_requests_fail_before_cooking() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut bus = CommandBus::new(Project::empty("Validation")).unwrap();
    for source in [
        "",
        "./a.png",
        "../a.png",
        "/a.png",
        "C:\\a.png",
        "dir//a.png",
        "dir/a.png/",
        "a:hi.png",
        "\0.png",
    ] {
        assert!(matches!(
            ImportSnapshot::capture(&bus).prepare(root, None, &[request(source)]),
            Err(ImportError::Path(_))
        ));
    }
    for requests in [vec![], vec![request("a.png"); MAX_BATCH_IMPORTS + 1]] {
        assert!(matches!(
            ImportSnapshot::capture(&bus).prepare(root, None, &requests),
            Err(ImportError::BatchSize)
        ));
    }
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare(root, None, &[request("a.png"), request("a.png")]),
        Err(ImportError::DuplicateSource(_))
    ));
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare(root, None, &[request("a.fbx")]),
        Err(ImportError::Unsupported(_))
    ));
    let mut model = request("a.gltf");
    model.texture_usage = Some(TextureUsage::Normal);
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare(root, None, &[model]),
        Err(ImportError::ModelUsage(_))
    ));
    image(root, "a.png", 5);
    prepare(&bus, root, &["a.png"])
        .commit(&mut bus, Actor::import("test"), "Import")
        .unwrap();
    let mut duplicate = bus.project().assets.values().next().unwrap().clone();
    duplicate.id = new_id();
    bus.execute(
        vec![Command::UpsertAsset { asset: duplicate }],
        Actor::user("test"),
        "Duplicate",
        None,
    )
    .unwrap();
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare(root, None, &[request("a.png")]),
        Err(ImportError::AmbiguousSource(_))
    ));
}
