use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Project;

#[test]
fn session_identity_is_stable_across_edits_and_distinct_for_same_project_buses() {
    let project = Project::empty("Session identity");
    let mut bus = CommandBus::new(project.clone()).unwrap();
    let identity = bus.session_id().to_owned();
    assert_ne!(
        identity,
        CommandBus::new(project.clone()).unwrap().session_id()
    );
    assert_ne!(
        identity,
        CommandBus::simulation(project).unwrap().session_id()
    );
    bus.execute(
        vec![Command::SetMemory {
            section: "test".into(),
            text: "value".into(),
        }],
        Actor::user("author"),
        "Edit",
        Some(0),
    )
    .unwrap();
    assert_eq!(bus.session_id(), identity);
    bus.undo().unwrap();
    assert_eq!(bus.session_id(), identity);
    bus.redo().unwrap();
    assert_eq!(bus.session_id(), identity);
    assert!(
        serde_json::to_value(bus.project())
            .unwrap()
            .get("session_id")
            .is_none()
    );
}
#[test]
fn reopening_same_journal_keeps_authored_state_but_retires_session_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal");
    let project = Project::empty("Reopened session");
    let bus = CommandBus::persistent(&path, project.clone()).unwrap();
    let identity = bus.session_id().to_owned();
    let revision = bus.revision();
    drop(bus);
    let bus = CommandBus::persistent(&path, project.clone()).unwrap();
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), revision);
    assert_ne!(bus.session_id(), identity);
}
