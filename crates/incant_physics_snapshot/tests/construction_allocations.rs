//! Exact allocator request sizes for the pinned primitive/Box construction path.
//! Logical requests exclude allocator bookkeeping, committed pages and RSS.
#![cfg(feature = "native-admission")]
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
thread_local! {
    static COUNT: Cell<(bool,usize,usize)> = const { Cell::new((false,0,0)) };
}
struct Measured;
#[global_allocator]
static ALLOCATOR: Measured = Measured;
fn add(bytes: usize) {
    let _ = COUNT.try_with(|c| {
        let (on, b, n) = c.get();
        if on {
            c.set((on, b.checked_add(bytes).unwrap(), n.checked_add(1).unwrap()));
        }
    });
}
// SAFETY: every operation forwards its unchanged contract to System; observation
// uses only allocation-free thread-local integers and never changes pointers.
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
fn measure<T>(create: impl FnOnce() -> T) -> (T, usize, usize) {
    COUNT.with(|c| c.set((true, 0, 0)));
    let stop = Stop;
    let value = black_box(create());
    drop(stop);
    let (_, bytes, n) = COUNT.with(Cell::get);
    (value, bytes, n)
}
fn shared_size<T>() -> usize {
    Layout::array::<usize>(2)
        .unwrap()
        .extend(Layout::new::<T>())
        .unwrap()
        .0
        .pad_to_align()
        .size()
}
macro_rules! native {
    ($rapier:ident,$name:ident) => {
        #[test]
        fn $name() {
            use $rapier::math::Vector;
            use $rapier::parry::shape::{Ball, Capsule, Cuboid, SharedShape};
            for (size, make) in [
                (shared_size::<Ball>(), 0),
                (shared_size::<Cuboid>(), 1),
                (shared_size::<Capsule>(), 2),
            ] {
                let (s, bytes, calls) = measure(|| match make {
                    0 => SharedShape::new(Ball::new(0.5)),
                    1 => SharedShape::new(Cuboid::new(Vector::splat(0.5))),
                    _ => SharedShape::new(Capsule::new_y(0.3, 0.5)),
                });
                assert_eq!((bytes, calls), (size, 1));
                black_box(s);
                eprintln!(
                    "CONSTRUCTION_ALLOCATION dimension={} primitive={} request_bytes={} calls=1",
                    $rapier::math::DIM,
                    make,
                    bytes
                );
            }
            let (v, bytes, calls) =
                measure(|| Box::new($rapier::dynamics::RigidBodyAdditionalMassProps::Mass(2.)));
            assert_eq!(
                (bytes, calls),
                (
                    size_of::<$rapier::dynamics::RigidBodyAdditionalMassProps>(),
                    1
                )
            );
            black_box(v);
            let (v, bytes, calls) =
                measure(|| Box::new($rapier::dynamics::MassProperties::default()));
            assert_eq!(
                (bytes, calls),
                (size_of::<$rapier::dynamics::MassProperties>(), 1)
            );
            black_box(v);
        }
    };
}
native!(
    rapier2d,
    checked_allocation_plan_matches_actual_planar_primitive_requests
);
native!(
    rapier3d,
    checked_allocation_plan_matches_actual_spatial_primitive_requests
);
