use super::*;
use crate::diagnostic::{Frame, Region as Major, Span as Outer, mark};

fn tick(body: impl FnOnce()) {
    let mut trace = Frame::start();
    let _tick = crate::scope(crate::Phase::ScriptTick);
    for region in [
        Major::Snapshots,
        Major::GroupSetup,
        Major::ApiSetup,
        Major::Capabilities,
        Major::UndoCapture,
        Major::ViewExposure,
        Major::Callbacks,
    ] {
        mark(region);
    }
    {
        let _source = crate::script_scope(Some([1; 16]));
        let _callback = Outer::callback(Some([1; 16]));
        body();
    }
    for region in [
        Major::RevokeChecks,
        Major::Validation,
        Major::TransactionStage,
        Major::ScopeCleanup,
        Major::Publication,
        Major::HeapObservation,
    ] {
        mark(region);
    }
    trace.finish(true);
}
fn capture(body: impl FnOnce()) -> crate::diagnostic::Report {
    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        tick(body);
        Ok::<_, ()>(())
    })
    .unwrap();
    let report = capture.report();
    assert_eq!(report.version, 4);
    report.script_phase_diagnostic
}

#[test]
fn nested_navigation_corresponds_to_the_same_tick_and_retains_failures() {
    let report = capture(|| {
        let query = Span::start(Region::PathQuery);
        Span::start(Region::InitialStart).finish(true);
        Span::start(Region::RepeatedStart).finish(true);
        Span::start(Region::RepeatedStart).finish(false);
        query.finish(true);
        drop(Span::start(Region::PathQuery)); // ordinary early error/absent path
    });
    assert!(report.complete && !report.eligible_for_performance_gate);
    assert_eq!(report.schema, 2);
    assert_eq!(report.navigation_regions, REGIONS);
    let row = &report.rows[0];
    assert_eq!(row.navigation.calls, [2, 1, 2]);
    assert_eq!(row.navigation.failed, [1, 0, 1]);
    assert!(row.navigation.ns[1] + row.navigation.ns[2] <= row.navigation.ns[0]);
    assert!(row.navigation.ns[0] <= row.source_ns.iter().sum::<u64>());
    assert!(row.navigation.ns[0] <= row.regions_ns[Major::Callbacks as usize]);
    assert_eq!(row.trace_ns, row.regions_ns.iter().sum::<u64>());
    assert!(report.raw_row_bytes <= 1024 && report.raw_storage_bound_bytes <= 4 * 1024 * 1024);
    let json = serde_json::to_value(row).unwrap();
    assert_eq!(json["navigation"]["calls"], serde_json::json!([2, 1, 2]));
}

#[test]
fn missing_parent_overlapping_and_unclosed_observations_refuse_complete_coverage() {
    for case in 0..4 {
        let report = capture(|| match case {
            0 => Span::start(Region::InitialStart).finish(true),
            1 => {
                let _a = Span::start(Region::PathQuery);
                let _b = Span::start(Region::PathQuery);
            }
            2 => {
                let a = Span::start(Region::PathQuery);
                let b = Span::start(Region::InitialStart);
                drop(a);
                drop(b);
            }
            _ => std::mem::forget(Span::start(Region::PathQuery)),
        });
        assert!(!report.complete && !report.rows[0].valid, "case {case}");
    }
}

#[test]
fn navigation_counter_overflow_is_invalid_not_wrapped() {
    let report = capture(|| {
        let query = Span::start(Region::PathQuery);
        let lookup = Span::start(Region::RepeatedStart);
        ACTIVE.with_borrow_mut(|slot| slot.as_mut().unwrap().row.navigation.calls[2] = u16::MAX);
        lookup.finish(true);
        query.finish(true);
    });
    assert!(!report.complete);
    assert_eq!(report.rows[0].navigation.calls[2], u16::MAX);
}

#[test]
fn no_active_trace_does_not_claim_an_observation() {
    Span::start(Region::PathQuery).finish(false);
    let report = capture(|| {});
    assert!(report.complete);
    assert_eq!(report.rows[0].navigation.calls, [0; 3]);
}

#[test]
fn retired_navigation_guards_cannot_write_to_a_new_capture() {
    let capture_old = crate::Capture::start().unwrap();
    let frame_old = Frame::start();
    mark(Major::Callbacks);
    let source_old = crate::script_scope(Some([1; 16]));
    let callback_old = Outer::callback(Some([1; 16]));
    let query_old = Span::start(Region::PathQuery);
    let lookup_old = Span::start(Region::RepeatedStart);
    drop(capture_old);
    let report = capture(|| {
        drop(lookup_old);
        drop(query_old);
        drop(callback_old);
        drop(source_old);
        drop(frame_old);
        let query = Span::start(Region::PathQuery);
        Span::start(Region::InitialStart).finish(true);
        query.finish(true);
    });
    assert!(report.complete);
    assert_eq!(report.rows[0].navigation.calls, [1, 1, 0]);
}
