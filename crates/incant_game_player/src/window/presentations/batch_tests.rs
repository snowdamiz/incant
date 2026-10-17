use super::model::{Model, Role, Schedule, Status};

fn scheduled(due: u8) -> Model {
    let mut model = Model::new(8).unwrap();
    model.begin(Role::Gameplay, 2, 42, Some(20));
    model.schedule(Schedule {
        pulse_ns: Some(10),
        deadline_ns: Some(8),
        forced_resume: false,
        due,
        ordinal: 0,
        discarded_ticks: Some(0),
    });
    model.batch_begin(usize::from(due));
    model.reserved(Some(25));
    model
}

#[test]
fn callback_identity_stays_at_first_step_while_receipt_binds_final_display() {
    let mut model = scheduled(4);
    let context = model.context().unwrap();
    assert_eq!(context.2, 42);
    for step in 42..=45 {
        model.batch_accept(step, step == 45);
    }
    assert_eq!(model.context().unwrap(), context);
    model.complete(Status::HostCommitted, Some(50));
    model.host_finished(Some(55));
    let batch = model.attempts[0].batch.unwrap();
    assert_eq!((batch.planned_ticks, batch.accepted_ticks), (4, 4));
    assert_eq!(batch.displayed_host_step, Some(45));
    assert_eq!(model.timing_errors, 0);
    assert_eq!(model.attempts[0].host_timing.finished_ns, Some(55));
}

#[test]
fn early_boundary_preserves_accepted_prefix_and_deferred_batch_accepts_nothing() {
    let mut early = scheduled(4);
    early.batch_accept(42, false);
    early.batch_accept(43, true);
    early.complete(Status::HostCommitted, Some(50));
    let batch = early.attempts[0].batch.unwrap();
    assert_eq!(
        (batch.accepted_ticks, batch.displayed_host_step),
        (2, Some(43))
    );
    assert_eq!(early.timing_errors, 0);
    let mut deferred = scheduled(2);
    deferred.complete(Status::Deferred, Some(50));
    assert_eq!(deferred.attempts[0].batch.unwrap().accepted_ticks, 0);
    assert_eq!(deferred.timing_errors, 0);
}

#[test]
fn skipped_repeated_or_post_display_steps_cannot_be_reported_as_valid() {
    for sequence in [
        vec![(43, true)],
        vec![(42, false), (42, true)],
        vec![(42, true), (43, true)],
        vec![(42, false), (43, false), (44, true)],
    ] {
        let mut model = scheduled(2);
        for (step, displayed) in sequence {
            model.batch_accept(step, displayed);
        }
        assert!(model.timing_errors > 0);
    }
    let mut incomplete = scheduled(2);
    incomplete.batch_accept(42, false);
    incomplete.complete(Status::HostCommitted, Some(50));
    assert!(incomplete.timing_errors > 0);
    let mut deferred_after_tick = scheduled(2);
    deferred_after_tick.batch_accept(42, false);
    deferred_after_tick.complete(Status::Deferred, Some(50));
    assert!(deferred_after_tick.timing_errors > 0);
}

#[test]
fn ordinary_attempt_serialization_omits_batch_metadata() {
    let mut model = Model::new(1).unwrap();
    model.begin(Role::Gameplay, 1, 1, Some(1));
    let value = serde_json::to_value(&model.attempts[0]).unwrap();
    assert!(value.get("batch").is_none());
}
