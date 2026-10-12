// Test-only thread-local instrumentation; no production unsafe code or allocator.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static COUNT:Cell<Option<(usize,usize)>>=const {Cell::new(None)};}
pub struct CountAlloc;
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|n| {
            if let Some(v) = n.get() {
                n.set(Some((v.0 + 1, v.1 + layout.size())));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(v) = c.get() {
                c.set(Some((v.0 + 1, v.1 + n)));
            }
        });
        unsafe { System.realloc(p, layout, n) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|n| {
            if let Some(v) = n.get() {
                n.set(Some((v.0 + 1, v.1 + layout.size())));
            }
        });
        unsafe { System.alloc_zeroed(layout) }
    }
}
pub fn measured<T>(f: impl FnOnce() -> T) -> (T, (usize, usize)) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNT.with(|n| n.set(None));
        }
    }
    COUNT.with(|n| {
        assert!(n.get().is_none());
        n.set(Some((0, 0)));
    });
    let reset = Reset;
    let value = f();
    let count = COUNT.with(|n| n.get().unwrap());
    drop(reset);
    (value, count)
}
