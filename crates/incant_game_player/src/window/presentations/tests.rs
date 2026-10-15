use super::*;
use model::{Model, Status};

#[test]
fn attempts_are_not_commits_and_roles_survive_repeated_world_ticks() {
    let mut m = Model::new(8).unwrap();
    m.begin(Role::Initial, 1, 0, Some(0));
    assert_eq!(m.context(), Some((1, 1, 0, 1)));
    m.complete(Status::RefreshCompleted, Some(0));
    m.begin(Role::Gameplay, 1, 1, Some(0));
    assert_eq!(m.context(), Some((1, 2, 1, 2)));
    // A presentation request followed by a host error remains Open, not committed.
    m.begin(Role::Gameplay, 1, 1, Some(0));
    assert_eq!(m.context(), Some((1, 2, 1, 3)));
    m.complete(Status::HostCommitted, Some(0));
    m.boundary(Boundary::Restart);
    m.begin(Role::Initial, 2, 1, Some(0));
    assert_eq!(m.context(), Some((1, 3, 1, 4)));
    assert_eq!(m.attempts[1].status, Status::Open);
    assert_eq!(m.attempts[2].status, Status::HostCommitted);
    assert_eq!(m.segments[2].session_generation, 2);
    assert_eq!(m.segments[2].role, Role::Initial);
}
#[test]
fn focus_resize_retry_and_recreation_split_identity_without_advancing_host_steps() {
    let mut m = Model::new(8).unwrap();
    m.begin(Role::Gameplay, 1, 9, Some(0));
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
    m.complete(Status::Deferred, Some(0));
    m.begin(Role::Gameplay, 1, 9, Some(0));
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
    m.begin(Role::Initial, 1, 0, Some(0));
    assert!(m.context().is_some());
    m.boundary(Boundary::Retry);
    assert!(m.context().is_some());
    m.complete(Status::Deferred, Some(0));
    m.begin(Role::Gameplay, 1, 1, Some(0));
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
fn host_stages_preserve_catch_up_identity_and_measure_only_one_clock() {
    let mut m = Model::new(4).unwrap();
    for (ordinal, start) in [(0, 100), (1, 170)] {
        m.begin(Role::Gameplay, 1, u64::from(ordinal) + 1, Some(start));
        m.schedule(model::Schedule {
            pulse_ns: Some(90),
            deadline_ns: Some(80),
            forced_resume: false,
            due: 2,
            ordinal,
            discarded_ticks: Some(3),
        });
        m.reserved(Some(start + 25));
        m.complete(Status::HostCommitted, Some(start + 50));
        m.host_finished(Some(start + 60));
    }
    let records = serde_json::to_value(&m).unwrap();
    assert_eq!(records["timing_errors"], 0);
    for (i, start) in [(0, 100), (1, 170)] {
        let a = &records["attempts"][i];
        assert_eq!(a["id"], i + 1);
        assert_eq!(a["requested_host_step"], i + 1);
        assert_eq!(a["host_timing"]["began_ns"], start);
        assert_eq!(a["host_timing"]["reserved_ns"], start + 25);
        assert_eq!(a["host_timing"]["completed_ns"], start + 50);
        assert_eq!(a["host_timing"]["finished_ns"], start + 60);
        assert_eq!(a["host_timing"]["schedule"]["pulse_ns"], 90);
        assert_eq!(a["host_timing"]["schedule"]["deadline_ns"], 80);
        assert_eq!(a["host_timing"]["schedule"]["ordinal"], i);
        assert_eq!(a["host_timing"]["schedule"]["discarded_ticks"], 3);
    }
}
#[test]
fn failed_or_deferred_frames_do_not_invent_acquisition_commit_or_finish_times() {
    let mut m = Model::new(5).unwrap();
    m.begin(Role::Gameplay, 1, 1, Some(10));
    m.complete(Status::Deferred, Some(15));
    m.host_finished(Some(20));
    m.begin(Role::Gameplay, 1, 1, Some(30));
    m.reserved(Some(35));
    // Simulation fails, so the next attempt must not close its open predecessor.
    m.begin(Role::Gameplay, 1, 1, Some(40));
    m.reserved(Some(45));
    m.complete(Status::HostCommitted, Some(50));
    // Haptic failure after commit leaves finished absent, including after restart.
    m.begin(Role::Initial, 2, 1, Some(60));
    m.host_finished(Some(65));
    let a = &m.attempts;
    assert_eq!(a[0].status, Status::Deferred);
    assert!(a[0].host_timing.reserved_ns.is_none());
    assert_eq!(a[0].host_timing.completed_ns, Some(15));
    assert!(a[0].host_timing.finished_ns.is_none());
    assert_eq!(a[1].status, Status::Open);
    assert!(a[1].host_timing.completed_ns.is_none());
    assert!(a[1].host_timing.finished_ns.is_none());
    assert_eq!(a[2].status, Status::HostCommitted);
    assert_eq!(a[2].host_timing.completed_ns, Some(50));
    assert!(a[2].host_timing.finished_ns.is_none());
    assert!(a[3].host_timing.finished_ns.is_none());
}
#[test]
fn forced_resume_has_no_scheduled_deadline_and_zero_time_is_valid() {
    let mut m = Model::new(2).unwrap();
    m.begin(Role::Gameplay, 1, 1, Some(0));
    m.schedule(model::Schedule {
        pulse_ns: Some(0),
        deadline_ns: None,
        forced_resume: true,
        due: 1,
        ordinal: 0,
        discarded_ticks: Some(0),
    });
    m.reserved(Some(0));
    m.complete(Status::HostCommitted, Some(0));
    m.host_finished(Some(0));
    assert_eq!(m.timing_errors, 0);
    assert_eq!(m.attempts[0].host_timing.finished_ns, Some(0));
}
#[test]
fn missing_regressing_and_duplicate_observations_are_explicit() {
    let mut m = Model::new(4).unwrap();
    m.begin(Role::Gameplay, 1, 1, None);
    m.reserved(None);
    m.complete(Status::HostCommitted, None);
    m.host_finished(None);
    assert_eq!(m.timing_errors, 4);
    m.begin(Role::Gameplay, 1, 2, Some(50));
    m.reserved(Some(40));
    m.complete(Status::HostCommitted, Some(30));
    assert!(m.attempts[1].host_timing.reserved_ns.is_none());
    assert!(m.attempts[1].host_timing.completed_ns.is_none());
    m.begin(Role::Gameplay, 1, 3, Some(60));
    m.reserved(Some(65));
    m.reserved(Some(70));
    m.complete(Status::HostCommitted, Some(80));
    m.host_finished(Some(75));
    assert_eq!(m.attempts[2].host_timing.reserved_ns, Some(65));
    assert!(m.attempts[2].host_timing.finished_ns.is_none());
    assert_eq!(m.timing_errors, 8);
}
#[test]
fn invalid_schedule_is_retained_without_becoming_valid_timing() {
    for (due, ordinal, pulse, deadline, forced, dropped) in [
        (0, 0, Some(10), Some(5), false, Some(0)),
        (5, 0, Some(10), Some(5), false, Some(0)),
        (2, 2, Some(10), Some(5), false, Some(0)),
        (1, 0, Some(10), Some(15), false, Some(0)),
        (1, 0, Some(30), Some(5), false, Some(0)),
        (1, 0, Some(10), None, false, Some(0)),
        (1, 0, Some(10), Some(5), true, Some(0)),
        (1, 0, None, Some(5), false, Some(0)),
        (1, 0, Some(10), Some(5), false, None),
    ] {
        let mut m = Model::new(1).unwrap();
        m.begin(Role::Gameplay, 1, 1, Some(20));
        m.schedule(model::Schedule {
            pulse_ns: pulse,
            deadline_ns: deadline,
            forced_resume: forced,
            due,
            ordinal,
            discarded_ticks: dropped,
        });
        assert_eq!(m.timing_errors, 1);
        assert!(m.attempts[0].host_timing.schedule.is_some());
    }
}
#[test]
fn capture_clock_refuses_before_origin_and_marks_partial_stage_contract() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    let mut capture = Capture::new(path.to_str().unwrap(), 8, 6).unwrap();
    assert_eq!(capture.time(capture.clock_origin), Some(0));
    assert_eq!(
        capture.time(capture.clock_origin - Duration::from_nanos(1)),
        None
    );
    capture.gameplay_begin(
        1,
        1,
        Schedule {
            pulse: capture.clock_origin,
            deadline: None,
            due: 1,
            ordinal: 0,
            discarded_ticks: 0,
        },
    );
    capture.reserved();
    capture.complete(Status::HostCommitted);
    capture.host_finished();
    drop(capture);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["host"]["timing_errors"], 0);
    assert_eq!(
        value["host_timing_contract"]["shared_epoch_with_metal"],
        false
    );
    assert_eq!(
        value["host_timing_contract"]["changes_frame_admission"],
        false
    );
    assert_eq!(value["limits"]["attempt_bytes_max"], 192);
    let t = &value["host"]["attempts"][0]["host_timing"];
    let times: Vec<_> = ["began_ns", "reserved_ns", "completed_ns", "finished_ns"]
        .map(|key| t[key].as_u64().unwrap())
        .into_iter()
        .collect();
    assert!(times.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(value["coverage"]["rows"], 0);
    assert_eq!(value["display_gate"], false);
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
