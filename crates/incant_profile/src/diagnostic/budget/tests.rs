use super::*;
use crate::diagnostic::{Frame, Region as Major, Span as Outer, mark};

fn tick(other: impl FnOnce(), callback: impl FnOnce()) {
    let mut trace = Frame::start();
    let _tick = crate::scope(crate::Phase::ScriptTick);
    other();
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
        callback();
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
fn capture(other: impl FnOnce(), callback: impl FnOnce()) -> crate::diagnostic::Report {
    let capture = crate::Capture::start().unwrap();
    crate::frame(|| {
        tick(other, callback);
        Ok::<_, ()>(())
    })
    .unwrap();
    capture.report().script_phase_diagnostic
}
fn accepted(origin: Origin) {
    let span = Span::start(origin);
    clock_read();
    span.finish(true);
}
#[test]
fn exact_check_clock_and_refusal_counts_partition_callback_and_other_work() {
    let report = capture(
        || {
            clock_read(); // initialization/reset is not a bracketed check
            accepted(Origin::Explicit);
            let refused = Span::start(Origin::Interrupt);
            clock_read(); // the real provider could refuse after being attempted
            refused.finish(false);
        },
        || {
            accepted(Origin::Interrupt);
            accepted(Origin::Explicit);
            Span::start(Origin::Explicit).finish(false); // latched/owner refusal: no read
        },
    );
    assert!(report.complete && !report.eligible_for_performance_gate);
    assert_eq!(
        report.schema,
        4 * u32::from(cfg!(feature = "diagnostic-script-queries"))
            + if cfg!(feature = "diagnostic-navigation") {
                4
            } else {
                3
            }
    );
    assert_eq!(report.budget_check_regions, REGIONS);
    let row = &report.rows[0];
    assert_eq!(row.budget_checks.calls, [1, 2, 1, 1]);
    assert_eq!(row.budget_checks.refused, [0, 1, 1, 0]);
    assert_eq!(row.budget_checks.clock_reads, [1, 1, 1, 1]);
    assert!(row.budget_checks.ns[..2].iter().sum::<u64>() <= row.source_ns.iter().sum::<u64>());
    assert!(row.budget_checks.ns.iter().sum::<u64>() <= row.trace_ns);
    assert_eq!(row.trace_ns, row.regions_ns.iter().sum::<u64>());
    assert!(
        report.raw_row_bytes <= crate::diagnostic::MAX_RAW_ROW_BYTES
            && report.raw_storage_bound_bytes
                <= crate::diagnostic::MAX_RAW_ROW_BYTES * crate::diagnostic::MAX_FRAMES
    );
    assert_eq!(
        serde_json::to_value(row).unwrap()["budget_checks"]["calls"],
        serde_json::json!([1, 2, 1, 1])
    );
}
#[test]
fn missing_duplicate_nested_and_unclosed_budget_boundaries_refuse_coverage() {
    for case in 0..6 {
        let report = capture(
            || {},
            || match case {
                0 => Span::start(Origin::Explicit).finish(true), // accepted without read
                1 => {
                    let span = Span::start(Origin::Explicit);
                    clock_read();
                    clock_read();
                    span.finish(true);
                }
                2 => {
                    let _outer = Span::start(Origin::Explicit);
                    Span::start(Origin::Interrupt).finish(false);
                }
                3 => std::mem::forget(Span::start(Origin::Explicit)),
                4 => {
                    let span = Span::start(Origin::Explicit);
                    mark(Major::RevokeChecks);
                    span.finish(false);
                }
                _ => drop(Span::start(Origin::Explicit)), // no acceptance outcome
            },
        );
        assert!(!report.complete && !report.rows[0].valid, "case {case}");
    }
}
#[test]
fn check_clock_refusal_and_duration_overflow_invalidate_without_wrapping() {
    for case in 0..4 {
        let report = capture(
            || {},
            || {
                ACTIVE.with_borrow_mut(|a| {
                    let c = &mut a.as_mut().unwrap().row.budget_checks;
                    match case {
                        0 => c.calls[1] = u32::MAX,
                        1 => c.clock_reads[1] = u32::MAX,
                        2 => c.refused[1] = u32::MAX,
                        _ => c.ns[1] = u64::MAX,
                    }
                });
                let mut span = Span::start(Origin::Explicit);
                if case == 3 {
                    // Guarantee nonzero elapsed time without sleeping or a
                    // timing threshold: this is a test-only span boundary.
                    span.token.as_mut().unwrap().3 = Instant::now()
                        .checked_sub(std::time::Duration::from_secs(1))
                        .unwrap();
                }
                clock_read();
                span.finish(case != 2);
            },
        );
        assert!(!report.complete);
        let c = &report.rows[0].budget_checks;
        match case {
            0 => assert_eq!(c.calls[1], u32::MAX),
            1 => assert_eq!(c.clock_reads[1], u32::MAX),
            2 => assert_eq!(c.refused[1], u32::MAX),
            _ => assert_eq!(c.ns[1], u64::MAX),
        }
    }
}
#[test]
fn inactive_checks_are_ignored_but_foreign_thread_activity_refuses_capture() {
    accepted(Origin::Explicit); // outside capture and trace
    let report = capture(|| {}, || {});
    assert!(report.complete);
    assert_eq!(report.rows[0].budget_checks.calls, [0; 4]);
    let report = capture(
        || {},
        || {
            std::thread::spawn(|| Span::start(Origin::Explicit).finish(false))
                .join()
                .unwrap();
        },
    );
    assert!(!report.complete);
    assert_eq!(report.rows[0].budget_checks.calls, [0; 4]);
}
#[test]
fn stale_budget_guard_cannot_enter_successor_and_unwind_remains_visible() {
    let old_capture = crate::Capture::start().unwrap();
    let old_frame = Frame::start();
    let old_span = Span::start(Origin::Explicit);
    drop(old_capture);
    let report = capture(
        || {
            drop(old_span);
            drop(old_frame);
        },
        || accepted(Origin::Explicit),
    );
    assert!(report.complete);
    assert_eq!(report.rows[0].budget_checks.calls, [0, 1, 0, 0]);

    let capture = crate::Capture::start().unwrap();
    let failed = std::panic::catch_unwind(|| {
        let _: Result<(), ()> = crate::frame(|| {
            tick(
                || {},
                || {
                    let _check = Span::start(Origin::Explicit);
                    clock_read();
                    panic!("deliberate budget-check unwind");
                },
            );
            Ok(())
        });
    });
    assert!(failed.is_err());
    let report = capture.report().script_phase_diagnostic;
    assert!(!report.complete && report.rows[0].unwound);
    assert_eq!(report.rows[0].budget_checks.refused[1], 1);
}
