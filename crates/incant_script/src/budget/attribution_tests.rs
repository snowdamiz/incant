//! Exercise the actual check/clock/latch, not a replacement clock or budget.
use super::*;
use crate::diagnostics::check_deadline;
use incant_profile::diagnostic::{Frame, Region, Span, mark};
use std::sync::Mutex;

fn observe(body: impl FnOnce()) -> incant_profile::diagnostic::Report {
    let capture = incant_profile::Capture::start().unwrap();
    incant_profile::frame(|| {
        let mut frame = Frame::start();
        let _tick = incant_profile::scope(incant_profile::Phase::ScriptTick);
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
            let _source = incant_profile::script_scope(None);
            let _callback = Span::callback(None);
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
        frame.finish(true);
        Ok::<_, ()>(())
    })
    .unwrap();
    capture.report().script_phase_diagnostic
}
#[test]
fn actual_clock_attempts_and_latched_unavailable_refusal_are_counted_exactly() {
    let budget = Mutex::new(ExecutionBudget::new(Duration::from_secs(1)).unwrap());
    let report = observe(|| {
        check_deadline(&budget).unwrap();
        budget.lock().unwrap().failure = Some(Failure::Unavailable);
        for _ in 0..2 {
            assert!(matches!(
                check_deadline(&budget),
                Err(ScriptError::CpuClock {
                    reason: "unavailable"
                })
            ));
        }
        budget.lock().unwrap().reset().unwrap(); // reset read is not a check
        check_deadline(&budget).unwrap();
    });
    assert!(report.complete);
    let counters = &report.rows[0].budget_checks;
    assert_eq!(counters.calls, [0, 4, 0, 0]);
    assert_eq!(counters.refused, [0, 2, 0, 0]);
    assert_eq!(counters.clock_reads, [0, 2, 0, 0]);
}
#[test]
fn reversed_clock_and_expiration_remain_latched_fail_closed() {
    let budget = Mutex::new(ExecutionBudget::new(Duration::from_secs(1)).unwrap());
    let report = observe(|| {
        budget.lock().unwrap().start = now().unwrap() + Duration::from_secs(5);
        for _ in 0..2 {
            assert!(matches!(
                check_deadline(&budget),
                Err(ScriptError::CpuClock {
                    reason: "moved backwards"
                })
            ));
        }
        {
            let mut budget = budget.lock().unwrap();
            budget.reset().unwrap();
            budget.start = Duration::ZERO;
            budget.limit = Duration::ZERO; // exact >= limit refusal, test-only injection
        }
        for _ in 0..2 {
            assert!(matches!(
                check_deadline(&budget),
                Err(ScriptError::ExecutionDeadline { budget_ms: 0 })
            ));
        }
    });
    assert!(report.complete);
    let counters = &report.rows[0].budget_checks;
    assert_eq!(counters.calls, [0, 4, 0, 0]);
    assert_eq!(counters.refused, [0, 4, 0, 0]);
    assert_eq!(counters.clock_reads, [0, 2, 0, 0]);
}
#[test]
fn changed_budget_owner_refuses_before_reading_that_threads_cpu_clock() {
    let budget = Mutex::new(ExecutionBudget::new(Duration::from_secs(1)).unwrap());
    let report = std::thread::spawn(move || {
        observe(|| {
            for _ in 0..2 {
                assert!(matches!(
                    check_deadline(&budget),
                    Err(ScriptError::CpuClock {
                        reason: "execution moved to a different thread"
                    })
                ));
            }
        })
    })
    .join()
    .unwrap();
    assert!(report.complete); // complete attribution of two actual refused checks
    let counters = &report.rows[0].budget_checks;
    assert_eq!(counters.calls, [0, 2, 0, 0]);
    assert_eq!(counters.refused, [0, 2, 0, 0]);
    assert_eq!(counters.clock_reads, [0; 4]);
}
