#[path = "../../incant_cmd/tests/support/prefab.rs"]
mod support;
use incant_cmd::Command;
use incant_doc::{Project, new_id};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command as Process, Stdio},
};
fn rpc(root: &std::path::Path, calls: Vec<Value>) -> Vec<Value> {
    let mut child = Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .args(["rpc", "project.json", "--journal", "history.jsonl"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for (id, call) in calls.iter().enumerate() {
        let mut call = call.clone();
        call["id"] = json!(id);
        writeln!(input, "{call}").unwrap();
    }
    drop(input);
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let lines = String::from_utf8(result.stdout).unwrap();
    let rows: Vec<Value> = lines
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows.len(), calls.len());
    rows
}
fn execute(revision: u64, commands: Vec<Command>) -> Value {
    json!({"method":"command.execute","params":{"expected_revision":revision,"commands":commands,"description":"Prefab RPC"}})
}
fn ok(value: &Value) -> &Value {
    assert!(value.get("error").is_none(), "{value}");
    &value["result"]
}
#[test]
fn live_headless_prefab_commands_schema_save_and_journal_reopen() {
    let f = support::Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("project.json"),
        f.bus.project().canonical_text().unwrap(),
    )
    .unwrap();
    let a: BTreeMap<_, _> = f.nodes.values().map(|id| (id.clone(), new_id())).collect();
    let b: BTreeMap<_, _> = f.nodes.values().map(|id| (id.clone(), new_id())).collect();
    let calls = vec![
        json!({"method":"schema.list"}),
        execute(0, vec![f.create_command()]),
        execute(
            1,
            vec![
                Command::InstantiatePrefab {
                    source: f.prefab.clone(),
                    scene_id: f.scene.clone(),
                    parent: None,
                    entities: a.clone(),
                },
                Command::InstantiatePrefab {
                    source: f.prefab.clone(),
                    scene_id: f.scene.clone(),
                    parent: None,
                    entities: b.clone(),
                },
            ],
        ),
        json!({"method":"project.save"}),
    ];
    let result = rpc(dir.path(), calls);
    assert!(ok(&result[0]).get("Prefab").is_some());
    assert!(
        ok(&result[0])["Project"]["properties"]
            .get("prefabs")
            .is_some()
    );
    assert_eq!(ok(&result[1])["revision"], 1);
    assert_eq!(ok(&result[2])["revision"], 2);
    assert_eq!(ok(&result[3])["saved"], true);
    let saved =
        Project::from_text(&std::fs::read_to_string(dir.path().join("project.json")).unwrap())
            .unwrap();
    assert_eq!(saved.scenes[&f.scene].entities.len(), 6);
    let result = rpc(
        dir.path(),
        vec![
            json!({"method":"project.read"}),
            json!({"method":"history.undo"}),
            json!({"method":"history.redo"}),
        ],
    );
    assert_eq!(ok(&result[0])["project"], json!(saved));
    assert_eq!(
        ok(&result[1])["project"]["scenes"][&f.scene]["entities"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(ok(&result[2])["project"], json!(saved));
}
