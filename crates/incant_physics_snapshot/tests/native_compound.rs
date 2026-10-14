#![cfg(feature = "native-admission")]
use incant_physics_snapshot::construction::AllocationBudget;
use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn allocation() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 1_000_000)
}

fn scene2() -> rapier2d::pipeline::checked_restore::capture::CheckpointOwner {
    use rapier2d::{pipeline::checked_restore::capture::CheckpointOwner, prelude::*};
    let mut owner = CheckpointOwner::new().unwrap();
    let world = owner.access_mut().unwrap();
    let tiles = [1., 3., 5.]
        .map(|x| (Pose::translation(x, 0.25), SharedShape::cuboid(1., 0.25)))
        .to_vec();
    world
        .colliders
        .insert(ColliderBuilder::compound(tiles).translation(Vector::new(-3., -0.5)));
    world.colliders.insert(
        ColliderBuilder::ball(1.5)
            .sensor(true)
            .translation(Vector::Y),
    );
    for (x, y) in [(-0.75, 1.8), (0.75, 2.7)] {
        world.insert(
            RigidBodyBuilder::dynamic()
                .can_sleep(false)
                .translation(Vector::new(x, y)),
            ColliderBuilder::ball(0.35).friction(0.7),
        );
    }
    owner
}
fn scene3() -> rapier3d::pipeline::checked_restore::capture::CheckpointOwner {
    use rapier3d::{pipeline::checked_restore::capture::CheckpointOwner, prelude::*};
    let mut owner = CheckpointOwner::new().unwrap();
    let world = owner.access_mut().unwrap();
    let floor = [-2., 0., 2.]
        .map(|x| {
            (
                Pose::translation(x, -0.25, 0.),
                SharedShape::cuboid(1., 0.25, 2.),
            )
        })
        .to_vec();
    world.colliders.insert(ColliderBuilder::compound(floor));
    world.colliders.insert(
        ColliderBuilder::ball(1.5)
            .sensor(true)
            .translation(Vector::Y),
    );
    let parts = vec![
        (
            Pose::from_parts(Vector::new(-0.15, 0., 0.), Rotation::from_rotation_z(0.17)),
            SharedShape::cuboid(0.25, 0.3, 0.2),
        ),
        (Pose::translation(0.2, 0.15, 0.), SharedShape::ball(0.22)),
        (
            Pose::translation(0., 0.4, 0.),
            SharedShape::capsule_y(0.12, 0.15),
        ),
    ];
    world.insert(
        RigidBodyBuilder::dynamic()
            .can_sleep(false)
            .translation(Vector::new(-0.7, 1.8, 0.)),
        ColliderBuilder::compound(parts).friction(0.7),
    );
    world.insert(
        RigidBodyBuilder::dynamic()
            .can_sleep(false)
            .translation(Vector::new(0.8, 2.7, 0.)),
        ColliderBuilder::ball(0.35),
    );
    owner
}

macro_rules! continuation {
    ($name:ident, $rapier:ident, $scene:ident, $expected:expr) => {
        #[test]
        fn $name() {
            use $rapier::{pipeline::checked_restore::WorkBudget, prelude::*};
            let mut source = $scene();
            for _ in 0..96 {
                source.step().unwrap();
            }
            let saved = source
                .checkpoint(&mut budget(), &mut WorkBudget::new(u64::MAX))
                .unwrap();
            // NativeFrame has a private 16-byte owner envelope. Inspect only its
            // inert payload; this does not create a restore capability.
            assert_eq!(&saved.encoded()[..8], b"ICNATIV1");
            assert_eq!(
                u32::from_le_bytes(saved.encoded()[8..12].try_into().unwrap()) as usize,
                saved.encoded().len() - 16
            );
            with_decoded_world_boundary(
                &saved.encoded()[16..],
                saved.identity(),
                &mut budget(),
                |f| {
                    let pairs = f.broad().context().narrow.context().rigid.pairs();
                    assert!(
                        pairs
                            .record()
                            .pairs
                            .iter()
                            .any(|p| p.workspace == $expected)
                    );
                    assert!(!pairs.record().detectors.is_empty());
                    let geometries = f
                        .pipeline()
                        .context()
                        .ccd
                        .context()
                        .colliders
                        .context()
                        .geometries;
                    assert!(geometries.iter().any(|g| g.compound().is_some()));
                    // A refused child walk retains the earlier metadata charge.
                    // This is observable cumulative budgeting, including failure.
                    let colliders = f.pipeline().context().ccd.context().colliders;
                    let manifolds = f.manifolds().record();
                    let rows = colliders.context().bodies.record().bodies.len()
                        + colliders.record().colliders.len()
                        + manifolds.manifolds.len()
                        + manifolds.solver.len();
                    let mut metadata_only = WorkBudget::new(1 + rows as u64 * 8);
                    assert!(matches!(
                        $rapier::pipeline::checked_restore::admit_numeric_frame(
                            f,
                            saved.identity(),
                            &mut metadata_only,
                        ),
                        Err($rapier::pipeline::checked_restore::AdmissionError::Work)
                    ));
                    assert_eq!(metadata_only.remaining(), 0);
                    Ok(())
                },
            )
            .unwrap();
            assert!(
                saved
                    .reconstruct(
                        &mut budget(),
                        &mut WorkBudget::new(u64::MAX),
                        &mut AllocationBudget::new(0, 0)
                    )
                    .is_err()
            );
            assert!(
                saved
                    .reconstruct(&mut budget(), &mut WorkBudget::new(0), &mut allocation())
                    .is_err()
            );
            let mut restored = saved
                .reconstruct(
                    &mut budget(),
                    &mut WorkBudget::new(u64::MAX),
                    &mut allocation(),
                )
                .unwrap();
            assert_eq!(
                saved.encoded(),
                restored
                    .checkpoint(&mut budget(), &mut WorkBudget::new(u64::MAX))
                    .unwrap()
                    .encoded()
            );
            for tick in 0..72 {
                // Remove one touching body, then introduce a fresh contact with
                // reused arena storage. This exercises retained detector churn.
                if tick == 20 {
                    for owner in [&mut source, &mut restored] {
                        let world = owner.access_mut().unwrap();
                        let handle = world.bodies.iter().last().unwrap().0;
                        world.remove_body(handle);
                    }
                }
                if tick == 30 {
                    for owner in [&mut source, &mut restored] {
                        owner.access_mut().unwrap().insert(
                            RigidBodyBuilder::dynamic()
                                .can_sleep(false)
                                .translation(Vector::Y * 1.2),
                            ColliderBuilder::ball(0.3),
                        );
                    }
                }
                source.step().unwrap();
                restored.step().unwrap();
                let a = source
                    .checkpoint(&mut budget(), &mut WorkBudget::new(u64::MAX))
                    .unwrap_or_else(|e| panic!("source tick {tick}: {e:?}"));
                let b = restored
                    .checkpoint(&mut budget(), &mut WorkBudget::new(u64::MAX))
                    .unwrap_or_else(|e| panic!("restored tick {tick}: {e:?}"));
                assert_eq!(
                    a.encoded(),
                    b.encoded(),
                    "compound complete native state at {tick}"
                );
                let ray = Ray::new(Vector::Y * 5., -Vector::Y);
                let hit = |w: &$rapier::pipeline::PhysicsWorld| {
                    w.query_pipeline()
                        .cast_ray(&ray, 10., true)
                        .map(|(h, t)| (h.into_raw_parts(), t.to_bits()))
                };
                assert_eq!(hit(source.get().unwrap()), hit(restored.get().unwrap()));
                if tick == 40 {
                    restored = b
                        .reconstruct(
                            &mut budget(),
                            &mut WorkBudget::new(u64::MAX),
                            &mut allocation(),
                        )
                        .unwrap();
                }
            }
        }
    };
}
continuation!(
    tile_compound_full_native_continuation,
    rapier2d,
    scene2,
    WorkspaceKind::CompositeShape
);
continuation!(
    spatial_compounds_full_native_continuation,
    rapier3d,
    scene3,
    WorkspaceKind::CompositeComposite
);
