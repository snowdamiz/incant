use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

struct RpcChild(Child);
impl std::ops::Deref for RpcChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}
impl std::ops::DerefMut for RpcChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}
impl Drop for RpcChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn request(child: &mut Child, output: &mut impl BufRead, method: &str, params: Value) -> Value {
    writeln!(
        child.stdin.as_mut().unwrap(),
        "{}",
        json!({"id":1,"method":method,"params":params})
    )
    .unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn cli_and_rpc_preview_are_read_only_and_commands_use_normal_shared_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.incant.json");
    let journal = dir.path().join("history.jsonl");
    let mut p = Project::empty("Legacy");
    p.engine_version = None;
    let original = p.canonical_text().unwrap();
    fs::write(&path, &original).unwrap();
    let cli = env!("CARGO_BIN_EXE_incant_headless");
    let status = Command::new(cli)
        .arg("engine-compatibility")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&status.stdout).unwrap()["declaration"],
        "legacy_unpinned"
    );
    let preview = Command::new(cli)
        .arg("review-engine-pin")
        .arg(&path)
        .output()
        .unwrap();
    assert!(preview.status.success());
    let plan: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(plan["file_migration_supported"], false);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

    let mut child = RpcChild(
        Command::new(cli)
            .arg("rpc")
            .arg(&path)
            .arg("--journal")
            .arg(&journal)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let preview = request(
        &mut child,
        &mut output,
        "project.review_engine_pin",
        json!({"expected_revision":0}),
    );
    assert_eq!(preview["result"], plan);
    let schemas = request(&mut child, &mut output, "schema.list", json!({}));
    assert!(schemas["result"]["EnginePinReview"].is_object());
    assert!(schemas["result"]["EngineCompatibility"].is_object());
    let before = fs::read(&journal).unwrap();
    for params in [
        json!({}),
        json!({"expected_revision":1}),
        json!({"expected_revision":0,"path":"unrelated.json"}),
    ] {
        assert!(
            request(&mut child, &mut output, "project.review_engine_pin", params)
                .get("error")
                .is_some()
        );
        assert_eq!(fs::read(&journal).unwrap(), before);
    }
    let mut external = p.clone();
    external.name = "External".into();
    fs::write(&path, external.canonical_text().unwrap()).unwrap();
    assert!(
        request(
            &mut child,
            &mut output,
            "project.review_engine_pin",
            json!({"expected_revision":0})
        )
        .get("error")
        .is_some()
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        external.canonical_text().unwrap()
    );
    assert_eq!(fs::read(&journal).unwrap(), before);
    fs::write(&path, &original).unwrap();
    let result = request(
        &mut child,
        &mut output,
        "command.execute",
        json!({"commands":plan["commands"],"expected_revision":0,"description":"Reviewed metadata pin"}),
    );
    assert_eq!(result["result"]["revision"], 1);
    assert_eq!(result["result"]["project"]["engine_version"], "=0.0.1");
    // Ordinary command execution writes the journal, not the project file.
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(
        request(&mut child, &mut output, "history.undo", json!({}))["result"]["revision"],
        2
    );
    assert_eq!(
        request(
            &mut child,
            &mut output,
            "project.engine_compatibility",
            json!({})
        )["result"]["declaration"],
        "legacy_unpinned"
    );
    child.stdin.take();
    assert!(child.wait().unwrap().success());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    let fresh_path = dir.path().join("fresh.incant.json");
    let init = Command::new(cli)
        .arg("init")
        .arg(&fresh_path)
        .arg("--entities")
        .arg("0")
        .output()
        .unwrap();
    assert!(init.status.success());
    assert_eq!(
        Project::from_text(&fs::read_to_string(&fresh_path).unwrap())
            .unwrap()
            .engine_version
            .unwrap()
            .as_str(),
        "=0.0.1"
    );
    assert!(
        !Command::new(cli)
            .arg("review-engine-pin")
            .arg(&fresh_path)
            .output()
            .unwrap()
            .status
            .success()
    );
    for (field, invalid) in [
        ("engine_version", json!(">=99.0.0")),
        ("schema_version", json!(2)),
    ] {
        let mut value = json!(p);
        value[field] = invalid;
        fs::write(&path, value.to_string()).unwrap();
        let refused = Command::new(cli)
            .arg("review-engine-pin")
            .arg(&path)
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), value.to_string());
    }
}
