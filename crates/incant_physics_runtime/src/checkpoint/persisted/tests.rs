use super::*;
use incant_physics_snapshot::MAX_ALLOCATION;
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
fn fixture() -> PhysicsRuntime<StableId> {
    let mut world = PhysicsRuntime::default();
    let state = |y| BodyState {
        translation: [0., y, 0.],
        rotation: [0., 0., 0., 1.],
        velocity: [0.; 3],
        angular_velocity: [0.; 3],
        sleeping: false,
    };
    let declarations = [id(1), id(2)]
        .into_iter()
        .map(|scene| {
            (
                scene,
                BTreeMap::from([
                    (
                        id(10),
                        PhysicsSpec {
                            body: None,
                            collider: DocCollider {
                                shape: ColliderShape::Box {
                                    half_extents: [4., 0.5, 4.],
                                },
                                ..Default::default()
                            },
                            state: state(-0.5),
                        },
                    ),
                    (
                        id(11),
                        PhysicsSpec {
                            body: Some(DocBody {
                                ccd: false,
                                ..Default::default()
                            }),
                            collider: DocCollider {
                                shape: ColliderShape::Sphere { radius: 0.4 },
                                ..Default::default()
                            },
                            state: state(1.),
                        },
                    ),
                    (
                        id(12),
                        PhysicsSpec {
                            body: None,
                            collider: DocCollider {
                                shape: ColliderShape::Sphere { radius: 1. },
                                sensor: true,
                                ..Default::default()
                            },
                            state: state(0.5),
                        },
                    ),
                ]),
            )
        })
        .collect();
    world
        .sync(PreparedPhysics::new(declarations).unwrap())
        .unwrap();
    for _ in 0..64 {
        world.step(1. / 64.).unwrap();
    }
    world
}
fn save(world: &mut PhysicsRuntime<StableId>) -> Vec<u8> {
    let mut b = budget();
    let mut w = work();
    world
        .checkpoint(&mut b, &mut w)
        .unwrap()
        .persisted_bytes(&mut b, &mut w)
        .unwrap()
}
#[test]
fn persisted_spatial_fresh_candidate_preserves_warm_state_events_and_continuation() {
    let mut source = fixture();
    let bytes = save(&mut source);
    let mut other =
        PhysicsRuntime::from_persisted_bytes(&bytes, &mut budget(), &mut work(), &mut allocation())
            .unwrap();
    // Imported origin can re-save before any step; it must not fabricate a step.
    assert_eq!(save(&mut other), bytes);
    for _ in 0..72 {
        source.step(1. / 64.).unwrap();
        other.step(1. / 64.).unwrap();
        assert_eq!(source.events(), other.events());
        for scene in [id(1), id(2)] {
            assert_eq!(source.state(&scene, &id(11)), other.state(&scene, &id(11)));
        }
        assert_eq!(save(&mut source), save(&mut other));
    }
}
#[test]
fn persisted_spatial_owner_generation_framing_and_shared_budget_refuse() {
    let mut source = fixture();
    let bytes = save(&mut source);
    let native_len = u32::from_le_bytes(bytes[28..32].try_into().unwrap()) as usize;
    let first_entry = 32 + native_len + 4;
    // First entry is a body-less floor; change only outer collider generation.
    assert_eq!(bytes[first_entry + 16], 0);
    let mut forged = bytes.clone();
    let at = first_entry + 17 + 4;
    forged[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(
        PhysicsRuntime::from_persisted_bytes(
            &forged,
            &mut budget(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsRuntime::from_persisted_bytes(
            &bytes,
            &mut Budget::new(0).unwrap(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsRuntime::from_persisted_bytes(
            &bytes,
            &mut budget(),
            &mut WorkBudget::new(0),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        PhysicsRuntime::from_persisted_bytes(
            &bytes,
            &mut budget(),
            &mut work(),
            &mut AllocationBudget::new(0, 0)
        )
        .is_err()
    );
    for n in [0, 7, 12, bytes.len() - 1] {
        assert!(
            PhysicsRuntime::from_persisted_bytes(
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
        PhysicsRuntime::from_persisted_bytes(&extra, &mut budget(), &mut work(), &mut allocation())
            .is_err()
    );
    assert_eq!(save(&mut source), bytes);
}
