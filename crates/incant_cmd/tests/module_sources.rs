use incant_cmd::{Actor, Command, CommandBus, CommandError, SourceError, SourceVersion};
use incant_doc::{Project, RustModuleSource, ScriptSource, new_id};
use incant_source::SourceKind;
use std::fs;

const TEXT: &str = "pub fn update() -> u32 { 42 }\n";
fn fixture() -> (tempfile::TempDir, Project, RustModuleSource) {
    let dir = tempfile::tempdir().unwrap();
    // Existing package directory is a host prerequisite, not a bus-created path.
    fs::create_dir(dir.path().join("movement")).unwrap();
    let module = RustModuleSource {
        id: new_id(),
        name: "Movement".into(),
        path: "movement/main.rs".into(),
    };
    (dir, Project::empty("Registered Rust"), module)
}
fn open(dir: &tempfile::TempDir, project: Project) -> CommandBus {
    CommandBus::persistent_with_sources(dir.path().join("journal"), project, dir.path()).unwrap()
}
fn register(module: &RustModuleSource) -> Command {
    Command::UpsertModuleSource {
        module: module.clone(),
    }
}
fn write(module: &RustModuleSource, expected: Option<&str>, source: &str) -> Command {
    Command::WriteModuleSource {
        module_id: module.id.clone(),
        expected_source: SourceVersion::of(expected),
        source: source.into(),
    }
}
fn apply(bus: &mut CommandBus, commands: Vec<Command>) -> Result<(), CommandError> {
    bus.execute(
        commands,
        Actor::agent("fixture-model", "module-turn"),
        "Author Rust",
        Some(bus.revision()),
    )?;
    Ok(())
}
fn unchanged(bus: &CommandBus, project: &Project, dir: &tempfile::TempDir) {
    assert_eq!(bus.project(), project);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert!(!bus.can_redo());
    assert!(!dir.path().join("movement/main.rs").exists());
    bus.ensure_sources_ready().unwrap();
}

#[test]
fn registration_and_rust_source_publish_atomically_with_actor_and_durable_undo_redo() {
    for reverse in [false, true] {
        let (dir, project, module) = fixture();
        let mut bus = open(&dir, project.clone());
        let mut commands = vec![
            register(&module),
            write(&module, None, TEXT),
            Command::SetMemory {
                section: "module".into(),
                text: "authored".into(),
            },
        ];
        if reverse {
            commands.reverse();
        }
        apply(&mut bus, commands).unwrap();
        let tx = bus.history()[0].clone();
        assert_eq!(tx.actor, Actor::agent("fixture-model", "module-turn"));
        let edit = tx.source_edit.as_ref().unwrap();
        assert_eq!(edit.kind, SourceKind::Rust);
        assert_eq!(edit.script_id, module.id);
        assert_eq!(edit.before, None);
        assert_eq!(edit.after.as_deref(), Some(TEXT));
        let read = bus.read_module_source(&module.id).unwrap();
        assert_eq!(read.module_id, module.id);
        assert_eq!(read.path, module.path);
        assert_eq!(read.revision, 1);
        assert_eq!(read.version, SourceVersion::of(Some(TEXT)));
        assert_eq!(read.source.as_deref(), Some(TEXT));
        assert_eq!(read.matches_history, Some(true));
        assert!(bus.project().assets.is_empty());
        assert!(bus.project().scenes.is_empty());
        assert_eq!(bus.undo().unwrap(), tx.id);
        assert_eq!(bus.project(), &project);
        assert!(!dir.path().join(&module.path).exists());
        assert_eq!(bus.redo().unwrap(), tx.id);
        let after = bus.project().clone();
        drop(bus);
        let mut bus = open(&dir, project);
        assert_eq!(bus.project(), &after);
        assert_eq!(bus.revision(), 3);
        assert_eq!(bus.history(), &[tx]);
        assert_eq!(
            fs::read_to_string(dir.path().join(&module.path)).unwrap(),
            TEXT
        );
        bus.undo().unwrap();
        assert!(!dir.path().join(&module.path).exists());
    }
}

#[test]
fn external_preimages_and_postcommit_conflicts_remain_observable() {
    let (dir, mut project, module) = fixture();
    project
        .module_sources
        .insert(module.id.clone(), module.clone());
    fs::write(dir.path().join(&module.path), "// external predecessor").unwrap();
    let mut bus = open(&dir, project.clone());
    let read = bus.read_module_source(&module.id).unwrap();
    assert_eq!(read.matches_history, None);
    apply(&mut bus, vec![write(&module, read.source.as_deref(), TEXT)]).unwrap();
    bus.undo().unwrap();
    assert_eq!(
        bus.read_module_source(&module.id).unwrap().source,
        read.source
    );
    bus.redo().unwrap();
    fs::write(dir.path().join(&module.path), "// outside edit").unwrap();
    assert!(matches!(
        bus.undo(),
        Err(CommandError::Source(SourceError::Conflict))
    ));
    assert_eq!(bus.revision(), 3);
    assert_eq!(bus.history().len(), 1);
    drop(bus);
    let mut bus = open(&dir, project);
    let external = bus.read_module_source(&module.id).unwrap();
    assert_eq!(external.source.as_deref(), Some("// outside edit"));
    assert_eq!(external.matches_history, Some(false));
    assert!(apply(&mut bus, vec![write(&module, Some(TEXT), "stale")]).is_err());
    apply(
        &mut bus,
        vec![write(&module, external.source.as_deref(), TEXT)],
    )
    .unwrap();
    bus.undo().unwrap();
    assert_eq!(
        bus.read_module_source(&module.id).unwrap().source,
        external.source
    );
}

#[test]
fn mixed_typescript_rust_and_duplicate_writes_refuse_before_any_publication() {
    for reverse in [false, true] {
        let (dir, project, module) = fixture();
        let script = ScriptSource {
            id: new_id(),
            name: "TS".into(),
            path: "main.ts".into(),
        };
        let mut bus = open(&dir, project.clone());
        let ts = Command::WriteScript {
            script_id: script.id.clone(),
            expected_source: SourceVersion::Missing,
            source: "export const x=1;".into(),
        };
        let mut commands = vec![
            register(&module),
            Command::UpsertScript {
                script: script.clone(),
            },
            write(&module, None, TEXT),
            ts.clone(),
        ];
        if reverse {
            commands.reverse();
        }
        let error = apply(&mut bus, commands).unwrap_err();
        assert!(error.to_string().contains("one source write"));
        unchanged(&bus, &project, &dir);
        assert!(!dir.path().join("main.ts").exists());
        assert!(
            apply(
                &mut bus,
                vec![
                    register(&module),
                    write(&module, None, TEXT),
                    write(&module, None, TEXT)
                ]
            )
            .is_err()
        );
        unchanged(&bus, &project, &dir);
        apply(
            &mut bus,
            vec![
                Command::UpsertScript {
                    script: script.clone(),
                },
                ts,
            ],
        )
        .unwrap();
        assert_eq!(
            bus.read_script(&script.id).unwrap().source.as_deref(),
            Some("export const x=1;")
        );
        let edit = bus.history()[0].source_edit.as_ref().unwrap();
        assert_eq!(edit.kind, SourceKind::TypeScript);
        assert!(serde_json::to_value(edit).unwrap().get("kind").is_none());
    }
}

#[test]
fn invalid_sources_versions_and_final_metadata_leave_no_transaction_prefix() {
    let (dir, project, module) = fixture();
    let mut bus = open(&dir, project.clone());
    for commands in [
        vec![register(&module), write(&module, Some("stale"), TEXT)],
        vec![write(&module, None, TEXT)],
        vec![
            register(&module),
            write(&module, None, TEXT),
            Command::RemoveModuleSource {
                module_id: module.id.clone(),
            },
        ],
        vec![
            register(&RustModuleSource {
                path: "missing/main.rs".into(),
                ..module.clone()
            }),
            write(&module, None, TEXT),
        ],
        vec![
            register(&RustModuleSource {
                path: "main.rs".into(),
                ..module.clone()
            }),
            write(&module, None, TEXT),
        ],
        vec![
            register(&module),
            write(&module, None, &"x".repeat(64 * 1024 + 1)),
        ],
    ] {
        assert!(apply(&mut bus, commands).is_err());
        unchanged(&bus, &project, &dir);
        assert!(!dir.path().join("missing").exists());
    }
    for revision in [None, Some(2)] {
        assert!(
            bus.execute(
                vec![register(&module), write(&module, None, TEXT)],
                Actor::user("author"),
                "Refuse",
                revision
            )
            .is_err()
        );
        unchanged(&bus, &project, &dir);
    }
    assert!(bus.read_module_source(&new_id()).is_err());
}

#[test]
fn unavailable_stores_refuse_and_metadata_removal_never_deletes_bytes() {
    let (dir, mut project, module) = fixture();
    project
        .module_sources
        .insert(module.id.clone(), module.clone());
    let mut bus = open(&dir, project.clone());
    apply(&mut bus, vec![write(&module, None, TEXT)]).unwrap();
    for mut memory in [
        CommandBus::new(project.clone()).unwrap(),
        CommandBus::simulation(project.clone()).unwrap(),
        CommandBus::from_crdt(&bus.export_crdt().unwrap()).unwrap(),
    ] {
        assert!(matches!(
            memory.read_module_source(&module.id),
            Err(CommandError::Source(SourceError::Unavailable))
        ));
        assert!(matches!(
            apply(&mut memory, vec![write(&module, None, TEXT)]),
            Err(CommandError::Source(SourceError::Unavailable))
        ));
        assert_eq!(memory.revision(), 0);
    }
    apply(
        &mut bus,
        vec![Command::RemoveModuleSource {
            module_id: module.id.clone(),
        }],
    )
    .unwrap();
    assert!(bus.read_module_source(&module.id).is_err());
    assert_eq!(
        fs::read_to_string(dir.path().join(&module.path)).unwrap(),
        TEXT
    );
    bus.undo().unwrap();
    assert_eq!(
        bus.read_module_source(&module.id)
            .unwrap()
            .source
            .as_deref(),
        Some(TEXT)
    );
    apply(
        &mut bus,
        vec![register(&RustModuleSource {
            path: "movement/renamed.rs".into(),
            ..module.clone()
        })],
    )
    .unwrap();
    assert_eq!(
        bus.read_module_source(&module.id).unwrap().version,
        SourceVersion::Missing
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(&module.path)).unwrap(),
        TEXT
    );
}

#[test]
fn save_and_reopen_keep_registered_bytes_and_reversible_history() {
    let (dir, project, module) = fixture();
    let path = dir.path().join("game.incant");
    fs::write(&path, project.canonical_text().unwrap()).unwrap();
    let expected = {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        apply(
            &mut bus,
            vec![register(&module), write(&module, None, TEXT)],
        )
        .unwrap();
        let saved = bus.save_project_file(1, Actor::user("save")).unwrap();
        assert!(saved.changed);
        assert_eq!(bus.revision(), 1);
        assert_eq!(bus.history().len(), 1);
        bus.project().clone()
    };
    // Read only after releasing the exclusive project/journal owner (Windows).
    assert_eq!(
        Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap(),
        expected
    );
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        assert_eq!(bus.project(), &expected);
        assert_eq!(bus.history().len(), 1);
        assert_eq!(
            bus.read_module_source(&module.id)
                .unwrap()
                .source
                .as_deref(),
            Some(TEXT)
        );
        bus.undo().unwrap();
        assert_eq!(bus.project(), &project);
        assert!(!dir.path().join(&module.path).exists());
    }
    let mut bus = CommandBus::open_project_file(&path).unwrap();
    assert!(bus.can_redo());
    bus.redo().unwrap();
    assert_eq!(bus.project(), &expected);
    assert_eq!(
        bus.read_module_source(&module.id)
            .unwrap()
            .source
            .as_deref(),
        Some(TEXT)
    );
}

#[test]
fn oversize_reads_and_linked_source_paths_fail_closed() {
    let (dir, mut project, module) = fixture();
    project
        .module_sources
        .insert(module.id.clone(), module.clone());
    fs::write(dir.path().join(&module.path), "x".repeat(64 * 1024 + 1)).unwrap();
    let mut bus = open(&dir, project.clone());
    assert!(bus.read_module_source(&module.id).is_err());
    assert!(apply(&mut bus, vec![write(&module, None, TEXT)]).is_err());
    fs::write(dir.path().join(&module.path), [0xff, 0xfe]).unwrap();
    assert!(bus.read_module_source(&module.id).is_err());
    assert!(apply(&mut bus, vec![write(&module, None, TEXT)]).is_err());
    fs::write(dir.path().join(&module.path), TEXT).unwrap();
    fs::hard_link(
        dir.path().join(&module.path),
        dir.path().join("movement/alias.rs"),
    )
    .unwrap();
    assert!(bus.read_module_source(&module.id).is_err());
    assert!(apply(&mut bus, vec![write(&module, Some(TEXT), "changed")]).is_err());
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
}

#[cfg(unix)]
#[test]
fn symlinked_entry_or_package_is_never_traversed() {
    use std::os::unix::fs::symlink;
    let (dir, mut project, module) = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.rs"), TEXT).unwrap();
    symlink(
        outside.path().join("outside.rs"),
        dir.path().join(&module.path),
    )
    .unwrap();
    project
        .module_sources
        .insert(module.id.clone(), module.clone());
    let mut bus = open(&dir, project.clone());
    assert!(bus.read_module_source(&module.id).is_err());
    assert!(apply(&mut bus, vec![write(&module, Some(TEXT), "changed")]).is_err());
    fs::remove_file(dir.path().join(&module.path)).unwrap();
    fs::remove_dir(dir.path().join("movement")).unwrap();
    symlink(outside.path(), dir.path().join("movement")).unwrap();
    assert!(apply(&mut bus, vec![write(&module, None, TEXT)]).is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("outside.rs")).unwrap(),
        TEXT
    );
    assert!(!outside.path().join("main.rs").exists());
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), 0);
}
