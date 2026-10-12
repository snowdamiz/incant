use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{
    AgentBudgetSettings, CollaborativeDocument, DEFAULT_AGENT_TOKEN_CAP, MAX_AGENT_TOKEN_CAP,
    Origin, Project,
};
use serde_json::json;

fn budget(max_tokens: u64) -> Command {
    Command::SetAgentBudget {
        settings: AgentBudgetSettings { max_tokens },
    }
}

fn memory() -> Command {
    Command::SetMemory {
        section: "attempt".into(),
        text: "must commit atomically".into(),
    }
}

fn nonusers() -> [Actor; 3] {
    [
        Actor::agent("test-model", "budget-test"),
        Actor::import("importer"),
        Actor {
            origin: Origin::Script,
            ..Actor::user("script")
        },
    ]
}

#[test]
fn user_caps_are_durable_with_provenance_and_historical_undo_redo() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.jsonl");
    let initial = Project::empty("Allowance");
    let original = initial.canonical_text().unwrap();
    let transaction_id;
    let checkpoint;
    {
        let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
        transaction_id = bus
            .execute(
                vec![budget(0), memory()],
                Actor::user("author"),
                "Disable turns",
                Some(0),
            )
            .unwrap()
            .id
            .clone();
        assert_eq!(bus.project().settings.agent_budget.max_tokens, 0);
        assert_eq!(bus.project().memory["attempt"], "must commit atomically");
        assert_eq!(bus.history()[0].actor.origin, Origin::User);
        assert_eq!(bus.history()[0].actor.actor, "author");
        checkpoint = bus.project().clone();
    }
    {
        let mut bus = CommandBus::persistent(&path, checkpoint).unwrap();
        assert_eq!(bus.history()[0].id, transaction_id);
        assert_eq!(bus.undo().unwrap(), transaction_id);
        assert_eq!(bus.project().canonical_text().unwrap(), original);
        assert!(bus.can_redo());
    }
    {
        let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
        assert_eq!(bus.redo().unwrap(), transaction_id);
        assert_eq!(bus.project().settings.agent_budget.max_tokens, 0);
        bus.execute(
            vec![budget(MAX_AGENT_TOKEN_CAP)],
            Actor::user("author"),
            "Raise cap",
            Some(3),
        )
        .unwrap();
        assert_eq!(
            bus.project().settings.agent_budget.max_tokens,
            MAX_AGENT_TOKEN_CAP
        );
        bus.undo().unwrap();
        assert_eq!(bus.project().settings.agent_budget.max_tokens, 0);
        bus.redo().unwrap();
    }
    let bus = CommandBus::persistent(&path, initial).unwrap();
    assert_eq!(
        bus.project().settings.agent_budget.max_tokens,
        MAX_AGENT_TOKEN_CAP
    );
    assert_eq!(bus.revision(), 6);
}

#[test]
fn every_nonuser_budget_command_is_refused_without_changing_state_or_journal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.jsonl");
    let mut bus = CommandBus::persistent(&path, Project::empty("Protected")).unwrap();
    bus.execute(vec![memory()], Actor::user("author"), "Prior edit", None)
        .unwrap();
    bus.undo().unwrap();
    let before = bus.project().clone();
    let revision = bus.revision();
    let history = bus.history().to_vec();
    let redo: Vec<_> = bus.redo_history().cloned().collect();
    let assert_unchanged = |bus: &CommandBus| {
        assert_eq!(bus.project(), &before);
        assert_eq!(bus.revision(), revision);
        assert_eq!(bus.history(), history);
        assert_eq!(bus.redo_history().cloned().collect::<Vec<_>>(), redo);
    };
    // Windows also enforces the owning bus's exclusive journal lock on reads.
    drop(bus);
    let journal = std::fs::read(&path).unwrap();
    let mut bus = CommandBus::persistent(&path, before.clone()).unwrap();
    assert_unchanged(&bus);
    for actor in nonusers() {
        // A no-op and a transient increase followed by restoring the old cap
        // still require user origin; a mixed transaction cannot launder either.
        for commands in [
            vec![budget(0)],
            vec![memory(), budget(DEFAULT_AGENT_TOKEN_CAP)],
            vec![
                budget(MAX_AGENT_TOKEN_CAP),
                memory(),
                budget(DEFAULT_AGENT_TOKEN_CAP),
            ],
        ] {
            let crdt = bus.export_crdt().unwrap();
            let error = bus
                .execute(
                    commands,
                    actor.clone(),
                    "Disallowed allowance",
                    Some(revision),
                )
                .unwrap_err();
            assert!(matches!(error, CommandError::AgentBudgetUserOnly));
            assert_unchanged(&bus);
            assert_eq!(bus.export_crdt().unwrap(), crdt);
            drop(bus);
            assert_eq!(std::fs::read(&path).unwrap(), journal);
            bus = CommandBus::persistent(&path, before.clone()).unwrap();
            assert_unchanged(&bus);
        }
    }
    bus.redo().unwrap();
    assert_eq!(bus.project().memory["attempt"], "must commit atomically");
}

#[test]
fn invalid_user_cap_refuses_the_entire_transaction_and_redo_remains_available() {
    let mut bus = CommandBus::new(Project::empty("Validated")).unwrap();
    bus.execute(vec![budget(0)], Actor::user("author"), "Disable", None)
        .unwrap();
    bus.undo().unwrap();
    let before = bus.project().clone();
    let revision = bus.revision();
    assert!(matches!(
        bus.execute(
            vec![memory(), budget(MAX_AGENT_TOKEN_CAP + 1)],
            Actor::user("author"),
            "Invalid",
            None
        ),
        Err(CommandError::Document(_))
    ));
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), revision);
    assert!(bus.history().is_empty());
    bus.redo().unwrap();
    assert_eq!(bus.project().settings.agent_budget.max_tokens, 0);
}

#[test]
fn whole_projection_merge_cannot_bypass_origin_and_user_merge_is_reversible() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.jsonl");
    let initial = Project::empty("Merged allowance");
    let bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    let mut peer = CollaborativeDocument::from_export(&bus.export_crdt().unwrap()).unwrap();
    let mut remote = peer.project().unwrap();
    remote.settings.agent_budget.max_tokens = 2_000_000;
    remote
        .memory
        .insert("remote".into(), "alongside cap".into());
    peer.replace(&remote).unwrap();
    let bytes = peer.export().unwrap();
    let assert_unchanged = |bus: &CommandBus| {
        assert_eq!(bus.project(), &initial);
        assert_eq!(bus.revision(), 0);
        assert!(bus.history().is_empty());
        assert!(!bus.can_redo());
    };
    drop(bus);
    let journal = std::fs::read(&path).unwrap();
    let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    assert_unchanged(&bus);
    for actor in nonusers() {
        let crdt = bus.export_crdt().unwrap();
        assert!(matches!(
            bus.merge(&bytes, actor),
            Err(CommandError::AgentBudgetUserOnly)
        ));
        assert_unchanged(&bus);
        assert_eq!(bus.export_crdt().unwrap(), crdt);
        drop(bus);
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        bus = CommandBus::persistent(&path, initial.clone()).unwrap();
        assert_unchanged(&bus);
    }
    let id = bus.merge(&bytes, Actor::user("author")).unwrap().id.clone();
    assert_eq!(bus.project(), &remote);
    assert_eq!(bus.undo().unwrap(), id);
    assert_eq!(bus.project(), &initial);
    assert_eq!(bus.redo().unwrap(), id);
    assert_eq!(bus.project(), &remote);
    drop(bus);
    let reopened = CommandBus::persistent(&path, initial).unwrap();
    assert_eq!(reopened.project(), &remote);
    assert_eq!(reopened.history()[0].actor.origin, Origin::User);
}

#[test]
fn removing_a_remote_explicit_cap_cannot_restore_the_default_as_a_nonuser() {
    let mut initial = Project::empty("Disabled");
    initial.settings.agent_budget.max_tokens = 0;
    let mut bus = CommandBus::new(initial.clone()).unwrap();
    let mut peer = CollaborativeDocument::from_export(&bus.export_crdt().unwrap()).unwrap();
    let mut remote = peer.project().unwrap();
    remote.settings.agent_budget = AgentBudgetSettings::default();
    assert!(json!(remote)["settings"].get("agent_budget").is_none());
    peer.replace(&remote).unwrap();
    for actor in nonusers() {
        assert!(matches!(
            bus.merge(&peer.export().unwrap(), actor),
            Err(CommandError::AgentBudgetUserOnly)
        ));
        assert_eq!(bus.project(), &initial);
        assert_eq!(bus.revision(), 0);
    }
    bus.merge(&peer.export().unwrap(), Actor::user("author"))
        .unwrap();
    assert_eq!(
        bus.project().settings.agent_budget.max_tokens,
        DEFAULT_AGENT_TOKEN_CAP
    );
}

#[test]
fn unrelated_nonuser_edits_and_imports_preserve_the_existing_allowance() {
    let mut bus = CommandBus::new(Project::empty("Ordinary agent edits")).unwrap();
    bus.execute(vec![budget(100)], Actor::user("author"), "Limit", None)
        .unwrap();
    bus.execute(
        vec![memory()],
        Actor::agent("test-model", "budget-test"),
        "Agent edit",
        None,
    )
    .unwrap();
    let mut peer = CollaborativeDocument::from_export(&bus.export_crdt().unwrap()).unwrap();
    let mut remote = peer.project().unwrap();
    remote.name = "Imported name".into();
    peer.replace(&remote).unwrap();
    bus.merge(&peer.export().unwrap(), Actor::import("importer"))
        .unwrap();
    assert_eq!(bus.project(), &remote);
    assert_eq!(bus.project().settings.agent_budget.max_tokens, 100);
}

#[test]
fn command_wire_shape_and_schema_publish_the_bounded_typed_setting() {
    let value = json!({"op": "set_agent_budget", "settings": {"max_tokens": 0}});
    assert_eq!(
        serde_json::from_value::<Command>(value.clone()).unwrap(),
        budget(0)
    );
    assert_eq!(serde_json::to_value(budget(0)).unwrap(), value);
    for settings in [
        json!({}),
        json!({"max_tokens": -1}),
        json!({"max_tokens": 1.5}),
        json!({"max_tokens": 0, "consumed": 0}),
    ] {
        assert!(
            serde_json::from_value::<Command>(
                json!({"op":"set_agent_budget", "settings": settings})
            )
            .is_err()
        );
    }
    let schema = json!(schemars::schema_for!(Command));
    let variant = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|variant| variant["properties"]["op"]["const"] == "set_agent_budget")
        .unwrap();
    assert_eq!(
        variant["properties"]["settings"]["$ref"],
        "#/$defs/AgentBudgetSettings"
    );
    assert_eq!(variant["additionalProperties"], false);
    assert_eq!(variant["required"], json!(["op", "settings"]));
    assert_eq!(
        schema["$defs"]["AgentBudgetSettings"]["properties"]["max_tokens"]["maximum"],
        MAX_AGENT_TOKEN_CAP
    );
}
