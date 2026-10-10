#[path = "support/play.rs"]
mod support;
use incant_doc::{Project, new_id};
use serde_json::{Value, json};
use std::fs;
use support::{fixture, run, success};
#[test]
fn localization_check_reports_missing_translations_and_requires_explicit_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let path = root.join("game.incant.json");
    let mut project = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let id = new_id();
    project.string_tables.insert(id.clone(), serde_json::from_value(json!({"id":id,"name":"UI","source_locale":"en",
        "messages":{"hello":{"en":"Hello {name}","ja":"こんにちは {name}"},"empty":{"en":"","ja":""},"missing":{"en":"English"}}})).unwrap());
    fs::write(&path, project.canonical_text().unwrap()).unwrap();
    let before = fs::read(&path).unwrap();
    assert_eq!(
        success(run(root, &["localization-check", "game.incant.json"]))["complete"],
        true
    );
    let bad = run(
        root,
        &["localization-check", "game.incant.json", "--locale", "ja"],
    );
    assert!(!bad.status.success());
    let report: Value = serde_json::from_slice(&bad.stdout).unwrap();
    assert_eq!(
        report["missing"],
        json!([{"table_id":id,"key":"missing","requested_locale":"ja","resolved_locale":"en","kind":"translation"}])
    );
    let permitted = success(run(
        root,
        &[
            "localization-check",
            "game.incant.json",
            "--locale",
            "ja",
            "--allow-fallback",
        ],
    ));
    assert_eq!(permitted["missing"], report["missing"]);
    assert_eq!(permitted["complete"], false);
    assert_eq!(permitted["fallback_allowed"], true);
    let bad = run(
        root,
        &[
            "localization-check",
            "game.incant.json",
            "--locale",
            "en_US",
            "--allow-fallback",
        ],
    );
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "author journal must not be opened"
    );
}

#[test]
fn xliff_cli_publishes_new_exports_and_imports_through_durable_shared_history() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    // This test starts a fresh valid journal for the import workflow.
    fs::remove_file(root.join("game.incant.journal.jsonl")).unwrap();
    let path = root.join("game.incant.json");
    let mut p = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let id = new_id();
    p.string_tables.insert(id.clone(),serde_json::from_value(json!({"id":id,"name":"UI","source_locale":"en","messages":{"hello":{"en":"Hello {name}","ja":"こんにちは {name}"}}})).unwrap());
    fs::write(&path, p.canonical_text().unwrap()).unwrap();
    let exported = success(run(
        root,
        &[
            "localization-export",
            "game.incant.json",
            &id,
            "ja",
            "ja.xlf",
        ],
    ));
    assert_eq!(exported["messages"], 1);
    let original = fs::read(root.join("ja.xlf")).unwrap();
    assert!(
        !run(
            root,
            &[
                "localization-export",
                "game.incant.json",
                &id,
                "ja",
                "ja.xlf"
            ]
        )
        .status
        .success()
    );
    assert_eq!(fs::read(root.join("ja.xlf")).unwrap(), original);
    assert!(
        !run(
            root,
            &[
                "localization-export",
                "game.incant.json",
                &id,
                "ja",
                "game.incant.json"
            ]
        )
        .status
        .success()
    );
    fs::write(
        root.join("translated.xlf"),
        String::from_utf8(original)
            .unwrap()
            .replace("こんにちは", "ようこそ"),
    )
    .unwrap();
    let result = success(run(
        root,
        &["localization-import", "game.incant.json", "translated.xlf"],
    ));
    assert_eq!(result["changed_messages"], 1);
    assert_eq!(result["changed_tables"], 1);
    assert_eq!(
        Project::from_text(&fs::read_to_string(&path).unwrap())
            .unwrap()
            .string_tables[&id]
            .messages["hello"]["ja"],
        "ようこそ {name}"
    );
    let journal = root.join("game.incant.journal.jsonl");
    let before = fs::read(&journal).unwrap();
    let project_before = fs::read(&path).unwrap();
    assert!(
        success(run(
            root,
            &["localization-import", "game.incant.json", "translated.xlf"]
        ))["transaction_id"]
            .is_null()
    );
    assert_eq!(fs::read(&journal).unwrap(), before);
    let malformed = fs::read_to_string(root.join("translated.xlf"))
        .unwrap()
        .replace("Hello", "Stale");
    fs::write(root.join("bad.xlf"), malformed).unwrap();
    assert!(
        !run(
            root,
            &["localization-import", "game.incant.json", "bad.xlf"]
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&journal).unwrap(), before);
    assert_eq!(fs::read(&path).unwrap(), project_before);
    let mut bus = incant_cmd::CommandBus::persistent(
        &journal,
        Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(bus.history()[0].actor.origin, incant_doc::Origin::Import);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &p);
}
