use super::*;
use crate::{CookedEntity, CookedPhysics, CookedPhysics2D, FrameCommands, StructuralCommand};
use incant_physics_snapshot::MAX_ALLOCATION;
use incant_types::{
    AngularVelocity, AngularVelocity2D, Collider, Collider2D, ColliderShape, ColliderShape2D,
    RigidBody, Transform,
};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn payload() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn planar_work() -> PlanarWork {
    PlanarWork::new(100_000_000)
}
fn spatial_work() -> SpatialWork {
    SpatialWork::new(100_000_000)
}
fn allocations() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 100_000)
}
fn capture(w: &mut NativeWorld) -> WorldCheckpoint {
    w.checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work())
        .unwrap()
}
fn reconstruct(c: &WorldCheckpoint) -> NativeWorld {
    c.reconstruct(
        &mut payload(),
        &mut planar_work(),
        &mut spatial_work(),
        &mut allocations(),
    )
    .unwrap()
}
fn restore(w: &mut NativeWorld, c: &WorldCheckpoint) -> Result<(), WorldCheckpointError> {
    w.restore_checkpoint(
        c,
        &mut payload(),
        &mut planar_work(),
        &mut spatial_work(),
        &mut allocations(),
    )
}
fn entity(n: u8, y: f64) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: [0., y, 0.],
            ..Default::default()
        },
        velocity: None,
    }
}
fn warm() -> NativeWorld {
    let entities = vec![
        entity(1, -0.5),
        entity(2, 1.),
        entity(3, 0.6),
        entity(11, -0.5),
        entity(12, 1.),
        entity(13, 0.6),
    ];
    let spatial = (1..=3)
        .map(|n| CookedPhysics {
            entity: id(n),
            body: (n == 2).then(RigidBody::default),
            collider: Collider {
                shape: ColliderShape::Box {
                    half_extents: if n == 2 { [0.4; 3] } else { [4., 0.5, 4.] },
                },
                sensor: n == 3,
                ..Default::default()
            },
            angular_velocity: None,
        })
        .collect();
    let planar = (11..=13)
        .map(|n| CookedPhysics2D {
            entity: id(n),
            body: (n == 12).then(RigidBody::default),
            collider: Collider2D {
                shape: ColliderShape2D::Box {
                    half_extents: if n == 12 { [0.4; 2] } else { [4., 0.5] },
                },
                sensor: n == 13,
                ..Default::default()
            },
            angular_velocity: None,
        })
        .collect();
    let scene = CookedScene::new(id(99), 64, entities)
        .unwrap()
        .with_physics(spatial)
        .unwrap()
        .with_physics_2d(planar)
        .unwrap();
    let mut w = NativeWorld::new(scene);
    for _ in 0..64 {
        w.step().unwrap();
    }
    w
}
fn observations(a: &NativeWorld, b: &NativeWorld) {
    assert_eq!(a.tick(), b.tick());
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.physics_events(), b.physics_events());
    assert_eq!(a.physics_2d_events(), b.physics_2d_events());
    for (&id, &handle) in &a.entities {
        assert_eq!(a.physics_state(id), b.physics_state(id));
        assert_eq!(a.physics_2d_state(id), b.physics_2d_state(id));
        assert_eq!(a.physics_configuration(id), b.physics_configuration(id));
        assert_eq!(
            a.physics_2d_configuration(id),
            b.physics_2d_configuration(id)
        );
        if let Some(binding) = a.world.get::<Binding2D>(handle) {
            let other = b.world.get::<Binding2D>(b.entities[&id]).unwrap();
            assert_eq!(
                binding.rotation.map(f64::to_bits),
                other.rotation.map(f64::to_bits)
            );
        }
    }
    let a_native: Vec<_> = a.physics.state_iter().collect();
    let b_native: Vec<_> = b.physics.state_iter().collect();
    assert_eq!(a_native, b_native);
    assert_eq!(
        a.physics_2d.states().collect::<Vec<_>>(),
        b.physics_2d.states().collect::<Vec<_>>()
    );
}
fn apply(w: &mut NativeWorld, items: impl IntoIterator<Item = StructuralCommand>) {
    let mut commands = FrameCommands::default();
    for item in items {
        commands.push(item).unwrap();
    }
    w.apply(&mut commands).unwrap();
}

#[test]
fn checkpoint_retains_pending_inputs_and_paired_native_continuation() {
    let mut source = warm();
    let spatial_before = source.physics.state(&id(99), &id(2)).unwrap().clone();
    let planar_before = source.physics_2d.state(id(12)).unwrap().clone();
    source.with_numeric_columns(|columns| {
        columns.for_each_chunk(|mut chunk| {
            for (i, entity) in chunk.ids.iter().enumerate() {
                if [id(2), id(12)].contains(entity) {
                    chunk.transforms[i * crate::TRANSFORM_STRIDE + 1] += 0.75;
                    chunk.velocities.as_mut().unwrap()[i * crate::VELOCITY_STRIDE] = 0.2;
                    if *entity == id(12) {
                        chunk.transforms
                            [i * crate::TRANSFORM_STRIDE + 3..i * crate::TRANSFORM_STRIDE + 7]
                            .copy_from_slice(&crate::physics_2d::rotation(0.31));
                    }
                }
            }
        })
    });
    // Angular inputs are committed ECS values and remain separate from native
    // observations, exactly as a post-physics script command would leave them.
    source
        .world
        .get_mut::<crate::physics::Spin>(source.entities[&id(2)])
        .unwrap()
        .0 = AngularVelocity {
        angular: [0., 0., 0.13],
    };
    source
        .world
        .get_mut::<crate::physics_2d::Spin2D>(source.entities[&id(12)])
        .unwrap()
        .0 = AngularVelocity2D { angular: 0.17 };
    let before = source.snapshot();
    let checkpoint = capture(&mut source);
    assert_eq!(source.snapshot(), before);
    assert_eq!(source.physics.state(&id(99), &id(2)), Some(&spatial_before));
    assert_eq!(source.physics_2d.state(id(12)), Some(&planar_before));
    let mut restored = reconstruct(&checkpoint);
    observations(&source, &restored);
    assert_ne!(source.incarnation(), restored.incarnation());
    for tick in 0..96 {
        for w in [&mut source, &mut restored] {
            if tick == 12 {
                apply(
                    w,
                    [
                        StructuralCommand::Despawn(id(3)),
                        StructuralCommand::Despawn(id(13)),
                    ],
                );
            }
            w.step().unwrap();
        }
        observations(&source, &restored);
    }
}

#[test]
fn checkpoint_restores_atomically_and_changes_incarnations() {
    let mut world = warm();
    let checkpoint = capture(&mut world);
    let at_checkpoint = world.snapshot();
    let incarnation = world.incarnation();
    let entity = world.entity_identity(id(2));
    let identity = world.identity.clone();
    world.step().unwrap();
    restore(&mut world, &checkpoint).unwrap();
    assert_eq!(world.snapshot(), at_checkpoint);
    assert_eq!(world.tick(), 64);
    assert_eq!(world.identity, identity);
    assert_ne!(world.incarnation(), incarnation);
    assert_ne!(world.entity_identity(id(2)), entity);
    assert_eq!(world.topology_generation, 1);
    assert_eq!(world.binding_generation, 1);
    world.step().unwrap();
}

#[test]
fn compatible_staging_keeps_live_world_unchanged_and_new_identity_fresh() {
    let mut current = warm();
    let checkpoint = capture(&mut current);
    current.step().unwrap();
    let before = current.snapshot();
    let incarnation = current.incarnation();
    let identity = current.identity.clone();
    let candidate = checkpoint
        .reconstruct_for(
            &current,
            &mut crate::CheckpointPayloadBudget::new(MAX_ALLOCATION).unwrap(),
            &mut crate::PlanarCheckpointWorkBudget::new(100_000_000),
            &mut crate::SpatialCheckpointWorkBudget::new(100_000_000),
            &mut crate::CheckpointAllocationBudget::new(MAX_ALLOCATION, 100_000),
        )
        .unwrap();
    assert_eq!(current.snapshot(), before);
    assert_eq!(current.incarnation(), incarnation);
    assert_eq!(current.identity, identity);
    assert_ne!(candidate.identity, current.identity);
    assert_ne!(candidate.incarnation(), incarnation);
    assert_eq!(candidate.tick(), 64);
    current.source_scene_fingerprint[0] ^= 1;
    assert!(matches!(
        checkpoint.reconstruct_for(
            &current,
            &mut payload(),
            &mut planar_work(),
            &mut spatial_work(),
            &mut allocations()
        ),
        Err(WorldCheckpointError::Components(SaveError::WorldMismatch))
    ));
    assert_eq!(current.snapshot(), before);
    assert_eq!(current.incarnation(), incarnation);
}

#[test]
fn checkpoint_late_cache_and_generation_failures_do_not_publish() {
    let mut world = warm();
    let mut checkpoint = capture(&mut world);
    world.step().unwrap();
    let before = world.snapshot();
    let incarnation = world.incarnation();
    checkpoint.planar_rotations[0].1[2] += 0.1;
    assert!(matches!(
        restore(&mut world, &checkpoint),
        Err(WorldCheckpointError::Owner("planar published rotation"))
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.incarnation(), incarnation);
    let checkpoint = capture(&mut world);
    world.ui_generation = u64::MAX;
    assert!(matches!(
        restore(&mut world, &checkpoint),
        Err(WorldCheckpointError::Components(SaveError::Invalid(
            "UI generation exhausted"
        )))
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.incarnation(), incarnation);
    world.step().unwrap();
}

#[test]
fn checkpoint_budget_and_world_binding_failures_do_not_publish() {
    let mut world = warm();
    let checkpoint = capture(&mut world);
    let before = world.snapshot();
    let incarnation = world.incarnation();
    assert!(
        world
            .restore_checkpoint(
                &checkpoint,
                &mut payload(),
                &mut planar_work(),
                &mut spatial_work(),
                &mut AllocationBudget::new(0, 0)
            )
            .is_err()
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.incarnation(), incarnation);
    world.source_scene_fingerprint[0] ^= 1;
    assert!(matches!(
        restore(&mut world, &checkpoint),
        Err(WorldCheckpointError::Components(SaveError::WorldMismatch))
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.incarnation(), incarnation);
}

#[test]
fn checkpoint_keeps_byte_admission_closed_and_retains_empty_history() {
    let mut world = warm();
    let checkpoint = capture(&mut world);
    assert_eq!(world.save_bytes(), Err(SaveError::PhysicsUnsupported));
    assert!(matches!(
        NativeWorld::from_save_bytes(&checkpoint.components),
        Err(SaveError::PhysicsUnsupported)
    ));
    apply(
        &mut world,
        [1, 2, 3, 11, 12, 13].map(|n| StructuralCommand::Despawn(id(n))),
    );
    assert!(
        world
            .checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work())
            .is_err()
    );
    world.step().unwrap();
    assert!(world.physics_events().iter().any(|e| !e.entered));
    assert!(world.physics_2d_events().iter().any(|e| !e.entered));
    let checkpoint = capture(&mut world);
    let restored = reconstruct(&checkpoint);
    observations(&world, &restored);
    assert_eq!(restored.entity_count(), 0);
    assert!(restored.physics_used && restored.physics_2d_used);
    assert_eq!(restored.save_bytes(), Err(SaveError::PhysicsUnsupported));
}

#[test]
fn checkpoint_rejects_invalid_pending_and_inconsistent_configuration() {
    let mut world = warm();
    let original = world.physics.state(&id(99), &id(2)).unwrap().clone();
    world.with_numeric_columns(|c| {
        c.for_each_chunk(|chunk| {
            if let Some(i) = chunk.ids.iter().position(|n| *n == id(2)) {
                chunk.transforms[i * crate::TRANSFORM_STRIDE] = f64::NAN;
            }
        })
    });
    assert!(
        world
            .checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work())
            .is_err()
    );
    assert_eq!(world.physics.state(&id(99), &id(2)), Some(&original));
    let mut world = warm();
    world
        .world
        .get_mut::<crate::physics::Binding>(world.entities[&id(2)])
        .unwrap()
        .collider
        .friction += 0.1;
    assert!(matches!(
        world.checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work()),
        Err(WorldCheckpointError::Owner("spatial configuration"))
    ));
}

#[test]
fn failed_world_refuses_before_consuming_budgets() {
    let mut world = warm();
    let checkpoint = capture(&mut world);
    world.physics_failed = true;
    let mut p = payload();
    let mut pwork = planar_work();
    let mut swork = spatial_work();
    let before = (p.remaining(), pwork.remaining(), swork.remaining());
    assert!(matches!(
        world.checkpoint(&mut p, &mut pwork, &mut swork),
        Err(WorldCheckpointError::Owner("failed world"))
    ));
    assert!(matches!(
        world.restore_checkpoint(
            &checkpoint,
            &mut p,
            &mut pwork,
            &mut swork,
            &mut allocations()
        ),
        Err(WorldCheckpointError::Owner("failed world"))
    ));
    assert_eq!(
        before,
        (p.remaining(), pwork.remaining(), swork.remaining())
    );
}

#[test]
fn never_used_empty_world_needs_no_native_completion() {
    let mut world = NativeWorld::new(CookedScene::new(id(99), 64, vec![]).unwrap());
    let checkpoint = capture(&mut world);
    assert!(checkpoint.planar.is_none() && checkpoint.spatial.is_none());
    let restored = reconstruct(&checkpoint);
    assert_eq!(world.save_bytes().unwrap(), restored.save_bytes().unwrap());
}

#[test]
fn spatial_only_world_retains_stepped_empty_planar_owner() {
    let scene = CookedScene::new(id(99), 64, vec![entity(2, 1.)])
        .unwrap()
        .with_physics(vec![CookedPhysics {
            entity: id(2),
            body: Some(RigidBody::default()),
            collider: Collider::default(),
            angular_velocity: None,
        }])
        .unwrap();
    let mut source = NativeWorld::new(scene);
    for _ in 0..8 {
        source.step().unwrap();
    }
    assert!(!source.physics_2d_used);
    let checkpoint = capture(&mut source);
    let mut restored = reconstruct(&checkpoint);
    assert!(!restored.physics_2d_used);
    // A fresh empty planar owner cannot yield a completed native checkpoint.
    // Hydration must preserve the real owner even before its first declaration.
    assert!(
        restored
            .physics_2d
            .checkpoint(&mut payload(), &mut planar_work())
            .is_ok()
    );
    for world in [&mut source, &mut restored] {
        apply(world, [StructuralCommand::Spawn(entity(12, 3.))]);
        apply(
            world,
            [StructuralCommand::SetPhysics2D {
                entity: id(12),
                physics: Some(CookedPhysics2D {
                    entity: id(12),
                    body: Some(RigidBody::default()),
                    collider: Collider2D::default(),
                    angular_velocity: None,
                }),
            }],
        );
    }
    for _ in 0..32 {
        source.step().unwrap();
        restored.step().unwrap();
        observations(&source, &restored);
    }
}

#[test]
fn rewind_keeps_save_obligation_without_inventing_solver_history() {
    let mut source = NativeWorld::new(CookedScene::new(id(99), 64, vec![]).unwrap());
    let checkpoint = capture(&mut source);
    let mut oracle = reconstruct(&checkpoint);
    apply(
        &mut source,
        [
            StructuralCommand::Spawn(entity(12, 3.)),
            StructuralCommand::SetPhysics2D {
                entity: id(12),
                physics: Some(CookedPhysics2D {
                    entity: id(12),
                    body: Some(RigidBody::default()),
                    collider: Collider2D::default(),
                    angular_velocity: None,
                }),
            },
        ],
    );
    source.step().unwrap();
    restore(&mut source, &checkpoint).unwrap();
    assert_eq!(source.save_bytes(), Err(SaveError::PhysicsUnsupported));
    for _ in 0..8 {
        source.step().unwrap();
        oracle.step().unwrap();
        observations(&source, &oracle);
    }
    // Neither world has advanced a planar owner in this restored timeline.
    assert!(
        source
            .physics_2d
            .checkpoint(&mut payload(), &mut planar_work())
            .is_err()
    );
    assert!(
        oracle
            .physics_2d
            .checkpoint(&mut payload(), &mut planar_work())
            .is_err()
    );
    let recaptured = capture(&mut source);
    let again = reconstruct(&recaptured);
    assert_eq!(again.save_bytes(), Err(SaveError::PhysicsUnsupported));
    for world in [&mut source, &mut oracle] {
        apply(
            world,
            [
                StructuralCommand::Spawn(entity(12, 3.)),
                StructuralCommand::SetPhysics2D {
                    entity: id(12),
                    physics: Some(CookedPhysics2D {
                        entity: id(12),
                        body: Some(RigidBody::default()),
                        collider: Collider2D::default(),
                        angular_velocity: None,
                    }),
                },
            ],
        );
    }
    for _ in 0..32 {
        source.step().unwrap();
        oracle.step().unwrap();
        observations(&source, &oracle);
    }
}

mod compound_tiles;
