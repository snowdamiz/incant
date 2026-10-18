use super::*;
use crate::diagnostic::{Frame, Region, Span as Outer, mark};

fn tick(body: impl FnOnce()) {
    let mut trace = Frame::start();
    let _tick = crate::scope(crate::Phase::ScriptTick);
    for region in [
        Region::Snapshots,
        Region::GroupSetup,
        Region::ApiSetup,
        Region::Capabilities,
        Region::UndoCapture,
        Region::ViewExposure,
        Region::Callbacks,
    ] {
        mark(region);
    }
    {
        let _source = crate::script_scope(Some([1; 16]));
        let _callback = Outer::callback(Some([1; 16]));
        body();
    }
    for region in [
        Region::RevokeChecks,
        Region::Validation,
        Region::TransactionStage,
        Region::ScopeCleanup,
        Region::Publication,
        Region::HeapObservation,
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
    capture.report().script_phase_diagnostic
}
fn stages(call: &Call) {
    for stage in [Stage::Parse, Stage::Execute, Stage::Materialize] {
        call.stage(stage).finish(true);
    }
}

#[test]
fn exact_categories_stages_sources_and_early_refusals_are_retained() {
    let report = capture(|| {
        for capability in [
            Capability::Character,
            Capability::Triggers,
            Capability::Navigation,
            Capability::Locale,
            Capability::Localize,
            Capability::Other,
        ] {
            let call = Call::start(capability);
            stages(&call);
            call.finish(true);
        }
        drop(Call::start(Capability::Locale)); // admission refusal before parsing
        let call = Call::start(Capability::Localize);
        call.stage(Stage::Parse).finish(false);
        call.finish(false);
    });
    assert!(report.complete && !report.eligible_for_performance_gate);
    assert_eq!(
        report.schema,
        5 + u32::from(cfg!(feature = "diagnostic-navigation"))
            + 2 * u32::from(cfg!(feature = "diagnostic-script-budget"))
    );
    assert_eq!(report.query_capabilities, CAPABILITIES);
    assert_eq!(report.query_regions, REGIONS);
    let q = &report.rows[0].queries;
    assert_eq!(q.calls[Capability::Locale as usize], [2, 1, 1, 1]);
    assert_eq!(q.failed[Capability::Locale as usize], [1, 0, 0, 0]);
    assert_eq!(q.calls[Capability::Localize as usize], [2, 2, 1, 1]);
    assert_eq!(q.failed[Capability::Localize as usize], [1, 1, 0, 0]);
    assert_eq!(q.source_masks, [1; 6]);
    assert_eq!(q.recursive_calls, [0; 6]);
    for times in q.inclusive_ns {
        assert!(times[1..].iter().sum::<u64>() <= times[0]);
    }
    assert!(
        q.inclusive_ns.iter().map(|t| t[0]).sum::<u64>()
            <= report.rows[0].source_ns.iter().sum::<u64>()
    );
    assert!(report.raw_row_bytes <= 2048 && report.raw_storage_bound_bytes <= 8 * 1024 * 1024);
    assert_eq!(
        serde_json::to_value(q).unwrap()["calls"][3],
        serde_json::json!([2, 1, 1, 1])
    );
}
#[test]
fn recursive_getter_observations_remain_inclusive_and_invalidate_coverage() {
    let report = capture(|| {
        let outer = Call::start(Capability::Localize);
        let parse = outer.stage(Stage::Parse);
        let inner = Call::start(Capability::Locale);
        stages(&inner);
        inner.finish(true);
        parse.finish(true);
        outer.stage(Stage::Execute).finish(true);
        outer.stage(Stage::Materialize).finish(true);
        outer.finish(true);
    });
    assert!(!report.complete && !report.rows[0].valid);
    let q = &report.rows[0].queries;
    assert_eq!(q.recursive_calls, [0, 0, 0, 1, 0, 0]);
    assert_eq!(q.calls[3], [1; 4]);
    assert_eq!(q.calls[4], [1; 4]);
    assert!(q.inclusive_ns[3][0] <= q.inclusive_ns[4][1]);
}
#[test]
fn missing_reordered_overlapping_and_unclosed_stages_cannot_claim_coverage() {
    for case in 0..5 {
        let report = capture(|| {
            let call = Call::start(Capability::Triggers);
            match case {
                0 => {
                    call.stage(Stage::Execute).finish(true);
                    call.finish(true);
                }
                1 => {
                    let parse = call.stage(Stage::Parse);
                    call.stage(Stage::Parse).finish(true);
                    parse.finish(true);
                }
                2 => {
                    drop(call.stage(Stage::Parse));
                }
                3 => {
                    std::mem::forget(call.stage(Stage::Parse));
                }
                _ => {
                    std::mem::forget(call);
                }
            }
        });
        assert!(!report.complete && !report.rows[0].valid, "case {case}");
    }
}
#[test]
fn counter_and_clock_anomalies_are_invalid_not_valid_zero_or_wrapped() {
    for case in 0..3 {
        let report = capture(|| {
            let mut call = Call::start(Capability::Other);
            match case {
                0 => ACTIVE
                    .with_borrow_mut(|s| s.as_mut().unwrap().row.queries.calls[5][0] = u32::MAX),
                1 => {
                    ACTIVE.with_borrow_mut(|s| {
                        s.as_mut().unwrap().row.queries.inclusive_ns[5][0] = u64::MAX
                    });
                    call.token.as_mut().unwrap().3 =
                        Instant::now() - std::time::Duration::from_millis(1);
                }
                _ => {
                    call.token.as_mut().unwrap().3 =
                        Instant::now() + std::time::Duration::from_secs(1)
                }
            }
            drop(call);
        });
        assert!(!report.complete && !report.rows[0].valid);
        if case == 0 {
            assert_eq!(report.rows[0].queries.calls[5][0], u32::MAX);
        }
        if case == 1 {
            assert_eq!(report.rows[0].queries.inclusive_ns[5][0], u64::MAX);
        }
    }
}
#[test]
fn no_capture_no_trace_and_other_thread_cannot_add_query_rows() {
    let inert = Call::start(Capability::Locale);
    stages(&inert);
    inert.finish(true);
    let report = capture(|| {
        std::thread::spawn(|| {
            let call = Call::start(Capability::Locale);
            stages(&call);
            call.finish(true);
        })
        .join()
        .unwrap();
    });
    assert!(report.complete);
    assert_eq!(report.rows[0].queries.calls, [[0; 4]; 6]);
}
#[test]
fn retired_call_and_stage_cannot_modify_successor_capture() {
    let old = crate::Capture::start().unwrap();
    let frame = Frame::start();
    mark(Region::Callbacks);
    let source = crate::script_scope(Some([1; 16]));
    let callback = Outer::callback(Some([1; 16]));
    let call = Call::start(Capability::Locale);
    let parse = call.stage(Stage::Parse);
    drop(old);
    let report = capture(|| {
        drop(parse);
        drop(call);
        drop(callback);
        drop(source);
        drop(frame);
        let call = Call::start(Capability::Triggers);
        stages(&call);
        call.finish(true);
    });
    assert!(report.complete);
    assert_eq!(report.rows[0].queries.calls[1], [1; 4]);
    assert_eq!(report.rows[0].queries.calls[3], [0; 4]);
}
#[test]
fn unwind_is_retained_even_when_the_caller_catches_it() {
    let report = capture(|| {
        assert!(
            std::panic::catch_unwind(|| {
                let call = Call::start(Capability::Localize);
                let _parse = call.stage(Stage::Parse);
                panic!("deliberate query unwind");
            })
            .is_err()
        );
    });
    assert!(!report.complete);
    assert_eq!(report.rows[0].queries.failed[4], [1, 1, 0, 0]);
}
