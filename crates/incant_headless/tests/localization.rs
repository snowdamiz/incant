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
