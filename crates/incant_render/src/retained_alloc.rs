//! Test-only, thread-local counter for CPU pose staging, excluding GPU work.
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
pub(super) fn measured(f: impl FnOnce()) -> usize {
    ALLOCS.set(0);
    ENABLED.set(true);
    let recording = Recording;
    f();
    drop(recording);
    ALLOCS.get()
}
