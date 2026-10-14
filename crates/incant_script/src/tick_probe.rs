//! Test-only timestamp boundaries. Never built into a native library/player.
use std::{cell::RefCell, time::Instant};

#[derive(Clone, Copy)]
pub(super) enum Region {
    Enter,
    InitialChecks,
    Snapshots,
    GroupSetup,
    Capabilities,
    UndoCapture,
    ViewExposure,
    Callbacks,
    RevokeChecks,
    NumericValidation,
    TransactionStage,
    ScopeCleanup,
    Publication,
    HeapTelemetry,
}
const NAMES: [&str; 14] = [
    "enter",
    "initial_checks",
    "snapshots",
    "group_setup",
    "capabilities",
    "undo_capture",
    "view_exposure",
    "callbacks",
    "revoke_checks",
    "numeric_validation",
    "transaction_stage",
    "scope_cleanup",
    "publication",
    "heap_telemetry",
];
struct Frame {
    start: Instant,
    previous: Instant,
    intervals: [u64; NAMES.len()],
    next: usize,
}
thread_local! {
    static ACTIVE: RefCell<Option<Frame>> = const { RefCell::new(None) };
}

pub(super) fn mark(region: Region) {
    ACTIVE.with_borrow_mut(|slot| {
        if let Some(frame) = slot {
            let index = region as usize;
            assert_eq!(frame.next, index, "diagnostic boundaries out of order");
            let now = Instant::now();
            frame.intervals[index] = u64::try_from(now.duration_since(frame.previous).as_nanos())
                .expect("bounded diagnostic duration");
            frame.previous = now;
            frame.next += 1;
        }
    });
}

struct Trace;
impl Trace {
    fn start() -> Self {
        ACTIVE.with_borrow_mut(|slot| {
            assert!(slot.is_none(), "nested diagnostic frame");
            let now = Instant::now();
            *slot = Some(Frame {
                start: now,
                previous: now,
                intervals: [0; NAMES.len()],
                next: 0,
            });
        });
        Self
    }
    fn finish(self) -> ([u64; NAMES.len()], u64) {
        ACTIVE.with_borrow_mut(|slot| {
            let frame = slot.take().expect("active diagnostic frame");
            assert_eq!(frame.next, NAMES.len(), "incomplete diagnostic frame");
            let outer = u64::try_from(frame.start.elapsed().as_nanos()).unwrap();
            assert!(outer >= frame.intervals.iter().sum());
            (frame.intervals, outer)
        })
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        ACTIVE.with_borrow_mut(|slot| *slot = None);
    }
}

#[cfg(feature = "compiler")]
mod tests;
