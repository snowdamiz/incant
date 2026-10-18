use super::model::{Display, DisplayReason, Model, Role, Schedule, Status};

fn admission(model: &mut Model, step: u64, begin: u64) {
    model.begin(Role::Simulation, 1, step, Some(begin));
    model.schedule(Schedule {
        pulse_ns: Some(begin),
        deadline_ns: Some(begin),
        forced_resume: false,
        due: 1,
        ordinal: 0,
        discarded_ticks: Some(0),
    });
}
#[test]
fn admission_has_no_drawable_and_display_does_not_invent_another_tick() {
    let mut m = Model::new(8).unwrap();
    for step in 1..=2 {
        admission(&mut m, step, step * 10);
        m.complete(Status::Admitted, Some(step * 10 + 1));
        m.host_finished(Some(step * 10 + 2));
    }
    m.begin(Role::Committed, 1, 2, Some(30));
    m.display(Display {
        reason: DisplayReason::Pulse,
        callback_sequence: Some(1),
        callback_arrived_ns: Some(28),
        callback_target_seconds: Some(9999.),
    });
    m.context().unwrap();
    m.reserved(Some(31));
    m.complete(Status::RefreshCompleted, Some(32));
    assert_eq!(m.attempts.len(), 3);
    assert_eq!(m.timing_errors, 0);
    assert!(
        m.attempts[..2]
            .iter()
            .all(|a| a.host_timing.reserved_ns.is_none() && a.host_timing.finished_ns.is_some())
    );
    assert_eq!(m.attempts[2].requested_host_step, 2);
    assert!(m.attempts[2].host_timing.schedule.is_none());
}
#[test]
fn intervening_simulation_does_not_split_the_display_segment() {
    let mut m = Model::new(8).unwrap();
    m.begin(Role::Committed, 1, 0, Some(1));
    let first = m.context().unwrap().1;
    m.reserved(Some(2));
    m.complete(Status::RefreshCompleted, Some(3));
    admission(&mut m, 1, 4);
    m.complete(Status::Admitted, Some(5));
    m.host_finished(Some(6));
    m.begin(Role::Committed, 1, 1, Some(7));
    assert_eq!(m.context().unwrap().1, first);
    m.boundary(super::model::Boundary::Resize);
    assert_ne!(m.context().unwrap().1, first);
}
#[test]
fn absent_or_fabricated_display_pulse_and_drawable_on_admission_are_errors() {
    for reason in [
        DisplayReason::Pulse,
        DisplayReason::Stop,
        DisplayReason::Restart,
    ] {
        let mut m = Model::new(1).unwrap();
        m.begin(Role::Committed, 1, 1, Some(10));
        let supplied = reason != DisplayReason::Pulse;
        m.display(Display {
            reason,
            callback_sequence: supplied.then_some(1),
            callback_arrived_ns: supplied.then_some(8),
            callback_target_seconds: supplied.then_some(100.),
        });
        assert_eq!(m.timing_errors, 1);
    }
    let mut m = Model::new(1).unwrap();
    admission(&mut m, 1, 10);
    assert!(m.context().is_none());
    m.reserved(Some(11));
    m.complete(Status::Admitted, Some(12));
    assert_eq!(m.timing_errors, 2);
}
#[test]
fn boundary_flush_does_not_claim_callback_or_simulation_and_capacity_stays_bounded() {
    let mut m = Model::new(1).unwrap();
    m.begin(Role::Committed, 1, 3, Some(10));
    m.display(Display {
        reason: DisplayReason::Stop,
        callback_sequence: None,
        callback_arrived_ns: None,
        callback_target_seconds: None,
    });
    m.reserved(Some(11));
    m.complete(Status::RefreshCompleted, Some(12));
    assert_eq!(m.timing_errors, 0);
    admission(&mut m, 4, 15);
    assert!(m.exhausted);
    assert_eq!(m.omitted_attempts, 1);
    assert_eq!(m.attempts.len(), 1);
}

#[test]
fn reused_callback_is_retained_as_an_error_instead_of_another_valid_display() {
    let mut m = Model::new(2).unwrap();
    for step in 1..=2 {
        m.begin(Role::Committed, 1, step, Some(step * 10));
        m.display(Display {
            reason: DisplayReason::Pulse,
            callback_sequence: Some(1),
            callback_arrived_ns: Some(5),
            callback_target_seconds: Some(9999.),
        });
        m.reserved(Some(step * 10 + 1));
        m.complete(Status::RefreshCompleted, Some(step * 10 + 2));
    }
    assert_eq!(m.timing_errors, 1);
    assert_eq!(m.attempts[1].display.unwrap().callback_sequence, Some(1));
}
