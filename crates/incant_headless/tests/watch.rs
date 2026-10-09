use incant_cmd::CommandBus;
use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

fn command(root: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_incant_headless"));
    cmd.current_dir(root);
    cmd
}
fn run(root: &Path, args: &[&str]) -> Value {
    let out = command(root).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn buffer(root: &Path, size: f32) {
    fs::write(
        root.join("model.bin"),
        [0_f32, 0., 0., size, 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
fn setup(root: &Path) -> Value {
    run(root, &["init", "game.json", "--entities", "0"]);
    buffer(root, 1.);
    fs::write(root.join("model.gltf"), json!({"asset":{"version":"2.0"},"buffers":[{"uri":"model.bin","byteLength":36}],
        "bufferViews":[{"buffer":0,"byteLength":36}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[10,1,0]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
        "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
    run(root, &["import", "game.json", "model.gltf"])
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn next(events: &mpsc::Receiver<Value>, matches: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..20 {
        let event = events
            .recv_timeout(Duration::from_secs(10))
            .expect("watch process did not report progress");
        if matches(&event) {
            return event;
        }
    }
    panic!("watch process emitted too many unexpected events");
}

#[test]
fn running_cli_reimports_recovers_reloads_and_preserves_external_edits_and_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let first = setup(root);
    let id = first["asset"]["id"].as_str().unwrap();
    let mut child = Process(
        command(root)
            .args([
                "watch-assets",
                "game.json",
                "--interval-ms",
                "10",
                "--debounce-ms",
                "0",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let event = serde_json::from_str(&line.unwrap()).unwrap();
            if sender.send(event).is_err() {
                break;
            }
        }
    });
    next(&receiver, |v| v["event"] == "watching");
    next(&receiver, |v| v["assets"][id]["generation"] == 1);
    // Exclusive journal ownership prevents a second writer.
    assert!(
        !command(root)
            .args(["import", "game.json", "model.gltf"])
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("model.bin"), b"incomplete write").unwrap();
    let failed = next(&receiver, |v| {
        v["report"]["diagnostics"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    });
    assert_eq!(
        failed["assets"][id]["fingerprint"],
        first["asset"]["sha256"]
    );
    buffer(root, 2.);
    let updated = next(&receiver, |v| {
        v["report"]["imports"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    });
    assert_eq!(updated["report"]["cleared"][0], id);
    assert_eq!(updated["assets"][id]["generation"], 2);
    assert_ne!(
        updated["assets"][id]["fingerprint"],
        first["asset"]["sha256"]
    );
    let saved = fs::read_to_string(root.join("game.json")).unwrap();
    let project = Project::from_text(&saved).unwrap();
    assert_eq!(
        project.assets[id].sha256,
        updated["assets"][id]["fingerprint"].as_str().unwrap()
    );
    // The command must stop, not replace a user's edit to the project file.
    let external = format!("{saved}\n");
    fs::write(root.join("game.json"), &external).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(10)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(!child.0.wait().unwrap().success());
    let mut error = String::new();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut error)
        .unwrap();
    assert!(error.contains("without overwriting"));
    assert_eq!(
        fs::read_to_string(root.join("game.json")).unwrap(),
        external
    );
    reader.join().unwrap();
    let mut recovered = CommandBus::persistent(root.join("game.journal.jsonl"), project).unwrap();
    assert_eq!(recovered.history().len(), 2);
    recovered.undo().unwrap();
    assert_eq!(
        recovered.project().assets[id].sha256,
        first["asset"]["sha256"].as_str().unwrap()
    );
}

#[test]
fn bounded_watch_reports_unsettled_sources_and_exits_unsuccessfully() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    let args = [
        "watch-assets",
        "game.json",
        "--polls",
        "1",
        "--debounce-ms",
        "0",
    ];
    assert!(command(root).args(args).output().unwrap().status.success());
    fs::remove_file(root.join("model.bin")).unwrap();
    let before = fs::read(root.join("game.json")).unwrap();
    let out = command(root).args(args).output().unwrap();
    assert!(!out.status.success());
    let last: Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).lines().last().unwrap()).unwrap();
    assert_eq!(last["event"], "stopped");
    assert_eq!(last["pending"], 1);
    assert_eq!(last["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(fs::read(root.join("game.json")).unwrap(), before);
}
