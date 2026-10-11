use incant_cmd::{Actor, Command, CommandBus, CommandError, SourceError, SourceVersion};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;
use std::fs;
fn fixture() -> (tempfile::TempDir, Project, ScriptSource, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Source transactions");
    let mut scene = Scene::new("World");
    let entity = Entity::new("Player");
    let ids = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let script = ScriptSource {
        id: new_id(),
        name: "Movement".into(),
        path: "main.ts".into(),
    };
    (dir, project, script, ids.0, ids.1)
}
fn write(script: &ScriptSource, expected: Option<&str>, source: &str) -> Command {
    Command::WriteScript {
        script_id: script.id.clone(),
        expected_source: SourceVersion::of(expected),
        source: source.into(),
    }
}
fn open(dir: &tempfile::TempDir, project: Project) -> CommandBus {
    CommandBus::persistent_with_sources(dir.path().join("journal"), project, dir.path()).unwrap()
}
#[test]
fn creation_registration_and_attachment_share_one_atomic_history_and_actor() {
    for reverse in [false, true] {
        let (dir, project, script, scene, entity) = fixture();
        let mut bus = open(&dir, project.clone());
        let mut commands = vec![
            write(&script, None, "export const speed = 3;"),
            Command::SetComponent {
                scene_id: scene.clone(),
                entity_id: entity.clone(),
                component: "Script".into(),
                value: json!({"source":script.id,"props":{}}),
            },
            Command::UpsertScript {
                script: script.clone(),
            },
        ];
        if reverse {
            commands.reverse();
        }
        let actor = Actor::agent("fixture-model", "source-authoring");
        let tx = bus
            .execute(commands, actor.clone(), "Create movement", Some(0))
            .unwrap()
            .clone();
        assert_eq!(tx.actor, actor);
        assert_eq!(tx.source_edit.as_ref().unwrap().before, None);
        assert_eq!(bus.revision(), 1);
        assert_eq!(
            bus.project().scenes[&scene].entities[&entity]
                .provenance
                .as_ref()
                .unwrap()
                .transaction,
            tx.id
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("main.ts")).unwrap(),
            "export const speed = 3;"
        );
        assert_eq!(bus.undo().unwrap(), tx.id);
        assert!(!dir.path().join("main.ts").exists());
        assert_eq!(bus.project(), &project);
        assert_eq!(bus.redo().unwrap(), tx.id);
        let after = bus.project().clone();
        drop(bus);
        let bus = open(&dir, project);
        assert_eq!(bus.project(), &after);
        assert_eq!(bus.revision(), 3);
        assert_eq!(bus.history(), &[tx]);
    }
}
#[test]
fn replacement_adopts_external_preimage_and_undo_redo_preserve_it() {
    let (dir, mut project, script, _, _) = fixture();
    project.scripts.insert(script.id.clone(), script.clone());
    fs::write(dir.path().join("main.ts"), "external predecessor").unwrap();
    let mut bus = open(&dir, project.clone());
    let read = bus.read_script(&script.id).unwrap();
    assert_eq!(read.matches_history, None);
    bus.execute(
        vec![write(
            &script,
            Some("external predecessor"),
            "agent revision",
        )],
        Actor::agent("model", "turn"),
        "Edit",
        Some(0),
    )
    .unwrap();
    assert_eq!(
        bus.read_script(&script.id).unwrap().matches_history,
        Some(true)
    );
    bus.undo().unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("main.ts")).unwrap(),
        "external predecessor"
    );
    bus.redo().unwrap();
    fs::write(dir.path().join("main.ts"), "outside edit after commit").unwrap();
    let revision = bus.revision();
    assert!(matches!(
        bus.undo(),
        Err(CommandError::Source(SourceError::Conflict))
    ));
    assert_eq!(bus.revision(), revision);
    assert_eq!(bus.history().len(), 1);
    drop(bus);
    let mut bus = open(&dir, project);
    assert_eq!(bus.revision(), revision);
    let read = bus.read_script(&script.id).unwrap();
    assert_eq!(read.source.as_deref(), Some("outside edit after commit"));
    assert_eq!(read.matches_history, Some(false));
    bus.execute(
        vec![write(&script, read.source.as_deref(), "new proposal")],
        Actor::user("author"),
        "Adopt outside edit",
        Some(revision),
    )
    .unwrap();
    bus.undo().unwrap();
    assert_eq!(bus.read_script(&script.id).unwrap().source, read.source);
}
#[test]
fn refusal_leaves_disk_document_revision_and_history_unchanged() {
    let (dir, project, script, _, _) = fixture();
    let mut bus = open(&dir, project.clone());
    let cases = [
        vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            write(&script, Some("stale"), "new"),
        ],
        vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            write(&script, None, "new"),
            write(&script, None, "second"),
        ],
        vec![
            Command::SetMemory {
                section: "prefix".into(),
                text: "must not commit".into(),
            },
            write(&script, None, "unregistered"),
        ],
        vec![
            Command::UpsertScript {
                script: ScriptSource {
                    path: "missing/main.ts".into(),
                    ..script.clone()
                },
            },
            write(&script, None, "new"),
        ],
        vec![
            Command::UpsertScript {
                script: ScriptSource {
                    path: "main.d.ts".into(),
                    ..script.clone()
                },
            },
            write(&script, None, "new"),
        ],
        vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            write(&script, None, &"x".repeat(256 * 1024 + 1)),
        ],
    ];
    for commands in cases {
        assert!(
            bus.execute(commands, Actor::user("author"), "Refuse", Some(0))
                .is_err()
        );
        assert_eq!(bus.project(), &project);
        assert_eq!(bus.revision(), 0);
        assert!(bus.history().is_empty());
        assert!(!dir.path().join("main.ts").exists());
        bus.ensure_sources_ready().unwrap();
    }
    assert!(
        bus.execute(
            vec![
                Command::UpsertScript {
                    script: script.clone()
                },
                write(&script, None, "new")
            ],
            Actor::user("author"),
            "No revision",
            None
        )
        .is_err()
    );
    assert!(
        bus.execute(
            vec![
                Command::UpsertScript {
                    script: script.clone()
                },
                write(&script, None, "new")
            ],
            Actor::user("author"),
            "Stale revision",
            Some(2)
        )
        .is_err()
    );
    assert_eq!(bus.revision(), 0);
}
#[test]
fn metadata_removal_does_not_delete_files_and_source_independent_buses_refuse_writes() {
    let (dir, mut project, script, _, _) = fixture();
    project.scripts.insert(script.id.clone(), script.clone());
    let mut bus = open(&dir, project.clone());
    bus.execute(
        vec![write(&script, None, "source")],
        Actor::user("author"),
        "Create",
        Some(0),
    )
    .unwrap();
    bus.execute(
        vec![Command::RemoveScript {
            script_id: script.id.clone(),
        }],
        Actor::user("author"),
        "Unregister",
        Some(1),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("main.ts")).unwrap(),
        "source"
    );
    bus.undo().unwrap();
    for mut memory in [
        CommandBus::new(project.clone()).unwrap(),
        CommandBus::simulation(project.clone()).unwrap(),
        CommandBus::from_crdt(&bus.export_crdt().unwrap()).unwrap(),
    ] {
        assert!(!memory.has_source_store());
        assert!(matches!(
            memory.execute(
                vec![write(&script, None, "source")],
                Actor::user("author"),
                "No capability",
                Some(0)
            ),
            Err(CommandError::Source(SourceError::Unavailable))
        ));
        assert_eq!(memory.revision(), 0);
    }
}
#[test]
fn committed_history_replay_never_rewrites_files_and_missing_capability_cannot_undo() {
    let (dir, mut project, script, _, _) = fixture();
    project.scripts.insert(script.id.clone(), script.clone());
    let mut bus = open(&dir, project.clone());
    bus.execute(
        vec![write(&script, None, "first")],
        Actor::user("author"),
        "Create",
        Some(0),
    )
    .unwrap();
    bus.execute(
        vec![write(&script, Some("first"), "second")],
        Actor::user("author"),
        "Replace",
        Some(1),
    )
    .unwrap();
    drop(bus);
    fs::remove_file(dir.path().join("main.ts")).unwrap();
    let mut bus = CommandBus::persistent(dir.path().join("journal"), project.clone()).unwrap();
    assert!(matches!(
        bus.undo(),
        Err(CommandError::Source(SourceError::Unavailable))
    ));
    drop(bus);
    let bus = open(&dir, project);
    assert_eq!(bus.revision(), 2);
    assert_eq!(
        bus.read_script(&script.id).unwrap().version,
        SourceVersion::Missing
    );
    assert!(!dir.path().join("main.ts").exists());
}
