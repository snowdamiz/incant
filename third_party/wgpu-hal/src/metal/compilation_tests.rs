use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn context(step: u64) -> Context {
    Context {
        session_generation: 1,
        surface_generation: 2,
        segment: 3,
        host_step: step,
        phase: Phase::Gameplay,
    }
}
fn success(observer: &Observer, kind: Kind) {
    observer
        .begin(kind, Stage::Unspecified)
        .complete(Instant::now(), None);
}
fn observed<T>(
    observer: &Observer,
    operation: impl FnOnce() -> Result<T, ErrorIdentity>,
) -> Result<T, ErrorIdentity> {
    let call = observer.begin(Kind::ComputePipeline, Stage::Compute);
    let result = operation();
    call.complete(Instant::now(), result.as_ref().err().copied());
    result
}

#[test]
fn capacity_and_identity_are_admitted_before_recording() {
    let observer = Observer::default();
    assert_eq!(observer.start(0), Err(Error::Capacity));
    assert_eq!(observer.start(MAX_SAMPLES + 1), Err(Error::Capacity));
    assert!(observer.scope(context(1)).is_err());
    observer.start(1).unwrap();
    assert_eq!(observer.start(1), Err(Error::AlreadyStarted));
    assert!(observer.snapshot().rows.is_empty());
}

#[test]
fn unbound_prefix_is_not_reconstructed_as_timed_rows() {
    let observer = Observer::default();
    let old = observer.begin(Kind::MslLibrary, Stage::Unspecified);
    observer.start(2).unwrap();
    old.complete(Instant::now(), None);
    success(&observer, Kind::MslLibrary);
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].sequence, 2);
    let index = Kind::MslLibrary as usize;
    assert_eq!(snapshot.counts[index].attempts, 2);
    assert_eq!(snapshot.counts[index].successes, 2);
    assert_eq!(snapshot.unbound[index].attempts, 1);
    assert_eq!(snapshot.unbound[index].successes, 1);
}

#[test]
fn device_counters_and_rows_are_independent() {
    let a = Observer::default();
    let b = Observer::default();
    a.start(1).unwrap();
    b.start(1).unwrap();
    success(&a, Kind::RenderPipeline);
    assert_ne!(a.snapshot().device, b.snapshot().device);
    assert_eq!(a.snapshot().rows.len(), 1);
    assert!(b.snapshot().rows.is_empty());
    assert_eq!(b.snapshot().counts, [Counts::default(); KIND_COUNT]);
}

#[test]
fn host_scope_never_attributes_other_thread_work() {
    let observer = Observer::default();
    observer.start(2).unwrap();
    let scope = observer.scope(context(17)).unwrap();
    let cloned = observer.clone();
    thread::spawn(move || success(&cloned, Kind::RenderPipeline))
        .join()
        .unwrap();
    success(&observer, Kind::RenderPipeline);
    drop(scope);
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows[0].context, Context::default());
    assert_eq!(snapshot.rows[1].context, context(17));
    assert!(!snapshot.scope_active);
}

#[test]
fn invalid_and_nested_contexts_refuse_without_overwriting_scope() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    assert!(matches!(
        observer.scope(Context::default()),
        Err(Error::Context)
    ));
    assert!(matches!(observer.scope(context(0)), Err(Error::Context)));
    let scope = observer.scope(context(1)).unwrap();
    assert!(matches!(observer.scope(context(2)), Err(Error::ScopeBusy)));
    success(&observer, Kind::NagaLibrary);
    drop(scope);
    assert_eq!(observer.snapshot().rows[0].context, context(1));
}

#[test]
fn in_flight_call_keeps_entry_scope_when_host_scope_changes() {
    let observer = Observer::default();
    observer.start(2).unwrap();
    let old_scope = observer.scope(context(1)).unwrap();
    let call = observer.begin(Kind::ComputePipeline, Stage::Compute);
    drop(old_scope);
    let new_scope = observer.scope(context(2)).unwrap();
    success(&observer, Kind::ComputePipeline);
    call.complete(Instant::now(), None);
    drop(new_scope);
    let rows = observer.snapshot().rows;
    assert_eq!(rows[0].context, context(1));
    assert_eq!(rows[1].context, context(2));
    assert_eq!(rows[0].sequence, 1);
    assert_eq!(rows[1].sequence, 2);
}

#[test]
fn native_result_and_error_are_returned_unchanged_exactly_once() {
    let observer = Observer::default();
    observer.start(2).unwrap();
    let mut calls = 0;
    let object = Box::new(37);
    let pointer = object.as_ref() as *const i32;
    let returned = observed(&observer, || {
        calls += 1;
        Ok(object)
    })
    .unwrap();
    assert_eq!(returned.as_ref() as *const i32, pointer);
    let error = ErrorIdentity {
        domain: ErrorDomain::ComputePipeline,
        code: -29,
    };
    let failed: Result<(), _> = observed(&observer, || {
        calls += 1;
        Err(error)
    });
    assert_eq!(failed, Err(error));
    assert_eq!(calls, 2);
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows[0].outcome, Outcome::Success);
    assert_eq!(snapshot.rows[1].outcome, Outcome::Error);
    assert_eq!(snapshot.rows[1].error_domain, error.domain);
    assert_eq!(snapshot.rows[1].error_code, error.code);
}

#[test]
fn scope_and_call_unwinding_remain_observable() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _scope = observer.scope(context(1)).unwrap();
        let _call = observer.begin(Kind::RenderPipeline, Stage::Unspecified);
        panic!("injected native-call failure");
    }));
    assert!(result.is_err());
    let snapshot = observer.snapshot();
    assert!(!snapshot.scope_active);
    assert_eq!(snapshot.rows[0].outcome, Outcome::Unwound);
    assert_eq!(snapshot.counts[Kind::RenderPipeline as usize].unwound, 1);
}

#[test]
fn overflow_retains_cumulative_outcomes_without_allocating_rows() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    success(&observer, Kind::RenderPipeline);
    success(&observer, Kind::RenderPipeline);
    drop(observer.begin(Kind::RenderPipeline, Stage::Unspecified));
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.overflow, 2);
    assert_eq!(
        snapshot.counts[Kind::RenderPipeline as usize],
        Counts {
            attempts: 3,
            successes: 2,
            errors: 0,
            unwound: 1
        }
    );
}

#[test]
fn snapshot_does_not_turn_pending_into_completion() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    let pending = observer.begin(Kind::BinaryLibrary, Stage::Unspecified);
    let before = observer.snapshot();
    assert_eq!(before.rows[0].outcome, Outcome::Pending);
    pending.complete(Instant::now(), None);
    assert_eq!(before.rows[0].outcome, Outcome::Pending);
    assert_eq!(observer.snapshot().rows[0].outcome, Outcome::Success);
}

#[test]
fn destruction_does_not_cancel_cumulative_creation_counts() {
    let observer = Observer::default();
    observer.start(2).unwrap();
    for _ in 0..2 {
        let object = observed(&observer, || Ok(Box::new(1))).unwrap();
        drop(object);
    }
    let count = observer.snapshot().counts[Kind::ComputePipeline as usize];
    assert_eq!(count.attempts, 2);
    assert_eq!(count.successes, 2);
}

#[test]
fn counters_and_duration_overflow_flag_saturation() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    {
        let mut state = observer.0.state.lock();
        state.sequence = u64::MAX;
        state.counts[Kind::RenderPipeline as usize].attempts = u64::MAX;
        state.counts[Kind::RenderPipeline as usize].successes = u64::MAX;
    }
    success(&observer, Kind::RenderPipeline);
    let snapshot = observer.snapshot();
    assert!(snapshot.saturated);
    assert_eq!(snapshot.rows[0].sequence, u64::MAX);
    let mut saturated = false;
    assert_eq!(nanos(u128::MAX, &mut saturated), u64::MAX);
    assert!(saturated);
}

#[test]
fn collector_handle_survives_owner_without_native_retention() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    let retained = observer.clone();
    let call = observer.begin(Kind::MslLibrary, Stage::Unspecified);
    drop(observer);
    call.complete(Instant::now(), None);
    assert_eq!(retained.snapshot().rows[0].outcome, Outcome::Success);
}
