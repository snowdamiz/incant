use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Project;
use serde_json::json;

#[test]
fn input_bindings_use_atomic_undoable_commands_and_preserve_legacy_empty_documents() {
    let project = Project::empty("Bindings");
    let before = project.canonical_text().unwrap();
    assert_eq!(json!(project)["settings"], json!({"tick_rate":60}));
    assert_eq!(
        serde_json::from_str::<Project>(&before)
            .unwrap()
            .canonical_text()
            .unwrap(),
        before
    );
    let mut bus = CommandBus::new(project).unwrap();
    let valid: Command = serde_json::from_value(json!({"op":"set_input_actions","actions":{
        "jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}}))
    .unwrap();
    bus.execute(
        vec![valid],
        Actor::agent("test-model", "bindings-test"),
        "Bind jump",
        Some(0),
    )
    .unwrap();
    let after = bus.project().canonical_text().unwrap();
    assert_ne!(before, after);
    assert_eq!(
        bus.history()[0].actor.conversation_id.as_deref(),
        Some("bindings-test")
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    let invalid: Command = serde_json::from_value(json!({"op":"set_input_actions","actions":{
        "jump":{"kind":"button","threshold":0,"bindings":[{"type":"key","code":"Space"}]}}}))
    .unwrap();
    let revision = bus.revision();
    assert!(
        bus.execute(
            vec![
                Command::SetMemory {
                    section: "prefix".into(),
                    text: "must not commit".into()
                },
                invalid
            ],
            Actor::user("test"),
            "Invalid bindings",
            Some(revision)
        )
        .is_err()
    );
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    assert_eq!(bus.revision(), revision);
    let mut corrupt: serde_json::Value = serde_json::from_str(&after).unwrap();
    corrupt["settings"]["input_actions"]["jump"]["threshold"] = json!(0);
    assert!(
        serde_json::from_value::<Project>(corrupt)
            .unwrap()
            .validate()
            .is_err()
    );
}
