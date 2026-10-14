//! Keep lock ownership out of the parallel parent test harness.
//!
//! A sibling process-recovery test can fork while another test owns a flock,
//! briefly retaining that descriptor after the owner drops it and before exec
//! closes it. Each parent case runs once in its own exact-filtered child; its
//! deliberate crash subprocesses remain unchanged.
pub(crate) fn isolated(case: &str) -> bool {
    if std::env::var("INCANT_CMD_EXACT_TEST").as_deref() == Ok(case) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", case, "--test-threads=1", "--nocapture"])
        .env("INCANT_CMD_EXACT_TEST", case)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{case}: {stdout}\n{stderr}");
    assert!(
        stdout.contains(&format!("test {case} ... ok"))
            && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "exact case did not execute successfully: {case}: {stdout}\n{stderr}"
    );
    true
}
