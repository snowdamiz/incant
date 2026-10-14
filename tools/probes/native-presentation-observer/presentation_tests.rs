use super::*;
fn binding(c: &Arc<Collector>, generation: u64, segment: u64, step: u64) -> Observation {
    Observation::new(
        Arc::clone(c),
        Context::new(generation, segment, step).unwrap(),
    )
    .unwrap()
}
#[test]
fn bounded_storage_and_overflow_remain_explicit() {
    assert!(matches!(Collector::new(0), Err(Error::Capacity)));
    assert!(matches!(
        Collector::new(MAX_SAMPLES + 1),
        Err(Error::Capacity)
    ));
    let c = Collector::new(1).unwrap();
    let b = binding(&c, 1, 1, 1);
    let t = b.acquire(0).unwrap();
    t.request().unwrap().presented(1.0);
    assert!(b.acquire(1).is_none());
    let s = c.snapshot();
    assert_eq!(s.rows.len(), 1);
    assert_eq!(s.overflow, 1);
    assert_eq!(s.rows[0].outcome, Outcome::Presented(1.0));
    assert_eq!(s.intervals().unwrap_err(), Error::Incomplete);
}
#[test]
fn invalid_context_has_no_acquisition_effect() {
    let c = Collector::new(1).unwrap();
    assert_eq!(Context::new(0, 1, 0).unwrap_err(), Error::Context);
    assert_eq!(Context::new(1, 0, 0).unwrap_err(), Error::Context);
    assert!(
        Observation::new(
            Arc::clone(&c),
            Context {
                surface_generation: 0,
                segment: 1,
                host_step: 0
            }
        )
        .is_err()
    );
    assert!(c.snapshot().rows.is_empty());
}
#[test]
fn callbacks_may_complete_out_of_order_without_reordering_frames() {
    let c = Collector::new(2).unwrap();
    let a = binding(&c, 1, 1, 1).acquire(7).unwrap();
    let b = binding(&c, 1, 1, 2).acquire(8).unwrap();
    let first = a.request().unwrap();
    b.request().unwrap().presented(4.02);
    assert_eq!(c.snapshot().intervals().unwrap_err(), Error::Incomplete);
    first.presented(4.0);
    let s = c.snapshot();
    assert_eq!(s.rows[0].context.host_step, 1);
    assert_eq!(s.rows[1].context.host_step, 2);
    let intervals = s.intervals().unwrap();
    assert_eq!((intervals[0].from, intervals[0].to), (0, 1));
    assert!((intervals[0].seconds - 0.02).abs() < 1e-12);
}
#[test]
fn zero_and_discard_do_not_shorten_the_display_gap() {
    let c = Collector::new(4).unwrap();
    let b = binding(&c, 1, 1, 1);
    b.acquire(0).unwrap().request().unwrap().presented(10.0);
    b.acquire(1).unwrap().request().unwrap().presented(0.0);
    b.acquire(2).unwrap().discard();
    b.acquire(3).unwrap().request().unwrap().presented(10.06);
    let s = c.snapshot();
    assert_eq!(s.rows[1].outcome, Outcome::ReportedZero);
    assert_eq!(s.rows[2].outcome, Outcome::Discarded);
    let i = s.intervals().unwrap();
    assert_eq!(i.len(), 1);
    assert_eq!((i[0].from, i[0].to), (0, 3));
    assert!((i[0].seconds - 0.06).abs() < 1e-12);
}
#[test]
fn missing_callback_abandonment_and_invalid_values_refuse_complete_intervals() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        let c = Collector::new(1).unwrap();
        binding(&c, 1, 1, 1)
            .acquire(0)
            .unwrap()
            .request()
            .unwrap()
            .presented(value);
        assert!(matches!(c.snapshot().rows[0].outcome, Outcome::Invalid(_)));
        assert_eq!(c.snapshot().intervals().unwrap_err(), Error::Incomplete);
    }
    let c = Collector::new(2).unwrap();
    let t = binding(&c, 1, 1, 1).acquire(0).unwrap();
    let completion = t.request().unwrap();
    drop(t);
    drop(binding(&c, 1, 1, 2).acquire(1));
    let s = c.snapshot();
    assert_eq!(s.rows[0].outcome, Outcome::Requested);
    assert_eq!(s.rows[1].outcome, Outcome::Abandoned);
    assert_eq!(s.intervals().unwrap_err(), Error::Incomplete);
    completion.presented(12.0);
    assert_eq!(c.snapshot().rows[0].outcome, Outcome::Presented(12.0));
}
#[test]
fn late_callback_retains_only_its_old_collector_and_context() {
    let old = Collector::new(1).unwrap();
    let weak = Arc::downgrade(&old);
    let t = binding(&old, 1, 2, 3).acquire(99).unwrap();
    let completion = t.request().unwrap();
    drop(t);
    drop(old);
    let new = Collector::new(1).unwrap();
    let next = binding(&new, 2, 3, 4).acquire(0).unwrap();
    completion.presented(5.0);
    let retained = weak.upgrade().unwrap();
    assert_eq!(
        retained.snapshot().rows[0].context,
        Context::new(1, 2, 3).unwrap()
    );
    assert_eq!(retained.snapshot().rows[0].outcome, Outcome::Presented(5.0));
    assert_eq!(new.snapshot().rows[0].outcome, Outcome::Acquired);
    drop(retained);
    drop(completion);
    assert!(weak.upgrade().is_none());
    next.discard();
}
#[test]
fn duplicate_completion_does_not_overwrite_first_timestamp() {
    let c = Collector::new(1).unwrap();
    let t = binding(&c, 1, 1, 0).acquire(0).unwrap();
    let done = t.request().unwrap();
    assert!(t.request().is_none());
    done.presented(9.0);
    done.presented(10.0);
    t.discard();
    let s = c.snapshot();
    assert_eq!(s.rows[0].outcome, Outcome::Presented(9.0));
    assert_eq!(s.protocol_errors, 3);
    assert_eq!(s.intervals().unwrap_err(), Error::Incomplete);
}
#[test]
fn segments_and_surface_generations_break_intervals() {
    let c = Collector::new(4).unwrap();
    for (index, (generation, segment, id, time)) in [
        (1, 1, 4, 1.0),
        (1, 2, 5, 50.0),
        (2, 3, 0, 60.0),
        (2, 3, 1, 60.02),
    ]
    .into_iter()
    .enumerate()
    {
        binding(&c, generation, segment, index as u64)
            .acquire(id)
            .unwrap()
            .request()
            .unwrap()
            .presented(time);
    }
    let s = c.snapshot();
    let intervals = s.intervals().unwrap();
    assert_eq!(intervals.len(), 1);
    assert_eq!((intervals[0].from, intervals[0].to), (2, 3));
}
#[test]
fn timestamp_or_drawable_identity_reversal_is_not_sorted_away() {
    for (ids, times) in [
        ([1, 0], [1.0, 2.0]),
        ([1, 1], [1.0, 2.0]),
        ([0, 1], [2.0, 1.0]),
        ([0, 1], [1.0, 1.0]),
    ] {
        let c = Collector::new(2).unwrap();
        for i in 0..2 {
            binding(&c, 1, 1, i as u64)
                .acquire(ids[i])
                .unwrap()
                .request()
                .unwrap()
                .presented(times[i]);
        }
        assert_eq!(c.snapshot().intervals().unwrap_err(), Error::Order);
    }
}
#[test]
fn coverage_keeps_leading_trailing_and_unobserved_segment_rows() {
    let c = Collector::new(5).unwrap();
    let first = binding(&c, 1, 1, 0);
    first.acquire(0).unwrap().request().unwrap().presented(0.0);
    first.acquire(1).unwrap().request().unwrap().presented(10.0);
    first.acquire(2).unwrap().discard();
    let next = binding(&c, 1, 2, 1);
    let pending = next.acquire(3).unwrap();
    let _completion = pending.request().unwrap();
    let acquired = next.acquire(4).unwrap();
    let snapshot = c.snapshot();
    assert_eq!(
        snapshot.coverage(),
        Coverage {
            rows: 5,
            segments: 2,
            acquired: 1,
            requested: 1,
            presented: 1,
            reported_zero: 1,
            discarded: 1,
            ..Coverage::default()
        }
    );
    assert_eq!(snapshot.intervals().unwrap_err(), Error::Incomplete);
    drop(acquired);
    assert_eq!(c.snapshot().coverage().abandoned, 1);
}
