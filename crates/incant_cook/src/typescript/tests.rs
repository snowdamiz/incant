use super::*;

#[test]
fn timeout_before_first_child_phase_still_refuses_and_reaps() {
    let directory = tempfile::tempdir().unwrap();
    let node = node_executable(directory.path()).unwrap();
    let stdout = directory.path().join("stdout");
    let stderr = directory.path().join("stderr");
    let phases = directory.path().join("phases");
    File::create(&phases).unwrap();
    // Fixed trusted child deliberately never emits a phase or publishes output.
    // The shortened deadline is private to this control, not a host setting.
    let mut command = Command::new(node);
    command
        .args(["--eval", "setInterval(() => {}, 1000)"])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(File::create(&stdout).unwrap())
        .stderr(File::create(&stderr).unwrap());
    for key in ["SystemRoot", "SystemDrive", "WINDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut child = CookChild(command.spawn().unwrap());
    let started = Instant::now();
    let result = wait_for_cook(
        &mut child,
        started,
        Duration::from_millis(20),
        &stdout,
        &stderr,
        &phases,
    );
    assert!(matches!(
        result,
        Err(TypeScriptCookError::Invalid(DEADLINE_MESSAGE))
    ));
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "child was still running at refusal"
    );
    child.stop(); // Same kill/wait used by the production Drop path.
    assert!(child.0.try_wait().unwrap().is_some(), "child was reaped");
    let observation = phases::observation(&phases, started.elapsed(), "deadline");
    assert_eq!(observation["state"], "empty");
    assert_eq!(observation["phases"], serde_json::json!([]));
    assert_eq!(observation["outcome"], "deadline");
    assert!(!directory.path().join("cooked").exists());
    assert_eq!(DEADLINE, Duration::from_secs(8));
}
