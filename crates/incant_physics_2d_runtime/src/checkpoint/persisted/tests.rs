use super::*;
use incant_physics_snapshot::MAX_ALLOCATION;
use incant_types::RigidBody;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn work() -> WorkBudget {
    WorkBudget::new(100_000_000)
}
fn allocation() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 100_000)
}
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn fixture() -> PhysicsWorld2D {
    let mut world = PhysicsWorld2D::default();
    let body = |dynamic, sensor, y| {
        PreparedBody::new(
            dynamic.then_some(RigidBody {
                ccd: false,
                ..Default::default()
            }),
            Collider2D {
                shape: ColliderShape2D::Box {
                    half_extents: if dynamic { [0.4; 2] } else { [4., 0.5] },
                },
                sensor,
                ..Default::default()
            },
            BodyState {
                translation: [0., y],
                ..Default::default()
            },
        )
        .unwrap()
    };
    world
        .apply(vec![
            (id(1), Some(body(false, false, -0.5))),
            (id(2), Some(body(true, false, 1.))),
            (id(3), Some(body(false, true, 0.5))),
        ])
        .unwrap();
    for _ in 0..64 {
        world.step(1. / 64.).unwrap();
    }
    world
}
fn save(world: &mut PhysicsWorld2D) -> Vec<u8> {
    let mut b = budget();
    let mut w = work();
    world
        .checkpoint(&mut b, &mut w)
        .unwrap()
        .persisted_bytes(&mut b, &mut w)
        .unwrap()
}
#[test]
fn persisted_planar_fresh_candidate_preserves_warm_state_and_event_history() {
    let mut source = fixture();
    let bytes = save(&mut source);
    let mut other =
        PhysicsWorld2D::from_persisted_bytes(&bytes, &mut budget(), &mut work(), &mut allocation())
            .unwrap();
    assert_eq!(save(&mut other), bytes);
    for _ in 0..72 {
        source.step(1. / 64.).unwrap();
        other.step(1. / 64.).unwrap();
        assert_eq!(source.events(), other.events());
        assert_eq!(source.state(id(2)), other.state(id(2)));
        assert_eq!(save(&mut source), save(&mut other));
    }
}
#[test]
fn persisted_planar_generation_framing_and_allocation_refuse_without_source_change() {
    let mut source = fixture();
    let bytes = save(&mut source);
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let first = 12 + n + 4;
    assert_eq!(bytes[first + 16], 0);
    let mut forged = bytes.clone();
    let at = first + 21;
    forged[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(
        PhysicsWorld2D::from_persisted_bytes(
            &forged,
            &mut budget(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsWorld2D::from_persisted_bytes(
            &bytes,
            &mut Budget::new(0).unwrap(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsWorld2D::from_persisted_bytes(
            &bytes,
            &mut budget(),
            &mut WorkBudget::new(0),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsWorld2D::from_persisted_bytes(
            &bytes,
            &mut budget(),
            &mut work(),
            &mut AllocationBudget::new(0, 0)
        )
        .is_err()
    );
    for n in [0, 7, 12, bytes.len() - 1] {
        assert!(
            PhysicsWorld2D::from_persisted_bytes(
                &bytes[..n],
                &mut budget(),
                &mut work(),
                &mut allocation()
            )
            .is_err()
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(
        PhysicsWorld2D::from_persisted_bytes(&extra, &mut budget(), &mut work(), &mut allocation())
            .is_err()
    );
    assert_eq!(save(&mut source), bytes);
}
