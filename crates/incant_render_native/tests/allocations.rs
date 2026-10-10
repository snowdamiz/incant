//! Isolated allocator instrumentation: only the adapter call is measured, not ECS
//! stepping, initialization, test harness threads or the caller's work.
mod common;
use common::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct Count;
thread_local! {static ENABLED:Cell<bool>=const{Cell::new(false)};static ALLOCS:Cell<usize>=const{Cell::new(0)};}
fn allocation() {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }
}
// SAFETY: this wrapper forwards all pointers, layouts and sizes unchanged to the
// System allocator. TLS counters hold no allocation and never inspect memory.
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
struct Recording;
impl Drop for Recording {
    fn drop(&mut self) {
        ENABLED.set(false);
    }
}
#[test]
fn ordinary_committed_motion_and_callback_rejection_allocate_nothing() {
    let mut world = world();
    let mut input = load(&world);
    for _ in 0..64 {
        world.step().unwrap();
        let view = world.spatial_view().unwrap();
        ALLOCS.set(0);
        ENABLED.set(true);
        let recording = Recording;
        let rejected = input.try_sync(&view, |_, _, _| Err::<(), _>(()));
        let accepted = input.try_sync(&view, |_, _, _| Ok::<_, ()>(()));
        drop(recording);
        assert!(rejected.is_err());
        assert!(accepted.is_ok());
        assert_eq!(ALLOCS.get(), 0);
    }
}
