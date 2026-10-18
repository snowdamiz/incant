//! An empty saved project authors its first package through ordinary commands.
use incant_cmd::{Actor, Command, CommandBus, SourceVersion};
use incant_doc::{Project, RustModuleSource, new_id};
use std::fs;

const TEXT: &str = "pub fn update() -> u32 { 42 }\n";
fn module() -> RustModuleSource {
    RustModuleSource {
        id: new_id(),
        name: "Movement".into(),
        path: "modules/movement/module.rs".into(),
    }
}
fn commands(module: &RustModuleSource) -> Vec<Command> {
    vec![
        Command::UpsertModuleSource {
            module: module.clone(),
        },
        Command::WriteModuleSource {
            module_id: module.id.clone(),
            expected_source: SourceVersion::Missing,
            source: TEXT.into(),
        },
        Command::SetMemory {
            section: "package".into(),
            text: "authored".into(),
        },
    ]
}
fn create(bus: &mut CommandBus, module: &RustModuleSource) {
    bus.execute(
        commands(module),
        Actor::agent("model", "first-package"),
        "First Rust package",
        Some(bus.revision()),
    )
    .unwrap();
}

#[test]
fn empty_saved_project_creates_first_package_with_reopen_undo_and_redo() {
    let root = tempfile::tempdir().unwrap();
    let project = Project::empty("First package");
    let path = root.path().join("game.incant");
    fs::write(&path, project.canonical_text().unwrap()).unwrap();
    let module = module();
    let transaction = {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        create(&mut bus, &module);
        assert_eq!(bus.revision(), 1);
        assert_eq!(
            bus.read_module_source(&module.id)
                .unwrap()
                .source
                .as_deref(),
            Some(TEXT)
        );
        let tx = bus.history()[0].clone();
        assert_eq!(tx.actor, Actor::agent("model", "first-package"));
        assert!(tx.source_edit.as_ref().unwrap().created_tree.is_some());
        bus.save_project_file(1, Actor::user("save")).unwrap();
        tx
    };
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        assert_eq!(bus.history(), std::slice::from_ref(&transaction));
        assert_eq!(bus.undo().unwrap(), transaction.id);
        assert_eq!(bus.project(), &project);
        assert!(!root.path().join("modules").exists());
    }
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        assert!(bus.can_redo());
        assert_eq!(bus.redo().unwrap(), transaction.id);
        assert_eq!(bus.history(), &[transaction]);
        assert_eq!(
            fs::read(root.path().join(&module.path)).unwrap(),
            TEXT.as_bytes()
        );
        bus.undo().unwrap();
        assert!(!root.path().join("modules").exists());
    }
    // Undo retains the original subtree in recovery storage rather than deleting it.
    let retained: Vec<_> = fs::read_dir(root.path().join(".incant-source-transactions"))
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tree"))
        .collect();
    assert_eq!(retained.len(), 1);
    assert_eq!(
        fs::read(retained[0].path().join("movement/module.rs")).unwrap(),
        TEXT.as_bytes()
    );
}

#[test]
fn preexisting_ancestors_survive_and_ordinary_edits_do_not_lose_tree_ownership() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("modules")).unwrap();
    fs::write(root.path().join("modules/user.txt"), "preexisting").unwrap();
    let mut bus = CommandBus::persistent_with_sources(
        root.path().join("journal"),
        Project::empty("Existing parent"),
        root.path(),
    )
    .unwrap();
    let module = module();
    create(&mut bus, &module);
    bus.execute(
        vec![Command::WriteModuleSource {
            module_id: module.id.clone(),
            expected_source: SourceVersion::of(Some(TEXT)),
            source: "// edited".into(),
        }],
        Actor::user("edit"),
        "Ordinary edit",
        Some(bus.revision()),
    )
    .unwrap();
    assert!(
        bus.history()[1]
            .source_edit
            .as_ref()
            .unwrap()
            .created_tree
            .is_none()
    );
    bus.undo().unwrap();
    bus.undo().unwrap();
    assert!(!root.path().join("modules/movement").exists());
    assert_eq!(
        fs::read(root.path().join("modules/user.txt")).unwrap(),
        b"preexisting"
    );
    bus.redo().unwrap();
    assert_eq!(
        fs::read(root.path().join(&module.path)).unwrap(),
        TEXT.as_bytes()
    );
}

#[test]
fn foreign_additions_and_redo_collisions_leave_history_and_directories_untouched() {
    let root = tempfile::tempdir().unwrap();
    let mut bus = CommandBus::persistent_with_sources(
        root.path().join("journal"),
        Project::empty("Conflicts"),
        root.path(),
    )
    .unwrap();
    let module = module();
    create(&mut bus, &module);
    fs::write(root.path().join("modules/movement/user.rs"), "// foreign").unwrap();
    assert!(bus.undo().is_err());
    assert_eq!(bus.revision(), 1);
    assert_eq!(bus.history().len(), 1);
    assert_eq!(
        fs::read(root.path().join(&module.path)).unwrap(),
        TEXT.as_bytes()
    );
    assert_eq!(
        fs::read(root.path().join("modules/movement/user.rs")).unwrap(),
        b"// foreign"
    );
    // An external author explicitly removes their addition; only then Undo can own the chain.
    fs::remove_file(root.path().join("modules/movement/user.rs")).unwrap();
    bus.undo().unwrap();
    fs::create_dir(root.path().join("modules")).unwrap();
    fs::write(root.path().join("modules/user.txt"), "new root").unwrap();
    assert!(bus.redo().is_err());
    assert_eq!(bus.revision(), 2);
    assert!(bus.can_redo());
    assert_eq!(
        fs::read(root.path().join("modules/user.txt")).unwrap(),
        b"new root"
    );
    assert!(!root.path().join(&module.path).exists());
}

#[test]
fn invalid_revision_cas_or_mixed_write_creates_no_visible_directory_prefix() {
    let root = tempfile::tempdir().unwrap();
    let initial = Project::empty("Refusal");
    let mut bus = CommandBus::persistent_with_sources(
        root.path().join("journal"),
        initial.clone(),
        root.path(),
    )
    .unwrap();
    let module = module();
    for revision in [None, Some(9)] {
        assert!(
            bus.execute(commands(&module), Actor::user("author"), "Refuse", revision)
                .is_err()
        );
    }
    let mut stale = commands(&module);
    if let Command::WriteModuleSource {
        expected_source, ..
    } = &mut stale[1]
    {
        *expected_source = SourceVersion::of(Some("stale"));
    }
    assert!(
        bus.execute(stale, Actor::user("author"), "Stale", Some(0))
            .is_err()
    );
    let mut duplicate = commands(&module);
    duplicate.push(duplicate[1].clone());
    assert!(
        bus.execute(duplicate, Actor::user("author"), "Duplicate", Some(0))
            .is_err()
    );
    assert_eq!(bus.project(), &initial);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert!(!root.path().join("modules").exists());
    bus.ensure_sources_ready().unwrap();
}
