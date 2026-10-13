use super::*;

#[test]
fn absent_capability_fails_explicitly_without_changing_defaults() {
    assert_eq!(
        required_features(false, wgpu::Features::empty()).unwrap(),
        wgpu::Features::empty()
    );
    assert_eq!(
        required_features(true, wgpu::Features::empty()),
        Err(GpuTimingError::Unsupported)
    );
    assert_eq!(
        required_features(true, wgpu::Features::TIMESTAMP_QUERY).unwrap(),
        wgpu::Features::TIMESTAMP_QUERY
    );
}
#[test]
fn integer_delta_preserves_small_intervals_and_rejects_reversed_clocks() {
    assert_eq!(span(u64::MAX - 2, u64::MAX, 0.5).unwrap().nanoseconds, 1.);
    assert_eq!(span(19, 19, 1.).unwrap().nanoseconds, 0.); // Quantization can yield zero.
    assert_eq!(span(u64::MAX, 0, 1.), Err(GpuTimingError::Reversed));
    for period in [f32::NAN, f32::INFINITY, 0., -1.] {
        assert_eq!(span(0, 1, period), Err(GpuTimingError::Period));
    }
}
fn sample(sequence: u64) -> GpuFrameTiming {
    GpuFrameTiming {
        sequence,
        timestamp_period_ns: 1.,
        frame: span(1, 3, 1.).unwrap(),
        passes: [None; GPU_PASS_COUNT],
    }
}
#[test]
fn fixed_capacity_skips_and_reuses_only_completed_consumed_or_abandoned_slots() {
    let pool = pool::Pool::default();
    let a = pool.reserve().unwrap().unwrap();
    let b = pool.reserve().unwrap().unwrap();
    let c = pool.reserve().unwrap().unwrap();
    assert_eq!(pool.stats().unwrap().pending, GPU_TIMING_CAPACITY);
    assert!(pool.reserve().unwrap().is_none());
    let second = b.sequence;
    b.complete(Ok(sample(second)));
    assert!(pool.reserve().unwrap().is_none()); // Unread results are backpressure.
    assert_eq!(pool.take().unwrap(), Some(Ok(sample(second))));
    let d = pool.reserve().unwrap().unwrap();
    assert_eq!(d.index, 1);
    assert!(d.sequence > second);
    drop(a); // Command buffer abandoned before submit.
    let e = pool.reserve().unwrap().unwrap();
    assert_eq!(e.index, 0);
    c.complete(Err(GpuTimingError::Readback));
    assert_eq!(pool.take().unwrap(), Some(Err(GpuTimingError::Readback)));
    drop(d);
    drop(e);
    assert_eq!(
        pool.stats().unwrap(),
        GpuTimingStats {
            enabled: true,
            skipped: 2,
            abandoned: 3,
            retired: 1,
            ..Default::default()
        }
    );
}

#[test]
fn failed_mapping_is_delivered_once_and_its_buffer_slot_is_never_reserved_again() {
    let pool = pool::Pool::default();
    for index in 0..GPU_TIMING_CAPACITY {
        let lease = pool.reserve().unwrap().unwrap();
        assert_eq!(lease.index, index);
        lease.complete(Err(GpuTimingError::Readback));
        assert_eq!(pool.take().unwrap(), Some(Err(GpuTimingError::Readback)));
        assert!(pool.take().unwrap().is_none());
    }
    assert!(pool.reserve().unwrap().is_none());
    assert_eq!(pool.stats().unwrap().retired, GPU_TIMING_CAPACITY);
    // A successful map with a rejected counter is already unmapped, so that
    // error does not retire a perfectly usable readback resource.
    let clean = pool::Pool::default();
    clean
        .reserve()
        .unwrap()
        .unwrap()
        .complete(Err(GpuTimingError::Reversed));
    assert_eq!(clean.take().unwrap(), Some(Err(GpuTimingError::Reversed)));
    assert_eq!(clean.reserve().unwrap().unwrap().index, 0);
}
#[test]
fn delayed_cross_thread_completion_does_not_change_new_owner_or_reuse_pending_slot() {
    let old = pool::Pool::default();
    let pending = old.reserve().unwrap().unwrap();
    let current = pool::Pool::default();
    let live = current.reserve().unwrap().unwrap();
    drop(old);
    std::thread::spawn(move || pending.complete(Ok(sample(1))))
        .join()
        .unwrap();
    assert!(current.take().unwrap().is_none());
    assert_eq!(current.stats().unwrap().pending, 1);
    live.complete(Ok(sample(1)));
    assert_eq!(current.take().unwrap(), Some(Ok(sample(1))));
    assert!(current.take().unwrap().is_none());
}
#[test]
fn decoded_pass_labels_follow_their_query_pairs_not_execution_order() {
    let raw: Vec<_> = [100u64, 150, 160, 200, 210, 400]
        .into_iter()
        .flat_map(u64::to_ne_bytes)
        .collect();
    let mut pairs = [None; GPU_PASS_COUNT];
    pairs[GpuPass::Display.index()] = Some(4);
    pairs[GpuPass::InitializeScene.index()] = Some(2);
    let result = recorder::decode(&raw, 37, 0.5, pairs, 6).unwrap();
    assert_eq!(result.sequence, 37);
    assert_eq!(result.frame.nanoseconds, 150.);
    assert_eq!(
        result.passes[GpuPass::Display.index()].unwrap().nanoseconds,
        95.
    );
    assert!(result.passes[GpuPass::Models.index()].is_none());
    assert_eq!(
        recorder::decode(&raw[..raw.len() - 1], 37, 1., pairs, 6),
        Err(GpuTimingError::Payload)
    );
    pairs[GpuPass::Models.index()] = Some(6);
    assert_eq!(
        recorder::decode(&raw, 37, 1., pairs, 6),
        Err(GpuTimingError::Payload)
    );
}
#[test]
fn pool_bookkeeping_and_timestamp_conversion_allocate_nothing_after_setup() {
    let pool = pool::Pool::default();
    // Initialize the platform mutex's thread parking state outside measurement.
    let first = pool.reserve().unwrap().unwrap();
    first.complete(Ok(sample(1)));
    pool.take().unwrap();
    let count = crate::retained_alloc::measured(|| {
        for _ in 0..1000 {
            let lease = pool.reserve().unwrap().unwrap();
            let result = sample(lease.sequence);
            lease.complete(Ok(result));
            assert_eq!(pool.take().unwrap(), Some(Ok(result)));
        }
    });
    assert_eq!(count, 0);
}

#[test]
fn zero_filled_queries_are_refused_without_rejecting_nonzero_quantized_intervals() {
    assert_eq!(
        recorder::decode(&[0; 32], 1, 1., [None; GPU_PASS_COUNT], 4),
        Err(GpuTimingError::Unwritten)
    );
    let raw: Vec<_> = [19u64, 19].into_iter().flat_map(u64::to_ne_bytes).collect();
    assert_eq!(
        recorder::decode(&raw, 1, 1., [None; GPU_PASS_COUNT], 2)
            .unwrap()
            .frame
            .nanoseconds,
        0.
    );
    let pool = pool::Pool::default();
    pool.reserve()
        .unwrap()
        .unwrap()
        .complete(Err(GpuTimingError::Unwritten));
    assert_eq!(pool.take().unwrap(), Some(Err(GpuTimingError::Unwritten)));
    assert!(pool.take().unwrap().is_none());
    assert_eq!(pool.stats().unwrap().retired, 0);
    assert_eq!(pool.reserve().unwrap().unwrap().index, 0);
}
