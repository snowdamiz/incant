use super::*;

const AFTER_INITIAL: [Region; 13] = [
    Region::Snapshots,
    Region::GroupSetup,
    Region::ApiSetup,
    Region::Capabilities,
    Region::UndoCapture,
    Region::ViewExposure,
    Region::Callbacks,
    Region::RevokeChecks,
    Region::Validation,
    Region::TransactionStage,
    Region::ScopeCleanup,
    Region::Publication,
    Region::HeapObservation,
];
fn complete_tick() {
    let mut trace = Frame::start();
    let _tick = crate::scope(crate::Phase::ScriptTick);
    for region in AFTER_INITIAL {
        mark(region);
        match region {
            Region::Callbacks => {
                for id in [[1; 16], [2; 16]] {
                    let _source = crate::script_scope(Some(id));
                    let _callback = Span::callback(Some(id));
                    std::hint::black_box(id);
                }
            }
            Region::Validation => {
                drop(Span::numeric(NumericRegion::Prepare));
            }
            Region::ScopeCleanup => {
                drop(Span::numeric(NumericRegion::ChangeTicks));
                drop(Span::numeric(NumericRegion::PublishGlobals));
            }
            _ => (),
        }
    }
    trace.finish(true);
}
#[test]
fn capture_correlates_exact_tick_and_rejects_missing_nested_and_retired_traces() {
    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        complete_tick();
        Ok::<_, ()>(())
    })
    .unwrap();
    let report = capture.report();
    assert_eq!(report.version, 4);
    let d = report.script_phase_diagnostic;
    assert!(d.diagnostic_only && !d.eligible_for_performance_gate && d.complete);
    assert_eq!(d.rows.len(), 1);
    let r = &d.rows[0];
    assert_eq!(r.frame, 1);
    assert_eq!(r.trace_ns, r.regions_ns.iter().sum::<u64>());
    assert!(r.trace_ns >= r.whole_tick_ns);
    assert_eq!(
        r.whole_tick_ns,
        report
            .zones
            .iter()
            .find(|z| z.name == "script.tick")
            .unwrap()
            .totals
            .wall_ns
    );
    assert_eq!(r.source_calls.iter().sum::<u16>(), 2);
    assert_eq!(r.numeric_calls, [1, 1, 1]);
    assert!(r.source_ns.iter().sum::<u64>() <= r.regions_ns[Region::Callbacks as usize]);
    assert!(r.numeric_ns[0] <= r.regions_ns[Region::Validation as usize]);
    assert!(r.numeric_ns[1] + r.numeric_ns[2] <= r.regions_ns[Region::ScopeCleanup as usize]);
    let encoded = serde_json::to_value(&r).unwrap();
    assert_eq!(
        encoded["source_ns"].as_array().unwrap().len(),
        crate::MAX_SOURCES
    );
    drop(capture);

    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        let mut trace = Frame::start();
        let _tick = crate::scope(crate::Phase::ScriptTick);
        mark(Region::Validation); // deliberately omit intervening regions
        trace.finish(true);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert!(!capture.report().script_phase_diagnostic.complete);
    drop(capture);

    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        let _outer = Frame::start();
        let _nested = Frame::start();
        Ok::<_, ()>(())
    })
    .unwrap();
    let d = capture.report().script_phase_diagnostic;
    assert!(!d.complete);
    assert_eq!(d.attempted, 2);
    drop(capture);

    let old_capture = crate::Capture::start().unwrap();
    let old_trace = Frame::start();
    drop(old_capture);
    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        complete_tick();
        Ok::<_, ()>(())
    })
    .unwrap();
    drop(old_trace); // must not append into or invalidate the new generation
    let d = capture.report().script_phase_diagnostic;
    assert!(d.complete);
    assert_eq!(d.rows.len(), 1);
    drop(capture);

    let capture = crate::Capture::start().unwrap();
    let result = std::panic::catch_unwind(|| {
        let _: Result<(), ()> = crate::frame(|| {
            let _trace = Frame::start();
            let _tick = crate::scope(crate::Phase::ScriptTick);
            panic!("deliberate diagnostic unwind")
        });
    });
    assert!(result.is_err());
    let d = capture.report().script_phase_diagnostic;
    assert!(!d.complete);
    assert_eq!(d.rows.len(), 1);
    assert!(d.rows[0].unwound && !d.rows[0].success);
    drop(capture);

    let capture = crate::Capture::start().unwrap();
    std::thread::spawn(|| drop(Frame::start())).join().unwrap();
    let d = capture.report().script_phase_diagnostic;
    assert!(!d.complete && d.rows.is_empty());
    assert_eq!(d.attempted, 1);
}
#[test]
fn fixed_storage_overflow_is_reported_without_growing_or_dropping_successfully() {
    let mut storage = Storage::allocated().unwrap();
    for frame in 1..=MAX_FRAMES as u64 {
        let id = storage.start(1, Some(frame)).unwrap();
        let mut row = RawRow::new(id);
        row.success = true;
        storage.append(row);
    }
    assert!(storage.start(1, Some(MAX_FRAMES as u64 + 1)).is_none());
    let report = storage.report();
    assert!(!report.complete);
    assert_eq!(report.attempted, MAX_FRAMES as u64 + 1);
    assert_eq!(report.rows.len(), MAX_FRAMES);
    assert!(report.raw_storage_bound_bytes <= 4 * 1024 * 1024);
}
#[test]
fn boundary_arithmetic_refuses_regression_and_accumulation_overflow() {
    let now = Instant::now();
    let mut valid = true;
    let later = now + std::time::Duration::from_nanos(1);
    assert_eq!(elapsed(later, now, &mut valid), 0);
    assert!(!valid);
    valid = true;
    let mut value = u64::MAX;
    add(&mut value, 1, &mut valid);
    assert!(!valid);
    assert_eq!(value, u64::MAX);
}
