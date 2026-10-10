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

#[test]
fn xliff_import_is_one_reversible_transaction_and_replays_after_restart() {
    use incant_cmd::PreparedTranslations;
    let mut initial = Project::empty("Exchange");
    let table = table();
    let id = table.id.clone();
    initial.string_tables.insert(id.clone(), table.clone());
    let xml = incant_localization::export_xliff(&table, "ja")
        .unwrap()
        .replace("{n}枚", "{n}個");
    let path = std::env::temp_dir().join(format!("incant-xliff-{}.jsonl", new_id()));
    let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    let actor = Actor::import("xliff");
    let result = PreparedTranslations::prepare(&bus, &xml)
        .unwrap()
        .commit(&mut bus, actor.clone())
        .unwrap();
    assert_eq!(result.changed_messages, 1);
    assert_eq!(result.changed_tables, 1);
    assert!(result.transaction_id.is_some());
    assert_eq!(
        bus.project().string_tables[&id].messages["coins"]["ja"],
        "{n}個"
    );
    assert_eq!(bus.history()[0].actor, actor);
    assert!(matches!(
        &bus.history()[0].commands[0],
        Command::UpsertStringTable { .. }
    ));
    let before_repeat = bus.revision();
    assert!(
        PreparedTranslations::prepare(&bus, &xml)
            .unwrap()
            .commit(&mut bus, actor)
            .unwrap()
            .transaction_id
            .is_none()
    );
    assert_eq!(bus.revision(), before_repeat);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &initial);
    bus.redo().unwrap();
    let expected = bus.project().clone();
    drop(bus);
    let mut reopened = CommandBus::persistent(&path, initial.clone()).unwrap();
    assert_eq!(reopened.project(), &expected);
    reopened.undo().unwrap();
    assert_eq!(reopened.project(), &initial);
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn xliff_rejects_invalid_batches_and_stale_preparations_without_losing_concurrent_edits() {
    use incant_cmd::PreparedTranslations;
    let mut initial = Project::empty("Concurrent exchange");
    let table = table();
    let id = table.id.clone();
    initial.string_tables.insert(id.clone(), table.clone());
    let xml = incant_localization::export_xliff(&table, "ja")
        .unwrap()
        .replace("{n}枚", "{n}個");
    let mut bus = CommandBus::new(initial.clone()).unwrap();
    for bad in [xml.replace("{n}個","{unknown}"),xml.replace("# coin","# changed coin"),xml.replace("</unit>","</unit><unit id=\"absent\"><segment><source>bad</source><target>partial</target></segment></unit>")] {
        assert!(PreparedTranslations::prepare(&bus,&bad).is_err());assert_eq!(bus.project(),&initial);assert!(bus.history().is_empty());
    }
    let prepared = PreparedTranslations::prepare(&bus, &xml).unwrap();
    bus.execute(
        vec![Command::SetMemory {
            section: "concurrent".into(),
            text: "keep".into(),
        }],
        Actor::user("test"),
        "Concurrent",
        None,
    )
    .unwrap();
    let concurrent = bus.project().clone();
    assert!(prepared.commit(&mut bus, Actor::import("xliff")).is_err());
    assert_eq!(bus.project(), &concurrent);
    // A different bus with the same project ID/revision must not accept an old snapshot.
    let bus = CommandBus::new(initial.clone()).unwrap();
    let prepared = PreparedTranslations::prepare(&bus, &xml).unwrap();
    let mut changed = initial;
    changed.name = "Same ID, different document".into();
    let mut other = CommandBus::new(changed.clone()).unwrap();
    assert!(prepared.commit(&mut other, Actor::import("xliff")).is_err());
    assert_eq!(other.project(), &changed);
}
