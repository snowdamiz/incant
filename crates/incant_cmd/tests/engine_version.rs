use incant_cmd::{Actor, Command, CommandBus, CommandError, EngineReviewError};
use incant_doc::{EngineRequirement, Project};
use sha2::{Digest, Sha256};

fn legacy() -> Project {
    let mut project = Project::empty("Legacy");
    project.engine_version = None;
    project
}

#[test]
fn explicit_pin_is_atomic_reversible_provenanced_and_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.jsonl");
    let project = legacy();
    let before = project.canonical_text().unwrap();
    let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
    let command = Command::SetEngineVersion {
        requirement: EngineRequirement::exact_current(),
    };
    bus.execute(
        vec![command.clone()],
        Actor::agent("fixture", "pin"),
        "Declare current engine",
        Some(0),
    )
    .unwrap();
    assert_eq!(bus.history()[0].actor.model.as_deref(), Some("fixture"));
    let after = bus.project().canonical_text().unwrap();
    assert_ne!(before, after);
    assert_eq!(bus.revision(), 1);
    drop(bus);
    let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    let revision = bus.revision();
    // Windows enforces the live journal's exclusive lock for separate readers.
    drop(bus);
    let stable = std::fs::read(&path).unwrap();
    let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
    assert_eq!(bus.revision(), revision);
    for commands in [
        vec![Command::SetEngineVersion {
            requirement: EngineRequirement::parse(">=99.0.0").unwrap(),
        }],
        vec![
            command.clone(),
            Command::SetMemory {
                section: String::new(),
                text: "bad later op".into(),
            },
        ],
    ] {
        assert!(
            bus.execute(
                commands,
                Actor::user("test"),
                "Refuse atomically",
                Some(revision)
            )
            .is_err()
        );
        assert_eq!(bus.revision(), revision);
        assert_eq!(bus.project().canonical_text().unwrap(), after);
        drop(bus);
        assert_eq!(std::fs::read(&path).unwrap(), stable);
        bus = CommandBus::persistent(&path, project.clone()).unwrap();
        assert_eq!(bus.revision(), revision);
        assert_eq!(bus.project().canonical_text().unwrap(), after);
    }
    assert!(matches!(
        bus.execute(vec![command], Actor::user("test"), "Stale", Some(0)),
        Err(CommandError::Conflict { .. })
    ));
    drop(bus);
    assert_eq!(std::fs::read(&path).unwrap(), stable);
}

#[test]
fn review_binds_exact_source_identity_revision_and_candidate_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("history.jsonl");
    let project = legacy();
    let source = format!(" {}\n", project.canonical_text().unwrap());
    let bus = CommandBus::persistent(&journal, project.clone()).unwrap();
    drop(bus);
    let old_journal = std::fs::read(&journal).unwrap();
    let mut bus = CommandBus::persistent(&journal, project.clone()).unwrap();
    let plan = bus.review_engine_pin(&source, 0).unwrap();
    assert_eq!(
        plan.source_sha256,
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(plan.source_bytes, source.len());
    assert_eq!(
        plan.candidate_sha256,
        format!("{:x}", Sha256::digest(plan.candidate_text.as_bytes()))
    );
    assert_eq!(plan.previous_requirement, None);
    assert_eq!(plan.target_requirement.as_str(), "=0.0.1");
    assert!(!plan.file_migration_supported);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    drop(bus);
    assert_eq!(std::fs::read(&journal).unwrap(), old_journal);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let mut bus = CommandBus::persistent(&journal, project).unwrap();
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    let mut changed = bus.project().clone();
    changed.name = "Concurrent external edit".into();
    assert!(matches!(
        bus.review_engine_pin(&changed.canonical_text().unwrap(), 0),
        Err(CommandError::EngineReview(EngineReviewError::SourceChanged))
    ));
    changed.id = incant_doc::new_id();
    assert!(
        bus.review_engine_pin(&changed.canonical_text().unwrap(), 0)
            .is_err()
    );
    bus.execute(
        plan.commands,
        Actor::user("reviewer"),
        "Explicit in-memory pin",
        Some(plan.expected_revision),
    )
    .unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), plan.candidate_text);
    assert!(matches!(
        bus.review_engine_pin(&source, 0),
        Err(CommandError::Conflict { .. })
    ));
    assert!(matches!(
        bus.review_engine_pin(&plan.candidate_text, 1),
        Err(CommandError::EngineReview(
            EngineReviewError::AlreadyDeclared
        ))
    ));
    let oversized = " ".repeat(incant_cmd::MAX_ENGINE_REVIEW_BYTES + 1);
    assert!(matches!(
        bus.review_engine_pin(&oversized, 1),
        Err(CommandError::EngineReview(EngineReviewError::Limit))
    ));
}

#[test]
fn pin_command_has_no_file_authority_or_unknown_escape_hatch() {
    for value in [
        serde_json::json!({"op":"set_engine_version","requirement":null}),
        serde_json::json!({"op":"set_engine_version","requirement":"=0.0.1","path":"other.json"}),
        serde_json::json!({"op":"set_engine_version","requirement":"=0.0.1","force":true}),
    ] {
        assert!(serde_json::from_value::<Command>(value).is_err());
    }
}
