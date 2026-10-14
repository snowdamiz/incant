//! Real RPC child owns one host-bound bus; JSON requests never supply paths.
use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
};
struct Rpc {
    child: Child,
    out: BufReader<ChildStdout>,
}
impl Rpc {
    fn open(path: &Path, journal: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
            .arg("rpc")
            .arg(path)
            .arg("--journal")
            .arg(journal)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Self { child, out }
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        writeln!(
            self.child.stdin.as_mut().unwrap(),
            "{}",
            json!({"id":1,"method":method,"params":params})
        )
        .unwrap();
        let mut line = String::new();
        assert_ne!(self.out.read_line(&mut line).unwrap(), 0);
        serde_json::from_str(&line).unwrap()
    }
    fn file(&mut self, params: Value) -> Value {
        self.request("project.file", params)
    }
}
impl Drop for Rpc {
    fn drop(&mut self) {
        self.child.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[test]
fn live_review_apply_save_history_reopen_and_closed_authority() {
    let root = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant.json");
    let journal = logs.path().join("custom.jsonl");
    let mut p = Project::empty("RPC");
    p.engine_version = None;
    let source = p.canonical_text().unwrap();
    fs::write(&path, &source).unwrap();
    let mut rpc = Rpc::open(&path, &journal);
    let schemas = rpc.request("schema.list", json!({}));
    for name in [
        "ProjectPinReview",
        "ProjectFileRequest",
        "ProjectFileResponse",
        "ProjectSaveReceipt",
    ] {
        assert!(schemas["result"].get(name).is_some());
    }
    let review = rpc.file(json!({"operation":"review_pin","expected_revision":0}));
    let id = review["result"]["review"]["review_id"]
        .as_str()
        .unwrap()
        .to_owned();
    for extra in ["path", "candidate_text", "backup"] {
        let mut request = json!({"operation":"apply_pin","expected_revision":0,"review_id":id});
        request[extra] = json!("not authority");
        assert!(rpc.file(request).get("error").is_some());
    }
    assert!(
        rpc.file(json!({"operation":"apply_pin","expected_revision":1,"review_id":id}))
            .get("error")
            .is_some()
    );
    let applied = rpc.file(json!({"operation":"apply_pin","expected_revision":0,"review_id":id}));
    assert_eq!(applied["result"]["revision"], 1);
    assert!(
        Project::from_text(&fs::read_to_string(&path).unwrap())
            .unwrap()
            .engine_version
            .is_some()
    );
    assert!(
        rpc.file(json!({"operation":"apply_pin","expected_revision":1,"review_id":id}))
            .get("error")
            .is_some()
    );
    let edit=rpc.request("command.execute",json!({"expected_revision":1,"description":"note","commands":[{"op":"set_memory","section":"note","text":"edited"}]}));
    assert_eq!(edit["result"]["revision"], 2);
    assert!(
        rpc.request("project.save", json!({"expected_revision":1}))
            .get("error")
            .is_some()
    );
    let before_save = fs::read(&path).unwrap();
    for params in [
        json!({"path":"evil"}),
        json!({"expected_revision":null}),
        json!({"expected_revision":-1}),
        json!({"expected_revision":"2"}),
    ] {
        assert!(rpc.request("project.save", params).get("error").is_some());
        assert_eq!(fs::read(&path).unwrap(), before_save);
    }
    assert!(rpc.file(json!({"operation":"save"})).get("error").is_some());
    assert_eq!(fs::read(&path).unwrap(), before_save);
    // The legacy empty request uses this same host-owned bus and its current revision.
    assert_eq!(
        rpc.request("project.save", json!({}))["result"]["saved"],
        true
    );
    assert_eq!(
        rpc.request("project.save", json!({"expected_revision":2}))["result"]["saved"],
        true
    );
    assert_eq!(
        rpc.request("history.undo", json!({}))["result"]["revision"],
        3
    );
    assert_eq!(
        rpc.request("history.undo", json!({}))["result"]["revision"],
        4
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    drop(rpc);
    let mut rpc = Rpc::open(&path, &journal);
    assert_eq!(
        rpc.request("project.read", json!({}))["result"]["revision"],
        4
    );
    assert_eq!(
        rpc.request("history.redo", json!({}))["result"]["revision"],
        5
    );
    assert_eq!(
        rpc.request("history.redo", json!({}))["result"]["revision"],
        6
    );
    assert_eq!(
        rpc.file(json!({"operation":"save","expected_revision":6}))["result"]["receipt"]["revision"],
        6
    );
    drop(rpc);
    let bytes = fs::read(&path).unwrap();
    let log = fs::read(&journal).unwrap();
    let mut rpc = Rpc::open(&path, &journal);
    assert!(
        rpc.file(json!({"operation":"apply_pin","expected_revision":6,"review_id":id}))
            .get("error")
            .is_some()
    );
    drop(rpc);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read(&journal).unwrap(), log);
}
