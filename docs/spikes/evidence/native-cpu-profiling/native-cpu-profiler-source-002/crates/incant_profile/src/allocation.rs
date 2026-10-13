//! Counts requested Rust allocator sizes, not usable sizes, C heaps, GPU memory or RSS.
//! No pointer table, locks, callback, formatting or allocation inside allocator hooks.
use crate::Allocations;
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
static INSTALLED: AtomicBool = AtomicBool::new(false);
static SATURATED: AtomicBool = AtomicBool::new(false);
static ALLOC: AtomicU64 = AtomicU64::new(0);
static REALLOC: AtomicU64 = AtomicU64::new(0);
static FREE: AtomicU64 = AtomicU64::new(0);
static FAIL: AtomicU64 = AtomicU64::new(0);
static IN: AtomicU64 = AtomicU64::new(0);
static OUT: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
fn add(counter: &AtomicU64, n: u64) -> u64 {
    counter
        .try_update(Relaxed, Relaxed, |old| {
            if old.checked_add(n).is_none() {
                SATURATED.store(true, Relaxed);
            }
            Some(old.saturating_add(n))
        })
        .expect("update always returns Some")
        .saturating_add(n)
}
fn allocated(n: usize) {
    add(&IN, n as u64);
    let live = add(&LIVE, n as u64);
    PEAK.fetch_max(live, Relaxed);
}
fn freed(n: usize) {
    add(&OUT, n as u64);
    let _ = LIVE.try_update(Relaxed, Relaxed, |v| Some(v.saturating_sub(n as u64)));
}
/// Install once at the executable boundary, wrapping its existing allocator.
/// The generic wrapper preserves layout, pointer ownership, null and realloc semantics.
pub struct CountingAllocator<A>(pub A);
// SAFETY: all allocation operations are delegated with their exact caller arguments;
// metadata uses fixed atomics only and never dereferences or retains user pointers.
// The caller retains the GlobalAlloc obligations for pointer/layout validity.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        INSTALLED.store(true, Relaxed);
        add(&ALLOC, 1);
        // SAFETY: unchanged GlobalAlloc caller preconditions are forwarded to A.
        let p = unsafe { self.0.alloc(layout) };
        if p.is_null() {
            add(&FAIL, 1);
        } else {
            allocated(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        INSTALLED.store(true, Relaxed);
        add(&ALLOC, 1);
        // SAFETY: unchanged GlobalAlloc caller preconditions are forwarded to A.
        let p = unsafe { self.0.alloc_zeroed(layout) };
        if p.is_null() {
            add(&FAIL, 1);
        } else {
            allocated(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        add(&FREE, 1);
        freed(layout.size());
        // SAFETY: unchanged pointer and matching layout are delegated once.
        unsafe { self.0.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        add(&REALLOC, 1);
        // SAFETY: unchanged live pointer, old layout and nonzero size go to A.
        let next = unsafe { self.0.realloc(p, layout, size) };
        if next.is_null() {
            add(&FAIL, 1);
        } else {
            freed(layout.size());
            allocated(size);
        }
        next
    }
}
pub(crate) fn snapshot() -> Allocations {
    Allocations {
        installed: INSTALLED.load(Relaxed),
        allocations: ALLOC.load(Relaxed),
        reallocations: REALLOC.load(Relaxed),
        frees: FREE.load(Relaxed),
        failures: FAIL.load(Relaxed),
        allocated_requested_bytes: IN.load(Relaxed),
        freed_requested_bytes: OUT.load(Relaxed),
        live_requested_bytes: LIVE.load(Relaxed),
        process_peak_requested_bytes: PEAK.load(Relaxed),
        saturated: SATURATED.load(Relaxed),
    }
}
pub(crate) fn since(b: Allocations) -> Allocations {
    let mut n = snapshot();
    n.allocations = n.allocations.saturating_sub(b.allocations);
    n.reallocations = n.reallocations.saturating_sub(b.reallocations);
    n.frees = n.frees.saturating_sub(b.frees);
    n.failures = n.failures.saturating_sub(b.failures);
    n.allocated_requested_bytes = n
        .allocated_requested_bytes
        .saturating_sub(b.allocated_requested_bytes);
    n.freed_requested_bytes = n
        .freed_requested_bytes
        .saturating_sub(b.freed_requested_bytes);
    n
}
