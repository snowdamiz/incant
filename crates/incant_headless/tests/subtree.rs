#[path = "../../incant_cmd/tests/support/subtree.rs"]
mod support;
use incant_cmd::Command;
use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command as Process, Stdio},
};

fn rpc(root: &std::path::Path, calls: Vec<Value>) -> Vec<Value> {
    let mut child = Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .args(["rpc", "project.json", "--journal", "custom.jsonl"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for (id, mut call) in calls.into_iter().enumerate() {
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
    String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn execute(revision: u64, commands: Value) -> Value {
    json!({"method":"command.execute","params":{"expected_revision":revision,"commands":commands,"description":"Duplicate subtree"}})
}
fn ok(value: &Value) -> &Value {
    assert!(value.get("error").is_none(), "{value}");
    &value["result"]
}

#[test]
fn rpc_schema_copy_refusal_save_and_reopen_share_one_command_contract() {
    let f = support::Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.json");
    std::fs::write(&path, f.bus.project().canonical_text().unwrap()).unwrap();
    let command = json!(f.command());
    let mut unknown = command.clone();
    unknown["hidden_detach"] = json!(true);
    let rows = rpc(
        dir.path(),
        vec![
            json!({"method":"schema.list"}),
            execute(0, json!([unknown])),
            execute(0, json!([command])),
            execute(0, json!([command])),
            execute(
                1,
                json!([Command::RenameEntity {
                    scene_id: f.scene.clone(),
                    entity_id: f.mapping[&f.root].clone(),
                    name: "RPC copy".into()
                }]),
            ),
            json!({"method":"project.save","params":{"expected_revision":2}}),
        ],
    );
    assert_eq!(rows.len(), 6);
    let variants = ok(&rows[0])["Command"]["oneOf"].as_array().unwrap();
    let variant = variants
        .iter()
        .find(|v| v["properties"]["op"]["const"] == "duplicate_subtree")
        .unwrap();
    assert_eq!(variant["additionalProperties"], false);
    assert_eq!(variant["properties"]["entities"]["minProperties"], 1);
    assert_eq!(variant["properties"]["entities"]["maxProperties"], 1024);
    assert!(rows[1].get("error").is_some());
    assert_eq!(ok(&rows[2])["revision"], 1);
    assert!(
        rows[3]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("revision conflict")
    );
    assert_eq!(ok(&rows[4])["revision"], 2);
    assert_eq!(ok(&rows[5])["saved"], true);
    let saved = Project::from_text(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved.scenes[&f.scene].entities.len(), 8);
    assert_eq!(
        saved.scenes[&f.scene].entities[&f.mapping[&f.root]].name,
        "RPC copy"
    );
    let rows = rpc(
        dir.path(),
        vec![
            json!({"method":"project.read"}),
            json!({"method":"history.undo"}),
            json!({"method":"history.undo"}),
            json!({"method":"history.redo"}),
            json!({"method":"history.redo"}),
        ],
    );
    assert_eq!(rows.len(), 5);
    assert_eq!(ok(&rows[0])["project"], json!(saved));
    assert_eq!(ok(&rows[2])["project"], json!(f.bus.project()));
    assert_eq!(ok(&rows[4])["project"], json!(saved));
}
