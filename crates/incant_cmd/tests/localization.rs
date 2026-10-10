use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{LocaleSettings, Project, StringTable, new_id};
use serde_json::json;
fn table() -> StringTable {
    serde_json::from_value(json!({"id":new_id(),"name":"UI","source_locale":"en",
        "messages":{"coins":{"en":"{n, plural, one {# coin} other {# coins}}","ja":"{n}枚"}}}))
    .unwrap()
}
#[test]
fn tables_and_locale_are_atomic_reversible_journaled_and_attributed() {
    let initial = Project::empty("Localized");
    let path = std::env::temp_dir().join(format!("incant-locale-{}.jsonl", new_id()));
    let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    let table = table();
    let actor = Actor::agent("localization-test", "session-test");
    let tx = bus
        .execute(
            vec![
                Command::UpsertStringTable {
                    table: table.clone(),
                },
                Command::SetLocale {
                    settings: LocaleSettings {
                        locale: "ja".into(),
                        ..Default::default()
                    },
                },
            ],
            actor.clone(),
            "Add translated UI",
            Some(0),
        )
        .unwrap();
    assert_eq!(tx.actor, actor);
    assert_eq!(tx.commands.len(), 2);
    let text = bus.project().canonical_text().unwrap();
    assert_eq!(Project::from_text(&text).unwrap(), *bus.project());
    bus.undo().unwrap();
    assert_eq!(bus.project(), &initial);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), text);
    drop(bus);
    let mut restored = CommandBus::persistent(&path, initial.clone()).unwrap();
    assert_eq!(restored.project().canonical_text().unwrap(), text);
    assert_eq!(restored.history()[0].actor, actor);
    restored
        .execute(
            vec![Command::RemoveStringTable { table_id: table.id }],
            Actor::user("test"),
            "Remove",
            None,
        )
        .unwrap();
    assert!(restored.project().string_tables.is_empty());
    restored.undo().unwrap();
    assert_eq!(restored.project().canonical_text().unwrap(), text);
    drop(restored);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn invalid_translation_or_locale_cannot_commit_a_valid_prefix() {
    let mut bus = CommandBus::new(Project::empty("Reject")).unwrap();
    let before = bus.project().canonical_text().unwrap();
    let valid = table();
    let mut invalid = valid.clone();
    invalid
        .messages
        .get_mut("coins")
        .unwrap()
        .insert("fr".into(), "{n, plural, one {coin}}".into());
    let mut duplicate = valid.clone();
    duplicate.id = bus.project().id.clone();
    for bad in [
        Command::UpsertStringTable { table: invalid },
        Command::UpsertStringTable { table: duplicate },
        Command::SetLocale {
            settings: LocaleSettings {
                locale: "en_US".into(),
                ..Default::default()
            },
        },
        Command::RemoveStringTable { table_id: new_id() },
    ] {
        assert!(
            bus.execute(
                vec![
                    Command::UpsertStringTable {
                        table: valid.clone()
                    },
                    bad
                ],
                Actor::user("test"),
                "Must roll back",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), before);
        assert!(bus.history().is_empty());
        assert_eq!(bus.revision(), 0);
    }
    let mut unexpected = json!(valid);
    unexpected["unknown"] = json!(true);
    assert!(serde_json::from_value::<StringTable>(unexpected).is_err());
    assert!(!before.contains("string_tables"));
    assert!(!before.contains("localization"));
}
