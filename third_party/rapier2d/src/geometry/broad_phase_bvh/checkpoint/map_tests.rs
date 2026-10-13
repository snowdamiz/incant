//! Actual allocation requests for both selected native map implementations.
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::eprintln;
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
fn measure<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    COUNT.with(|c| c.set((true, 0, 0)));
    let stop = Stop;
    let value = black_box(f());
    drop(stop);
    let (_, b, n) = COUNT.with(Cell::get);
    (value, b, n)
}
fn rows(n: usize) -> Vec<incant_physics_snapshot::BroadPair> {
    (0..n as u32)
        .map(|i| incant_physics_snapshot::BroadPair {
            colliders: [
                Handle {
                    slot: i * 2,
                    generation: 0,
                },
                Handle {
                    slot: i * 2 + 1,
                    generation: 0,
                },
            ],
            inserted_frame: i,
        })
        .collect()
}
#[test]
fn checked_broad_map_plan_bounds_actual_native_allocations_without_growth() {
    for n in [0, 1, 3, 4, 7, 8, 14, 15, 28, 29, 127, 128, 4096] {
        let rows = rows(n);
        let plan = map::plan(n).unwrap();
        let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
            .admit(plan)
            .unwrap();
        let (pairs, bytes, calls) = measure(|| map::construct(&rows, &mut permit).unwrap());
        permit.finish().unwrap();
        assert_eq!(pairs.len(), n);
        assert!(bytes <= plan.bytes());
        assert_eq!(calls, plan.allocations());
        eprintln!(
            "CHECKED_BROAD_MAP dimension={} deterministic={} entries={} actual_bytes={} charged_bytes={} calls={}",
            crate::math::DIM,
            cfg!(feature = "enhanced-determinism"),
            n,
            bytes,
            plan.bytes(),
            calls
        );
    }
}
#[test]
fn checked_broad_map_budget_overflow_duplicate_and_canonical_order_controls() {
    assert!(map::plan(usize::MAX).is_err());
    let mut rows = rows(17);
    let plan = map::plan(rows.len()).unwrap();
    assert!(
        AllocationBudget::new(plan.bytes() - 1, plan.allocations())
            .admit(plan)
            .is_err()
    );
    assert!(
        AllocationBudget::new(plan.bytes(), plan.allocations() - 1)
            .admit(plan)
            .is_err()
    );
    let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let a = map::construct(&rows, &mut permit).unwrap();
    permit.finish().unwrap();
    rows.reverse();
    let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let b = map::construct(&rows, &mut permit).unwrap();
    permit.finish().unwrap();
    // Native serialized identity is canonical pairs/stamps, not insertion history.
    let sorted = |m: &map::Pairs| {
        let mut entries: Vec<_> = m.iter().map(|(k, v)| (*k, *v)).collect();
        entries.sort_by_key(|((a, b), _)| (a.into_raw_parts(), b.into_raw_parts()));
        entries
    };
    assert_eq!(sorted(&a), sorted(&b));
    rows[1] = rows[0];
    let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    assert!(matches!(
        map::construct(&rows, &mut permit),
        Err(ConstructionError::Payload)
    ));
    permit.finish().unwrap();
}
