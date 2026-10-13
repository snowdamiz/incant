// Test-only System delegation shared by actual native allocation controls.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
std::thread_local! { static COUNT: Cell<(bool,usize,usize)> = const { Cell::new((false,0,0)) }; }
struct Measured;
#[global_allocator]
static ALLOCATOR: Measured = Measured;
fn add(n: usize) {
    let _ = COUNT.try_with(|c| {
        let (on, b, k) = c.get();
        if on {
            c.set((on, b.checked_add(n).unwrap(), k.checked_add(1).unwrap()));
        }
    });
}
// SAFETY: forwarding preserves every System allocation/deallocation contract;
// thread-local integer observation neither allocates nor changes any pointer.
unsafe impl GlobalAlloc for Measured {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        add(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        add(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        add(n);
        unsafe { System.realloc(p, l, n) }
    }
}
struct Stop;
impl Drop for Stop {
    fn drop(&mut self) {
        COUNT.with(|c| {
            let (_, b, n) = c.get();
            c.set((false, b, n));
        });
    }
}
pub(crate) fn measure<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    COUNT.with(|c| c.set((true, 0, 0)));
    let stop = Stop;
    let value = black_box(f());
    drop(stop);
    let (_, b, n) = COUNT.with(Cell::get);
    (value, b, n)
}
