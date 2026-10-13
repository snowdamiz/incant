//! Dedicated process: the ordinary parallel test harness itself allocates on
//! other threads, which would invalidate exact process-wide counter assertions.
use incant_profile::*;
use std::alloc::{GlobalAlloc, Layout, System};
#[global_allocator]
static ALLOCATOR: CountingAllocator<System> = CountingAllocator(System);
struct Refuses;
// SAFETY: this test allocator never returns a non-null allocation. Its only
// successful operation is freeing a valid System allocation provided below.
unsafe impl GlobalAlloc for Refuses {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        std::ptr::null_mut()
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, _: *mut u8, _: Layout, _: usize) -> *mut u8 {
        std::ptr::null_mut()
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
    let wrapper = CountingAllocator(Refuses);
    let layout = Layout::from_size_align(64, 8).unwrap();
    // SAFETY: layout is valid/nonzero. Null means no ownership was acquired.
    let before = capture.allocations();
    assert!(unsafe { wrapper.alloc(layout) }.is_null());
    let after = capture.allocations();
    assert_eq!(after.failures, before.failures + 1);
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    // Allocate through the real installed allocator, then force a realloc failure:
    // the original allocation and its accounting must survive unchanged.
    let p = unsafe { ALLOCATOR.alloc(layout) };
    assert!(!p.is_null());
    unsafe { p.write(7) };
    let before = capture.allocations();
    assert!(unsafe { wrapper.realloc(p, layout, 128) }.is_null());
    let after = capture.allocations();
    assert_eq!(after.failures, before.failures + 1);
    assert_eq!(after.live_requested_bytes, before.live_requested_bytes);
    assert_eq!(unsafe { p.read() }, 7);
    unsafe { ALLOCATOR.dealloc(p, layout) };
    println!(
        "allocation controls: hot scopes no allocation; exact requested/live/free; null alloc and realloc preserve accounting"
    );
}
