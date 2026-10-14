use super::*;
use model::{Model, Status};

#[test]
fn attempts_are_not_commits_and_roles_survive_repeated_world_ticks() {
    let mut m = Model::new(8).unwrap();
    m.begin(Role::Initial, 1, 0);
    assert_eq!(m.context(), Some((1, 1, 0, 1)));
    m.complete(Status::RefreshCompleted);
    m.begin(Role::Gameplay, 1, 1);
    assert_eq!(m.context(), Some((1, 2, 1, 2)));
    // A presentation request followed by a host error remains Open, not committed.
    m.begin(Role::Gameplay, 1, 1);
    assert_eq!(m.context(), Some((1, 2, 1, 3)));
    m.complete(Status::HostCommitted);
    m.boundary(Boundary::Restart);
    m.begin(Role::Initial, 2, 1);
    assert_eq!(m.context(), Some((1, 3, 1, 4)));
    assert_eq!(m.attempts[1].status, Status::Open);
    assert_eq!(m.attempts[2].status, Status::HostCommitted);
    assert_eq!(m.segments[2].session_generation, 2);
    assert_eq!(m.segments[2].role, Role::Initial);
}
#[test]
fn focus_resize_retry_and_recreation_split_identity_without_advancing_host_steps() {
    let mut m = Model::new(8).unwrap();
    m.begin(Role::Gameplay, 1, 9);
    let first = m.context().unwrap();
    for reason in [
        Boundary::Focus,
        Boundary::Suspend,
        Boundary::Resume,
        Boundary::Resize,
        Boundary::Configure,
        Boundary::Inactive,
    ] {
        m.boundary(reason);
    }
    m.failure(Failure::Outdated);
    m.boundary(Boundary::Retry);
    let retry = m.context().unwrap();
    assert_eq!((first.2, first.3), (retry.2, retry.3));
    assert!(retry.1 > first.1);
    m.recreated();
    // At most two actual tries: record the third in a new host attempt.
    m.complete(Status::Deferred);
    m.begin(Role::Gameplay, 1, 9);
    let new_surface = m.context().unwrap();
    assert_eq!(new_surface.0, 2);
    assert_eq!(new_surface.2, 9);
    assert!(new_surface.3 > first.3);
    assert_eq!(m.acquisition_failures[Failure::Outdated as usize], 1);
    assert_eq!(m.boundary_counts[Boundary::Focus as usize], 1);
}
#[test]
fn metadata_capacity_refuses_observation_without_losing_prior_records() {
    let mut m = Model::new(1).unwrap();
    let attempts_capacity = m.attempts.capacity();
    let segments_capacity = m.segments.capacity();
    m.begin(Role::Initial, 1, 0);
    assert!(m.context().is_some());
    m.boundary(Boundary::Retry);
    assert!(m.context().is_some());
    m.complete(Status::Deferred);
    m.begin(Role::Gameplay, 1, 1);
    assert!(m.context().is_none());
    assert!(m.exhausted);
    assert_eq!(m.omitted_attempts, 1);
    assert_eq!(m.attempts.len(), 1);
    assert_eq!(m.segments.len(), 2);
    assert_eq!(m.attempts.capacity(), attempts_capacity);
    assert_eq!(m.segments.capacity(), segments_capacity);
}
#[test]
fn all_acquisition_refusals_are_distinct_and_do_not_invent_a_drawable() {
    let mut m = Model::new(2).unwrap();
    for failure in [
        Failure::Timeout,
        Failure::Occluded,
        Failure::Outdated,
        Failure::Lost,
        Failure::Validation,
        Failure::UnsupportedSize,
        Failure::Minimized,
        Failure::Reservation,
        Failure::Device,
    ] {
        m.failure(failure);
    }
    assert_eq!(m.acquisition_failures, [1; 9]);
    assert!(m.attempts.is_empty() && m.segments.is_empty());
}
#[test]
fn output_is_create_only_and_early_setup_error_has_explicit_incomplete_status() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    let mut capture = Capture::new(path.to_str().unwrap(), 8, 6).unwrap();
    capture.begin(Role::Initial, 1, 0);
    assert!(Capture::new(path.to_str().unwrap(), 8, 6).is_err());
    drop(capture);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        value["finalization"]["stage"],
        "abandoned_before_finalization"
    );
    assert_eq!(value["coverage"]["rows"], 0);
    assert_eq!(value["host"]["attempts"][0]["status"], "open");
    assert_eq!(value["display_gate"], false);
}
#[test]
fn finalization_reports_host_failure_without_inventing_process_exit_or_timestamps() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    let mut capture = Capture::new(path.to_str().unwrap(), 8, 6).unwrap();
    let result = Err("original gameplay failure".into());
    capture.finish(2, 1, 0, &result, None).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        value["finalization"]["window_error"],
        "original gameplay failure"
    );
    assert_eq!(value["finalization"]["accepted_host_steps"], 2);
    assert!(value["final_process_exit_status"].is_null());
    assert_eq!(value["rows"], serde_json::json!([]));
}
#[test]
fn output_writer_refuses_growth_and_original_failure_has_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let file = File::create(dir.path().join("limited")).unwrap();
    let mut writer = Limited { file, remaining: 3 };
    writer.write_all(b"abc").unwrap();
    assert!(writer.write_all(b"d").is_err());
    assert_eq!(std::fs::read(dir.path().join("limited")).unwrap(), b"abc");
    let result: session::Result<serde_json::Value> = Err("original".into());
    assert_eq!(
        preserve_result(result, Err("sidecar".into()))
            .unwrap_err()
            .to_string(),
        "original"
    );
    assert_eq!(
        preserve_result(Ok(serde_json::Value::Null), Err("sidecar".into()))
            .unwrap_err()
            .to_string(),
        "sidecar"
    );
}

// Execute the exact vendored collector source without pulling upstream HAL's
// example/dev-dependency graph into the engine. The real HAL API compiles above.
#[allow(
    dead_code,
    reason = "Pure model has no native adapter for Error::Unavailable"
)]
#[path = "../../../../../third_party/wgpu-hal/src/metal/presentation.rs"]
mod collector_model;
