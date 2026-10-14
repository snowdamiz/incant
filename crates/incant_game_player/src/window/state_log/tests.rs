use super::*;
use std::path::Path;

fn snapshot(step: u64) -> Snapshot {
    Snapshot {
        host_steps: step,
        world_tick: step,
        generation: 1,
        cached_focused: true,
        native_focused: false,
        platform_suspended: false,
        simulation_suspended: false,
        minimized: false,
        supported_size: true,
        physical_size: [640, 480],
        retry_in_us: 0,
        surface_skipped: 0,
        surface_reconfigurations: 1,
        surface_recreations: 0,
    }
}
fn rows(path: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn completed_snapshots_are_readable_before_drop_and_preserve_actual_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    let now = Instant::now();
    let mut log = Log::open(path.to_str().unwrap(), Some(781), now).unwrap();
    log.observe(now, Reason::Initial, snapshot(0)).unwrap();
    let first = rows(&path);
    assert_eq!(first.len(), 2);
    assert_eq!(first[1]["state"]["cached_focused"], true);
    assert_eq!(first[1]["state"]["native_focused"], false);
    assert_eq!(first[1]["eligible_by_cached_flags"], true);
    log.reservation(false);
    let mut deferred = snapshot(3);
    deferred.surface_skipped = 1;
    deferred.retry_in_us = 50_000;
    log.observe(now + MIN_INTERVAL, Reason::Pulse, deferred)
        .unwrap();
    let deferred = rows(&path).pop().unwrap();
    assert_eq!(deferred["last_reservation"], "deferred");
    assert_eq!(deferred["surface_deferrals"], 1);
    assert_eq!(deferred["state"]["surface_skipped"], 1);
    assert_eq!(deferred["state"]["retry_in_us"], 50_000);
    let mut paused = snapshot(3);
    paused.cached_focused = false;
    paused.platform_suspended = true;
    paused.simulation_suspended = true;
    paused.minimized = true;
    paused.supported_size = false;
    paused.physical_size = [0, 0];
    log.observe(now + MIN_INTERVAL * 2, Reason::FocusLost, paused)
        .unwrap();
    let paused = rows(&path).pop().unwrap();
    assert_eq!(paused["eligible_by_cached_flags"], false);
    assert_eq!(paused["state"]["host_steps"], 3);
    for flag in ["platform_suspended", "simulation_suspended", "minimized"] {
        assert_eq!(paused["state"][flag], true);
    }
    log.finish(now + Duration::from_secs(1), 3, false).unwrap();
    assert_eq!(rows(&path).last().unwrap()["returned_report"], false);
}

#[test]
fn unchanged_frames_use_heartbeat_and_fast_transitions_are_explicitly_coalesced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    let now = Instant::now();
    let mut log = Log::open(path.to_str().unwrap(), Some(781), now).unwrap();
    log.observe(now, Reason::Initial, snapshot(0)).unwrap();
    for ms in 1..100 {
        log.observe(now + Duration::from_millis(ms), Reason::Pulse, snapshot(ms))
            .unwrap();
    }
    assert_eq!(rows(&path).len(), 2);
    let mut blurred = snapshot(100);
    blurred.cached_focused = false;
    log.observe(now + Duration::from_millis(100), Reason::FocusLost, blurred)
        .unwrap();
    log.observe(
        now + Duration::from_millis(150),
        Reason::FocusGained,
        snapshot(150),
    )
    .unwrap();
    assert_eq!(rows(&path).len(), 2);
    log.observe(now + MIN_INTERVAL, Reason::Pulse, snapshot(250))
        .unwrap();
    let transitions = rows(&path).pop().unwrap();
    assert_eq!(transitions["focus_lost_events"], 1);
    assert_eq!(transitions["focus_gained_events"], 1);
    assert_eq!(transitions["coalesced_samples"], 2);
    for ms in 251..1250 {
        log.observe(now + Duration::from_millis(ms), Reason::Pulse, snapshot(ms))
            .unwrap();
    }
    assert_eq!(rows(&path).len(), 3);
    log.observe(
        now + Duration::from_millis(1250),
        Reason::Pulse,
        snapshot(1250),
    )
    .unwrap();
    assert_eq!(rows(&path).len(), 4);
}

#[test]
fn event_storm_truncates_once_with_finite_record_and_byte_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    let now = Instant::now();
    let mut log = Log::open(path.to_str().unwrap(), Some(36_000), now).unwrap();
    for n in 0..10_000 {
        log.reservation(n % 2 == 0);
        log.observe(
            now + MIN_INTERVAL * n,
            Reason::Resize,
            snapshot(u64::from(n)),
        )
        .unwrap();
    }
    log.finish(now + MIN_INTERVAL * 10_000, 10_000, false)
        .unwrap();
    let recorded = rows(&path);
    assert_eq!(recorded.len(), MAX_RECORDS);
    assert_eq!(recorded.last().unwrap()["kind"], "truncated");
    assert_eq!(
        recorded.iter().filter(|r| r["kind"] == "truncated").count(),
        1
    );
    assert!(std::fs::metadata(&path).unwrap().len() <= MAX_BYTES as u64);
}

#[test]
fn bounds_paths_and_existing_destination_are_refused_before_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    for bound in [None, Some(0), Some(36_001)] {
        assert!(matches!(
            Log::open(path.to_str().unwrap(), bound, Instant::now()),
            Err(Error::Bound)
        ));
        assert!(!path.exists());
    }
    for path in ["", "contains\ncontrol", &"x".repeat(4097)] {
        assert!(matches!(validate(path, Some(1), false), Err(Error::Path)));
    }
    assert!(matches!(
        validate("trusted", Some(1), true),
        Err(Error::Bound)
    ));
    std::fs::write(&path, b"existing host evidence").unwrap();
    assert!(matches!(
        Log::open(path.to_str().unwrap(), Some(1), Instant::now()),
        Err(Error::Io(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"existing host evidence");
    #[cfg(unix)]
    {
        let link = dir.path().join("link.jsonl");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(Log::open(link.to_str().unwrap(), Some(1), Instant::now()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"existing host evidence");
    }
}

#[test]
fn backwards_clock_refuses_and_original_window_error_keeps_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    let now = Instant::now();
    let mut log = Log::open(path.to_str().unwrap(), Some(1), now).unwrap();
    log.observe(now + MIN_INTERVAL, Reason::Initial, snapshot(0))
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        log.observe(now, Reason::Pulse, snapshot(1)),
        Err(Error::Clock)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let original: session::Result<()> = Err("original window failure".into());
    assert_eq!(
        preserve_result(original, Err(Error::Clock))
            .unwrap_err()
            .to_string(),
        "original window failure"
    );
    assert!(
        preserve_result(Ok(()), Err(Error::Clock))
            .unwrap_err()
            .downcast_ref::<Error>()
            .is_some()
    );
}

#[test]
fn completed_snapshots_survive_process_exit_without_rust_drop() {
    const DESTINATION: &str = "INCANT_STATE_LOG_EXIT_FIXTURE";
    if let Some(path) = std::env::var_os(DESTINATION) {
        let now = Instant::now();
        let mut log = Log::open(Path::new(&path).to_str().unwrap(), Some(1), now).unwrap();
        log.observe(now, Reason::Initial, snapshot(0)).unwrap();
        // No Rust destructors or final row: model the observer's completed
        // writes remaining usable when the owning process does not unwind.
        std::process::exit(7);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("child.jsonl");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "window::state_log::tests::completed_snapshots_survive_process_exit_without_rust_drop",
            "--nocapture",
        ])
        .env(DESTINATION, &path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(7));
    let recorded = rows(&path);
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[1]["state"]["host_steps"], 0);
    assert!(!recorded.iter().any(|r| r["kind"] == "final"));
}
