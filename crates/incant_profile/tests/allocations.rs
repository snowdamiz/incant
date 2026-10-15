//! Dedicated process: the ordinary parallel test harness itself allocates on
//! other threads, which would invalidate exact process-wide counter assertions.
use incant_profile::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
#[global_allocator]
static ALLOCATOR: CountingAllocator<System> = CountingAllocator(System);
struct Refuses(AtomicBool);
// SAFETY: accepted operations delegate unchanged to System. Forced failure
// returns null without consuming a live allocation. Every pointer below is
// allocated, reallocated and freed through this same wrapper instance.
unsafe impl GlobalAlloc for Refuses {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if self.0.load(Relaxed) {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(l) }
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if self.0.load(Relaxed) {
            std::ptr::null_mut()
        } else {
            unsafe { System.realloc(p, l, n) }
        }
    }
}
fn main() {
    let capture = Capture::start().unwrap();
    {
        let _zone = script_scope(Some([0; 16]));
    }
    let before = capture.allocations();
    for _ in 0..1000 {
        let _zone = script_scope(Some([0; 16]));
        let _system = scope(Phase::Integrate);
    }
    for _ in 0..1000 {
        frame(|| {
            let _zone = scope(Phase::ScriptTick);
            let _source = script_scope(Some([0; 16]));
            Ok::<_, ()>(())
        })
        .unwrap();
    }
    let after = capture.allocations();
    assert_eq!(after.allocations, before.allocations);
    assert_eq!(after.reallocations, before.reallocations);
    let before = capture.allocations();
    let bytes = std::hint::black_box(vec![1_u8; 4096]);
    let live = capture.allocations();
    assert_eq!(
        live.live_requested_bytes - before.live_requested_bytes,
        4096
    );
    assert!(live.process_peak_requested_bytes >= live.live_requested_bytes);
    drop(bytes);
    let after = capture.allocations();
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    assert_eq!(
        after.allocated_requested_bytes - before.allocated_requested_bytes,
        4096
    );
    assert_eq!(
        after.freed_requested_bytes - before.freed_requested_bytes,
        4096
    );
    let wrapper = CountingAllocator(Refuses(AtomicBool::new(true)));
    let layout = Layout::from_size_align(64, 8).unwrap();
    // SAFETY: layout is valid/nonzero. Null means no ownership was acquired.
    let before = capture.allocations();
    assert!(unsafe { wrapper.alloc(layout) }.is_null());
    let after = capture.allocations();
    assert_eq!(after.failures, before.failures + 1);
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    // Allocate through this same allocator, then force a realloc failure: the
    // original allocation and its accounting must survive unchanged.
    wrapper.0.0.store(false, Relaxed);
    let p = unsafe { wrapper.alloc(layout) };
    assert!(!p.is_null());
    unsafe { p.write(7) };
    wrapper.0.0.store(true, Relaxed);
    let before = capture.allocations();
    assert!(unsafe { wrapper.realloc(p, layout, 128) }.is_null());
    let after = capture.allocations();
    assert_eq!(after.failures, before.failures + 1);
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    assert_eq!(unsafe { p.read() }, 7);
    unsafe { wrapper.dealloc(p, layout) };
    wrapper.0.0.store(false, Relaxed);
    let before = capture.allocations();
    let p = unsafe { wrapper.alloc_zeroed(layout) };
    assert!(!p.is_null());
    assert_eq!(p as usize % layout.align(), 0);
    assert!(
        unsafe { std::slice::from_raw_parts(p, 64) }
            .iter()
            .all(|b| *b == 0)
    );
    unsafe { p.write(9) };
    let p = unsafe { wrapper.realloc(p, layout, 128) };
    assert!(!p.is_null());
    assert_eq!(unsafe { p.read() }, 9);
    assert_eq!(
        capture.allocations().live_requested_bytes,
        before.live_requested_bytes + 128
    );
    let larger = Layout::from_size_align(128, 8).unwrap();
    let p = unsafe { wrapper.realloc(p, larger, 32) };
    assert!(!p.is_null());
    assert_eq!(unsafe { p.read() }, 9);
    let smaller = Layout::from_size_align(32, 8).unwrap();
    unsafe { wrapper.dealloc(p, smaller) };
    let after = capture.allocations();
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    assert_eq!(after.reallocations, before.reallocations + 2);
    assert_eq!(
        after.allocated_requested_bytes,
        before.allocated_requested_bytes + 224
    );
    assert_eq!(
        after.freed_requested_bytes,
        before.freed_requested_bytes + 224
    );
    println!(
        "allocation controls: hot scopes no allocation; exact requested/live/free; null allocation/reallocation; zeroed/grow/shrink preserve ownership and contents"
    );
    #[cfg(feature = "diagnostic-script-budget")]
    {
        drop(capture);
        budget_allocation_controls();
    }
}

#[cfg(feature = "diagnostic-script-budget")]
fn budget_allocation_controls() {
    use diagnostic::{Frame, Region, Span, budget, mark};
    let capture = Capture::start().unwrap();
    let before = capture.allocations();
    for _ in 0..1000 {
        frame(|| {
            let mut trace = Frame::start();
            let _tick = scope(Phase::ScriptTick);
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
                let _source = script_scope(Some([1; 16]));
                let _callback = Span::callback(Some([1; 16]));
                for _ in 0..32 {
                    for origin in [budget::Origin::Interrupt, budget::Origin::Explicit] {
                        let check = budget::Span::start(origin);
                        budget::clock_read();
                        check.finish(true);
                    }
                }
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
            Ok::<_, ()>(())
        })
        .unwrap();
    }
    let after = capture.allocations();
    assert_eq!(after.allocations, before.allocations);
    assert_eq!(after.reallocations, before.reallocations);
    let report = capture.report().script_phase_diagnostic;
    assert!(report.complete && report.rows.len() == 1000);
    for row in report.rows {
        assert_eq!(row.budget_checks.calls, [32, 32, 0, 0]);
        assert_eq!(row.budget_checks.clock_reads, [32, 32, 0, 0]);
        assert_eq!(row.budget_checks.refused, [0; 4]);
    }
    println!(
        "budget attribution: 64,000 check/clock annotations in 1,000 complete frames allocate/reallocate nothing"
    );
}
