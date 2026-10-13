//! Actual allocation requests for both selected native map implementations.
use super::*;
use crate::pipeline::checked_restore::allocation_test_meter::measure;
use std::eprintln;
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
