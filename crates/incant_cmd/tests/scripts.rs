use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;

fn fixture() -> (Project, String, String, ScriptSource) {
    let mut project = Project::empty("Script registration");
    let mut scene = Scene::new("World");
    let entity = Entity::new("Player");
    let ids = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let script = ScriptSource {
        id: new_id(),
        name: "Movement".into(),
        path: "scripts/movement.ts".into(),
    };
    (project, ids.0, ids.1, script)
}

fn attach(scene: &str, entity: &str, script: &str) -> Command {
    Command::SetComponent {
        scene_id: scene.into(),
        entity_id: entity.into(),
        component: "Script".into(),
        value: json!({"source":script,"props":{"speed":3}}),
    }
}

fn detach(scene: &str, entity: &str) -> Command {
    Command::RemoveComponent {
        scene_id: scene.into(),
        entity_id: entity.into(),
        component: "Script".into(),
    }
}

#[test]
fn registration_and_attachment_are_one_reversible_transaction_in_either_order() {
    for reverse in [false, true] {
        let (project, scene, entity, script) = fixture();
        let mut bus = CommandBus::new(project.clone()).unwrap();
        let mut commands = vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            attach(&scene, &entity, &script.id),
        ];
        if reverse {
            commands.reverse();
        }
        let actor = Actor::agent("fixture-model", "register-script");
        let transaction = bus
            .execute(commands.clone(), actor.clone(), "Attach movement", Some(0))
            .unwrap()
            .clone();
        assert_eq!(transaction.commands, commands);
        assert_eq!(transaction.actor, actor);
        assert_eq!(bus.project().scripts[&script.id], script);
        assert_eq!(bus.revision(), 1);
        assert_eq!(bus.history().len(), 1);
        let provenance = bus.project().scenes[&scene].entities[&entity]
            .provenance
            .as_ref()
            .unwrap();
        assert_eq!(provenance.transaction, transaction.id);
        assert_eq!(provenance.origin, actor.origin);
        assert_eq!(provenance.actor, actor.actor);
        assert_eq!(provenance.model, actor.model);
        assert_eq!(provenance.conversation_id, actor.conversation_id);
        let attached = bus.project().clone();
        assert_eq!(bus.undo().unwrap(), transaction.id);
        assert_eq!(bus.project(), &project);
        assert_eq!(bus.redo().unwrap(), transaction.id);
        assert_eq!(bus.project(), &attached);
    }
}

#[test]
fn replacement_preserves_identity_and_restores_metadata_on_undo_and_redo() {
    let (project, scene, entity, mut script) = fixture();
    let mut bus = CommandBus::new(project).unwrap();
    bus.execute(
        vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            attach(&scene, &entity, &script.id),
        ],
        Actor::user("author"),
        "Register",
        Some(0),
    )
    .unwrap();
    let before = bus.project().clone();
    // Keep the document's existing semantics: metadata does not need a file,
    // a particular extension, or a nonempty display name to be registered.
    script.path = "sources/future movement.source".into();
    script.name.clear();
    bus.execute(
        vec![Command::UpsertScript {
            script: script.clone(),
        }],
        Actor::agent("fixture-model", "replace-script"),
        "Replace registered metadata",
        Some(1),
    )
    .unwrap();
    assert_eq!(bus.project().scripts.len(), 1);
    assert_eq!(bus.project().scripts[&script.id], script);
    assert_eq!(bus.project().scenes, before.scenes);
    let after = bus.project().clone();
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &after);
}

#[test]
fn removal_and_detachment_are_atomic_and_reversible_in_either_order() {
    for reverse in [false, true] {
        let (project, scene, entity, script) = fixture();
        let mut bus = CommandBus::new(project).unwrap();
        bus.execute(
            vec![
                Command::UpsertScript {
                    script: script.clone(),
                },
                attach(&scene, &entity, &script.id),
            ],
            Actor::user("author"),
            "Register",
            Some(0),
        )
        .unwrap();
        let before = bus.project().clone();
        let mut commands = vec![
            Command::RemoveScript {
                script_id: script.id,
            },
            detach(&scene, &entity),
        ];
        if reverse {
            commands.reverse();
        }
        bus.execute(commands, Actor::user("author"), "Remove", Some(1))
            .unwrap();
        assert!(bus.project().scripts.is_empty());
        assert!(
            !bus.project().scenes[&scene].entities[&entity]
                .components
                .contains_key("Script")
        );
        let removed = bus.project().clone();
        bus.undo().unwrap();
        assert_eq!(bus.project(), &before);
        bus.redo().unwrap();
        assert_eq!(bus.project(), &removed);
    }
}

fn assert_refused(bus: &mut CommandBus, command: Command, reason: &str) {
    let before = bus.project().clone();
    let revision = bus.revision();
    let history = bus.history().to_vec();
    let redo: Vec<_> = bus.redo_history().cloned().collect();
    let crdt = bus.export_crdt().unwrap();
    let error = bus
        .execute(
            vec![
                Command::SetMemory {
                    section: "must-not-commit".into(),
                    text: "Rejected transaction prefix".into(),
                },
                command,
            ],
            Actor::agent("fixture-model", "invalid-script"),
            "Reject entire transaction",
            Some(revision),
        )
        .unwrap_err();
    assert!(error.to_string().contains(reason), "{error}");
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), revision);
    assert_eq!(bus.history(), history);
    assert_eq!(bus.redo_history().cloned().collect::<Vec<_>>(), redo);
    assert_eq!(bus.export_crdt().unwrap(), crdt);
}

#[test]
fn invalid_metadata_and_references_preserve_project_revision_history_and_redo() {
    let (project, scene, entity, script) = fixture();
    let mut bus = CommandBus::new(project).unwrap();
    bus.execute(
        vec![
            Command::UpsertScript {
                script: script.clone(),
            },
            attach(&scene, &entity, &script.id),
        ],
        Actor::user("author"),
        "Register",
        Some(0),
    )
    .unwrap();
    bus.execute(
        vec![Command::SetMemory {
            section: "redo".into(),
            text: "Preserve this transaction".into(),
        }],
        Actor::user("author"),
        "Retain redo",
        Some(1),
    )
    .unwrap();
    bus.undo().unwrap();
    for path in [
        "",
        "/outside.ts",
        "../outside.ts",
        "scripts/../outside.ts",
        "scripts\\move.ts",
        "C:/move.ts",
        "scripts//move.ts",
        "./move.ts",
        "scripts/move\0.ts",
    ] {
        let mut invalid = script.clone();
        invalid.path = path.into();
        assert_refused(
            &mut bus,
            Command::UpsertScript { script: invalid },
            "portable relative path",
        );
    }
    let mut invalid = script.clone();
    invalid.id = "not-a-ulid".into();
    assert_refused(
        &mut bus,
        Command::UpsertScript { script: invalid },
        "invalid ULID",
    );
    let mut collision = script.clone();
    collision.id = entity.clone();
    assert_refused(
        &mut bus,
        Command::UpsertScript { script: collision },
        "globally unique",
    );
    for missing in [new_id(), "not-a-ulid".into()] {
        assert_refused(
            &mut bus,
            Command::RemoveScript { script_id: missing },
            "script does not exist",
        );
    }
    assert_refused(
        &mut bus,
        attach(&scene, &entity, &new_id()),
        "script source does not exist",
    );
    assert_refused(
        &mut bus,
        Command::RemoveScript {
            script_id: script.id,
        },
        "script source does not exist",
    );
}

#[test]
fn stale_revision_and_invalid_actor_cannot_register_a_source() {
    let (project, _, _, script) = fixture();
    let mut bus = CommandBus::new(project.clone()).unwrap();
    let commands = vec![Command::UpsertScript { script }];
    assert!(matches!(
        bus.execute(commands.clone(), Actor::user("author"), "Stale", Some(1)),
        Err(CommandError::Conflict {
            expected: 1,
            actual: 0
        })
    ));
    assert!(
        bus.execute(commands, Actor::agent("", ""), "Invalid actor", Some(0))
            .is_err()
    );
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
}

#[test]
fn journal_replays_registration_replacement_removal_and_original_actors() {
    let (project, scene, entity, mut script) = fixture();
    let path = std::env::temp_dir().join(format!("incant-scripts-{}.jsonl", new_id()));
    let actor = Actor::agent("fixture-model", "journal-script");
    let (registered, history, replacement);
    {
        let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
        bus.execute(
            vec![
                Command::UpsertScript {
                    script: script.clone(),
                },
                attach(&scene, &entity, &script.id),
            ],
            actor.clone(),
            "Register and attach",
            Some(0),
        )
        .unwrap();
        registered = bus.project().clone();
        let registered_history = bus.history().to_vec();
        // Windows enforces the journal's exclusive lock for reads as well.
        drop(bus);
        let journal = std::fs::read(&path).unwrap();
        let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
        assert_eq!(bus.project(), &registered);
        assert_eq!(bus.history(), registered_history);
        assert_eq!(bus.revision(), 1);
        assert!(!bus.can_redo());
        assert_refused(
            &mut bus,
            Command::RemoveScript {
                script_id: script.id.clone(),
            },
            "script source does not exist",
        );
        drop(bus);
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
        assert_eq!(bus.project(), &registered);
        assert_eq!(bus.history(), registered_history);
        assert_eq!(bus.revision(), 1);
        assert!(!bus.can_redo());
        script.path = "scripts/replacement.ts".into();
        bus.execute(
            vec![Command::UpsertScript {
                script: script.clone(),
            }],
            actor.clone(),
            "Replace",
            Some(1),
        )
        .unwrap();
        replacement = bus.history()[1].clone();
        bus.undo().unwrap();
        history = bus.history().to_vec();
    }
    let (removed, removed_history, revision);
    {
        let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
        assert_eq!(bus.project(), &registered);
        assert_eq!(bus.history(), history);
        assert_eq!(bus.revision(), 3);
        assert_eq!(
            bus.redo_history().cloned().collect::<Vec<_>>(),
            vec![replacement.clone()]
        );
        assert_eq!(bus.redo().unwrap(), replacement.id);
        assert_eq!(bus.project().scripts[&script.id], script);
        bus.execute(
            vec![
                detach(&scene, &entity),
                Command::RemoveScript {
                    script_id: script.id,
                },
            ],
            actor.clone(),
            "Detach and remove",
            Some(4),
        )
        .unwrap();
        removed = bus.project().clone();
        bus.undo().unwrap();
        bus.redo().unwrap();
        removed_history = bus.history().to_vec();
        revision = bus.revision();
    }
    {
        let bus = CommandBus::persistent(&path, project).unwrap();
        assert_eq!(bus.project(), &removed);
        assert_eq!(bus.history(), removed_history);
        assert_eq!(bus.revision(), revision);
        assert!(bus.history().iter().all(|tx| tx.actor == actor));
        assert!(bus.project().scripts.is_empty());
        assert!(!bus.can_redo());
    }
    std::fs::remove_file(path).unwrap();
}
