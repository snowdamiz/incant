use super::*;
use incant_physics_snapshot::MAX_ALLOCATION;
use incant_types::{Collider2D, ColliderShape2D, RigidBody};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn work() -> WorkBudget {
    WorkBudget::new(100_000_000)
}
fn allocations() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 100_000)
}
fn declaration(dynamic: bool, sensor: bool, y: f64) -> PreparedBody {
    PreparedBody::new(
        dynamic.then(RigidBody::default),
        Collider2D {
            shape: ColliderShape2D::Box {
                half_extents: if dynamic { [0.4, 0.4] } else { [4., 0.5] },
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
}
fn warm() -> PhysicsWorld2D {
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![
            (id(1), Some(declaration(false, false, -0.5))),
            (id(2), Some(declaration(true, false, 1.))),
            (id(3), Some(declaration(false, true, 0.6))),
        ])
        .unwrap();
    for _ in 0..64 {
        world.step(1. / 64.).unwrap();
    }
    world
}
fn capture(world: &mut PhysicsWorld2D) -> PhysicsCheckpoint {
    world.checkpoint(&mut budget(), &mut work()).unwrap()
}
fn ray() -> RayQuery {
    RayQuery {
        origin: [0., 4.],
        direction: [0., -1.],
        max_distance: 10.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}
fn states(world: &PhysicsWorld2D) -> Vec<(StableId, BodyState)> {
    world
        .states()
        .map(|(id, state)| (id, state.clone()))
        .collect()
}

#[test]
fn planar_checkpoint_preserves_native_continuation_queries_and_stable_ids() {
    let mut source = warm();
    let before_query = capture(&mut source);
    assert!(source.raycast(&ray()).unwrap().is_some());
    let echo = source.state(id(2)).unwrap().clone();
    source.sync_motion(&[(id(2), echo)]).unwrap();
    let checkpoint = capture(&mut source);
    assert_eq!(before_query.native.encoded(), checkpoint.native.encoded());
    let mut restored = checkpoint
        .reconstruct(&mut budget(), &mut work(), &mut allocations())
        .unwrap();
    assert_eq!(states(&source), states(&restored));
    assert_eq!(source.events(), restored.events());
    for tick in 0..96 {
        // Exercise removal/reuse and a genuine motion edit after restoration.
        for world in [&mut source, &mut restored] {
            if tick == 8 {
                world.apply(vec![(id(3), None)]).unwrap();
            }
            if tick == 16 {
                world
                    .apply(vec![(id(4), Some(declaration(false, true, 0.6)))])
                    .unwrap();
            }
            if tick == 40 {
                let mut motion = world.state(id(2)).unwrap().clone();
                motion.translation[1] += 1.;
                motion.velocity[0] = 0.2;
                world.sync_motion(&[(id(2), motion)]).unwrap();
            }
            world.step(1. / 64.).unwrap();
        }
        assert_eq!(states(&source), states(&restored), "tick {tick}");
        assert_eq!(source.events(), restored.events(), "tick {tick}");
        assert_eq!(
            source.raycast(&ray()),
            restored.raycast(&ray()),
            "tick {tick}"
        );
        assert_eq!(
            capture(&mut source).native.encoded(),
            capture(&mut restored).native.encoded(),
            "tick {tick}"
        );
    }
}

#[test]
fn planar_checkpoint_refusals_do_not_mutate_the_running_world() {
    let mut world = warm();
    let mut checkpoint = capture(&mut world);
    let before = checkpoint.native.encoded().to_vec();
    assert!(
        world
            .checkpoint(&mut Budget::new(0).unwrap(), &mut work())
            .is_err()
    );
    assert!(
        world
            .checkpoint(&mut budget(), &mut WorkBudget::new(0))
            .is_err()
    );
    assert!(
        world
            .restore_checkpoint(
                &checkpoint,
                &mut budget(),
                &mut work(),
                &mut AllocationBudget::new(0, 0)
            )
            .is_err()
    );
    assert_eq!(capture(&mut world).native.encoded(), before);
    // Fail after constructing the native candidate: an invalid retained outer
    // observation must not replace an otherwise healthy running owner.
    checkpoint.entries[1].config.state.translation[0] += 2.;
    assert!(matches!(
        world.restore_checkpoint(&checkpoint, &mut budget(), &mut work(), &mut allocations()),
        Err(CheckpointError::Owner("committed body observation"))
    ));
    assert_eq!(capture(&mut world).native.encoded(), before);
    world.step(1. / 64.).unwrap();
}

#[test]
fn planar_checkpoint_requires_a_completed_step_after_real_mutations() {
    let mut world = warm();
    let mut motion = world.state(id(2)).unwrap().clone();
    motion.velocity[0] += 1.;
    world.sync_motion(&[(id(2), motion)]).unwrap();
    assert!(world.checkpoint(&mut budget(), &mut work()).is_err());
    world.step(1. / 64.).unwrap();
    capture(&mut world);
    world.apply(vec![(id(3), None)]).unwrap();
    assert!(world.checkpoint(&mut budget(), &mut work()).is_err());
    world.step(1. / 64.).unwrap();
    let checkpoint = capture(&mut world);
    assert!(
        world
            .events()
            .iter()
            .any(|event| !event.entered && event.second == id(3))
    );
    let restored = checkpoint
        .reconstruct(&mut budget(), &mut work(), &mut allocations())
        .unwrap();
    assert_eq!(restored.events(), world.events());
    assert!(restored.state(id(3)).is_none());
}

#[test]
fn planar_outer_join_charges_false_native_intersection_candidates() {
    let mut world = PhysicsWorld2D::default();
    let bodies = [([0., 0.], id(1)), ([0.8, 0.8], id(2))]
        .into_iter()
        .map(|(translation, id)| {
            (
                id,
                Some(
                    PreparedBody::new(
                        None,
                        Collider2D {
                            shape: ColliderShape2D::Circle { radius: 0.5 },
                            sensor: true,
                            ..Default::default()
                        },
                        BodyState {
                            translation,
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                ),
            )
        })
        .collect();
    world.apply(bodies).unwrap();
    world.step(1. / 64.).unwrap();
    assert!(world.overlaps.is_empty());
    let pairs: Vec<_> = world.world.read().intersection_pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert!(!pairs[0].4);
    assert!(charge_native_pairs(&world, &mut WorkBudget::new(0)).is_err());
    let mut exact = WorkBudget::new(1);
    charge_native_pairs(&world, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
}

#[test]
fn planar_failed_simulation_refuses_capture_before_native_access_or_charges() {
    let mut world = warm();
    let mut motion = world.state(id(2)).unwrap().clone();
    motion.translation[0] = 999_999.;
    motion.velocity[0] = 2.;
    world.sync_motion(&[(id(2), motion)]).unwrap();
    assert_eq!(world.step(1.), Err(PhysicsError::Failed));
    let mut payload = budget();
    let mut visits = work();
    let before = (payload.remaining(), visits.remaining());
    assert!(matches!(
        world.checkpoint(&mut payload, &mut visits),
        Err(CheckpointError::Owner("failed runtime"))
    ));
    assert_eq!(before, (payload.remaining(), visits.remaining()));
}
