use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{PhysicsMode, Project};
use serde_json::json;

#[test]
fn mode_change_has_atomic_provenance_and_survives_journal_reopen_undo_redo() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("history.jsonl");
    let project = Project::empty("Physics settings");
    let original = project.canonical_text().unwrap();
    let mut bus = CommandBus::persistent(&journal, project.clone()).unwrap();
    let command: Command =
        serde_json::from_value(json!({"op":"set_physics_mode", "mode":"fast"})).unwrap();
    let transaction = bus
        .execute(
            vec![command],
            Actor::agent("fixture", "mode-test"),
            "Use fast physics",
            Some(0),
        )
        .unwrap()
        .id
        .clone();
    assert_eq!(bus.project().settings.physics_mode, PhysicsMode::Fast);
    assert_eq!(
        bus.history()[0].actor.conversation_id.as_deref(),
        Some("mode-test")
    );
    let changed = bus.project().canonical_text().unwrap();
    drop(bus);
    let mut bus = CommandBus::persistent(&journal, project.clone()).unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), changed);
    assert_eq!(bus.undo().unwrap(), transaction);
    assert_eq!(bus.project().canonical_text().unwrap(), original);
    drop(bus);
    let mut bus = CommandBus::persistent(&journal, project).unwrap();
    assert_eq!(bus.redo().unwrap(), transaction);
    assert_eq!(bus.project().canonical_text().unwrap(), changed);
    let revision = bus.revision();
    assert!(
        bus.execute(
            vec![
                Command::SetPhysicsMode {
                    mode: PhysicsMode::Deterministic
                },
                Command::SetMemory {
                    section: String::new(),
                    text: "invalid later operation".into()
                }
            ],
            Actor::user("fixture"),
            "Must refuse atomically",
            Some(revision)
        )
        .is_err()
    );
    assert_eq!(bus.revision(), revision);
    assert_eq!(bus.project().canonical_text().unwrap(), changed);
}

#[test]
fn commands_cannot_select_cargo_features_or_an_unrecognized_variant() {
    for value in [
        json!("fast-simd8"),
        json!("fast_simd8"),
        json!(null),
        json!(true),
        json!({"mode":"fast"}),
    ] {
        assert!(
            serde_json::from_value::<Command>(json!({"op":"set_physics_mode", "mode":value}))
                .is_err()
        );
    }
    assert!(
        serde_json::from_value::<Command>(
            json!({"op":"set_physics_mode", "mode":"fast", "features":["arbitrary"]})
        )
        .is_err()
    );
}
