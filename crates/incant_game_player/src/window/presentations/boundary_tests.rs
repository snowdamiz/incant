use super::{Role, Status, boundaries::*, model::Model};

#[test]
fn a_wake_is_consumed_by_one_pulse_even_if_that_pulse_has_no_tick() {
    let mut wakes = Wakes::default();
    wakes.event(WakeCause::WaitCancelled, Some(10), true, Some(20));
    let first = wakes.pulse(Some(15)).unwrap();
    assert_eq!(first.sequence, 1);
    assert_eq!(first.event.unwrap().arrived_ns, Some(10));
    assert_eq!(first.event.unwrap().requested_resume_ns, Some(20));
    // Preserve early arrival as timestamps; no saturating lateness conversion.
    assert_eq!(i128::from(10) - i128::from(20), -10);
    let without_wake = wakes.pulse(Some(25)).unwrap();
    assert!(without_wake.event.is_none());
    assert_eq!(without_wake.events_since_previous_pulse, 0);
    assert_eq!(wakes.counts.pulses_without_event, 1);
    wakes.event(WakeCause::ResumeTimeReached, Some(30), true, Some(29));
    let next = wakes.pulse(Some(31)).unwrap();
    assert_eq!(next.event.unwrap().sequence, 2);
    assert_eq!(next.sequence, 3);
}

#[test]
fn multiple_wakes_retain_ambiguity_instead_of_becoming_one_timer_sample() {
    let mut wakes = Wakes::default();
    wakes.event(WakeCause::Poll, Some(10), false, None);
    wakes.event(WakeCause::ResumeTimeReached, Some(11), true, Some(9));
    let pulse = wakes.pulse(Some(15)).unwrap();
    assert_eq!(pulse.events_since_previous_pulse, 2);
    assert_eq!(pulse.event.unwrap().sequence, 2);
    assert_eq!(wakes.counts.superseded_events, 1);
    assert_eq!(wakes.counts.events, 2);
}

#[test]
fn sequence_exhaustion_is_explicit_and_never_reuses_the_last_wake() {
    let mut wakes = Wakes::default();
    wakes.event(WakeCause::Init, Some(0), false, None);
    wakes.counts.events = u64::MAX;
    wakes.event(WakeCause::Poll, Some(2), false, None);
    assert!(wakes.counts.overflow);
    assert!(wakes.pulse(Some(3)).unwrap().event.is_none());
    wakes.counts.pulses = u64::MAX;
    assert!(wakes.pulse(Some(4)).is_none());
    assert_eq!(wakes.counts.pulses, u64::MAX);
}

#[test]
fn acquisition_retry_keeps_both_actual_results_and_durations_without_growth() {
    let mut m = Model::new(2).unwrap();
    let capacity = m.attempts.capacity();
    m.begin(Role::Gameplay, 1, 1, Some(10));
    m.acquisition_begin(Some(12));
    m.acquisition_end(Some(20), AcquisitionOutcome::Outdated);
    m.acquisition_begin(Some(25));
    m.acquisition_end(Some(35), AcquisitionOutcome::Success);
    m.reserved(Some(40));
    m.complete(Status::HostCommitted, Some(50));
    m.host_finished(Some(55));
    let timing = m.attempts[0].host_timing;
    assert_eq!(timing.acquisitions[0].unwrap().began_ns, Some(12));
    assert_eq!(timing.acquisitions[0].unwrap().ended_ns, Some(20));
    assert_eq!(timing.acquisitions[1].unwrap().began_ns, Some(25));
    assert_eq!(timing.acquisitions[1].unwrap().ended_ns, Some(35));
    assert_eq!(m.timing_errors, 0);
    assert_eq!(m.attempts.capacity(), capacity);
}

#[test]
fn failures_unwind_and_duplicate_calls_do_not_invent_a_successful_acquisition() {
    let mut m = Model::new(4).unwrap();
    m.begin(Role::Gameplay, 1, 1, Some(10));
    m.acquisition_begin(Some(12));
    m.acquisition_end(Some(20), AcquisitionOutcome::Timeout);
    m.complete(Status::Deferred, Some(21));
    assert!(m.attempts[0].host_timing.reserved_ns.is_none());
    m.begin(Role::Gameplay, 1, 1, Some(30));
    m.acquisition_begin(Some(31)); // Unwound call keeps end/outcome absent.
    m.begin(Role::Gameplay, 1, 1, Some(40));
    m.acquisition_end(Some(42), AcquisitionOutcome::Success);
    assert_eq!(m.timing_errors, 1);
    let incomplete = m.attempts[1].host_timing.acquisitions[0].unwrap();
    assert!(incomplete.ended_ns.is_none() && incomplete.outcome.is_none());
    assert!(
        m.attempts[2]
            .host_timing
            .acquisitions
            .iter()
            .all(Option::is_none)
    );
    m.acquisition_begin(Some(43));
    m.acquisition_begin(Some(44));
    assert_eq!(m.timing_errors, 2);
    m.acquisition_end(Some(42), AcquisitionOutcome::Lost);
    assert_eq!(m.timing_errors, 3);
    assert!(
        m.attempts[2].host_timing.acquisitions[0]
            .unwrap()
            .ended_ns
            .is_none()
    );
}

#[test]
fn serialization_is_versioned_with_explicit_larger_fixed_record_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v2.json");
    let mut capture = super::Capture::new(path.to_str().unwrap(), 1, 1).unwrap();
    capture.new_events(tao::event::StartCause::Init, capture.clock_origin);
    capture.pulse(capture.clock_origin);
    capture.gameplay_begin(
        1,
        1,
        super::Schedule {
            pulse: capture.clock_origin,
            deadline: None,
            due: 1,
            ordinal: 0,
            discarded_ticks: 0,
        },
    );
    drop(capture);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(value["version"], 2);
    assert_eq!(value["host_timing_contract"]["version"], 2);
    assert_eq!(value["limits"]["attempt_bytes_max"], 512);
    assert_eq!(value["wake_tracking"]["events"], 1);
    assert_eq!(
        value["host"]["attempts"][0]["host_timing"]["acquisitions"],
        serde_json::json!([null, null])
    );
    assert!(std::mem::size_of::<super::model::Attempt>() <= 512);
}
