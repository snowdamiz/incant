use incant_runtime::*;
use incant_types::{AngularVelocity2D, BodyMotion, Collider2D, RigidBody, Transform, Velocity};
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8, pos: [f64; 3], velocity: Option<[f64; 3]>) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: pos,
            ..Default::default()
        },
        velocity: velocity.map(|linear| Velocity { linear }),
    }
}
fn declaration(n: u8, motion: BodyMotion) -> CookedPhysics2D {
    CookedPhysics2D {
        entity: id(n),
        body: Some(RigidBody {
            motion,
            ..Default::default()
        }),
        collider: Collider2D::default(),
        angular_velocity: None,
    }
}
fn scene() -> CookedScene {
    let mut map = CookedSpatial::empty(id(1));
    map.tilemap = Some(Box::new(NativeTilemap {
        tileset: id(90),
        size: [3, 1],
        cell_size: [1., 1.],
        layers: vec![NativeTilemapLayer {
            id: id(80),
            cells: vec![Some(id(81)), None, Some(id(81))],
            layer: 0,
            order: 0,
            solid: true,
        }],
    }));
    CookedScene::new(
        id(99),
        64,
        vec![
            entity(1, [0., 0., 7.], None),
            entity(2, [0.5, 3., 2.], None),
            entity(3, [10., 3., 4.], Some([2., 0., 0.])),
            entity(4, [20., 3., 5.], Some([1., 0., 0.])),
        ],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: [3; 32],
        }],
        vec![map],
    )
    .unwrap()
    .with_physics_2d(vec![
        declaration(2, BodyMotion::Dynamic),
        declaration(4, BodyMotion::Kinematic),
    ])
    .unwrap()
}
fn apply(world: &mut NativeWorld, items: Vec<StructuralCommand>) -> Result<(), RuntimeError> {
    let mut q = FrameCommands::default();
    for item in items {
        q.push(item).unwrap();
    }
    world.apply(&mut q)
}
#[test]
fn v7_round_trip_real_tile_collision_sleep_and_no_double_integration() {
    let input = scene();
    let bytes = input.to_bytes();
    assert_eq!(&bytes[8..12], &7u32.to_le_bytes());
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.to_bytes(), bytes);
    assert_eq!(loaded.physics_2d(), input.physics_2d());
    let mut world = NativeWorld::new(loaded);
    let pointer = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(1))
        .unwrap()
        .tilemap
        .unwrap()
        .collision()
        .as_ptr();
    world.step().unwrap();
    assert_eq!(
        world.inspect(id(3)).unwrap().transform.translation,
        [10.03125, 3., 4.]
    );
    assert_eq!(
        world.inspect(id(4)).unwrap().transform.translation,
        [20.015625, 3., 5.]
    );
    for _ in 1..640 {
        world.step().unwrap();
    }
    let rest = world.physics_2d_state(id(2)).unwrap();
    assert!(rest.sleeping, "{rest:?}");
    assert!((rest.translation[1] - 1.5).abs() < 0.02);
    assert_eq!(world.inspect(id(2)).unwrap().transform.translation[2], 2.);
    for _ in 0..32 {
        world.step().unwrap();
        assert_eq!(world.physics_2d_state(id(2)).unwrap(), rest);
    }
    assert_eq!(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .find(|n| n.id == id(1))
            .unwrap()
            .tilemap
            .unwrap()
            .collision()
            .as_ptr(),
        pointer
    );
    assert!(world.save_bytes().is_err());
}
#[test]
fn raycast_sees_numeric_teleport_and_tile_hole_without_advancing_clock() {
    let mut world = NativeWorld::new(scene());
    let mut q = RayQuery2D {
        origin: [0.5, 2.],
        direction: [0., -1.],
        max_distance: 2.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    assert_eq!(world.raycast_2d(&q).unwrap().unwrap().entity, id(1));
    q.origin[0] = 1.5;
    assert!(world.raycast_2d(&q).unwrap().is_none());
    world.for_each_moving_mut(|id, t, _| {
        if id == crate::id(2) {
            t.translation = [1.5, 1., 2.];
        }
    });
    assert_eq!(world.raycast_2d(&q).unwrap().unwrap().entity, id(2));
    assert_eq!(world.tick(), 0);
    world.step().unwrap();
    assert!(world.physics_2d_state(id(2)).unwrap().translation[1] < 1.);
}
#[test]
fn invalid_final_motion_batch_preserves_all_components_and_queue() {
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    let mut q = FrameCommands::default();
    q.push(StructuralCommand::SetVelocity {
        entity: id(3),
        velocity: Some(Velocity {
            linear: [9., 0., 0.],
        }),
    })
    .unwrap();
    q.push(StructuralCommand::SetAngularVelocity2D {
        entity: id(1),
        velocity: None,
    })
    .unwrap();
    assert!(world.apply(&mut q).is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(q.len(), 2);
    q.clear();
    q.push(StructuralCommand::SetVelocity {
        entity: id(2),
        velocity: Some(Velocity {
            linear: [0., 0., 1.],
        }),
    })
    .unwrap();
    assert!(world.apply(&mut q).is_err());
    assert_eq!(world.snapshot(), before);
    apply(
        &mut world,
        vec![StructuralCommand::SetAngularVelocity2D {
            entity: id(4),
            velocity: Some(AngularVelocity2D { angular: 2. }),
        }],
    )
    .unwrap();
    world.step().unwrap();
    assert!((world.physics_2d_state(id(4)).unwrap().rotation - 0.03125).abs() < 1e-6);
}
#[test]
fn structural_preflight_rejects_parent_mixed_dimension_and_nonplanar_spawn_atomically() {
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    for bad in [
        StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(3)),
        },
        StructuralCommand::SetPhysics {
            entity: id(2),
            physics: Some(CookedPhysics {
                entity: id(2),
                body: None,
                collider: Default::default(),
                angular_velocity: None,
            }),
        },
        StructuralCommand::SetPhysics2D {
            entity: id(1),
            physics: Some(declaration(1, BodyMotion::Dynamic)),
        },
    ] {
        assert!(
            apply(
                &mut world,
                vec![
                    StructuralCommand::SetVelocity {
                        entity: id(3),
                        velocity: Some(Velocity {
                            linear: [9., 0., 0.]
                        })
                    },
                    bad
                ]
            )
            .is_err()
        );
        assert_eq!(world.snapshot(), before);
    }
    let mut spawned = entity(5, [2., 2., 0.], None);
    spawned.transform.scale = [2., 1., 1.];
    assert!(
        apply(
            &mut world,
            vec![
                StructuralCommand::Spawn(spawned),
                StructuralCommand::SetPhysics2D {
                    entity: id(5),
                    physics: Some(declaration(5, BodyMotion::Dynamic))
                }
            ]
        )
        .is_err()
    );
    assert_eq!(world.snapshot(), before);
}
#[test]
fn solid_map_install_replace_and_despawn_update_solver_at_one_boundary() {
    let mut world = NativeWorld::new(scene());
    let q = RayQuery2D {
        origin: [0.5, 2.],
        direction: [0., -1.],
        max_distance: 2.,
        include_sensors: false,
        exclude_entity: Some(id(2)),
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    let mut node = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(1))
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    node.tilemap.as_mut().unwrap().layers[0].cells.swap(0, 1);
    apply(
        &mut world,
        vec![StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(node),
        }],
    )
    .unwrap();
    assert!(world.raycast_2d(&q).unwrap().is_none());
    apply(
        &mut world,
        vec![
            StructuralCommand::Despawn(id(1)),
            StructuralCommand::SetPhysics2D {
                entity: id(2),
                physics: None,
            },
        ],
    )
    .unwrap();
    assert!(world.physics_2d_state(id(1)).is_none());
    assert!(world.physics_2d_state(id(2)).is_none());
    world.step().unwrap();
    assert!(world.save_bytes().is_err());
}
#[test]
fn accepted_solver_failure_preserves_public_ecs_clock_and_terminalizes_world() {
    let e = entity(1, [1_000_000., 0., 2.], Some([10000., 0., 0.]));
    let mut world = NativeWorld::new(
        CookedScene::new(id(99), 64, vec![e.clone()])
            .unwrap()
            .with_physics_2d(vec![declaration(1, BodyMotion::Kinematic)])
            .unwrap(),
    );
    let before = world.snapshot();
    assert!(world.step().is_err());
    assert_eq!(world.tick(), 0);
    assert_eq!(world.snapshot(), before);
    assert!(world.physics_2d_state(id(1)).is_none());
    assert_eq!(world.step(), Err(RuntimeError::PhysicsFailed));
    assert!(world.save_bytes().is_err());
    assert!(apply(&mut world, vec![StructuralCommand::Despawn(id(1))]).is_err());
}
#[test]
fn cooked_rejects_malformed_and_unsupported_planar_data_and_preserves_v1() {
    let plain = CookedScene::new(id(99), 64, vec![entity(1, [0.; 3], None)]).unwrap();
    let bytes = plain.to_bytes();
    assert_eq!(&bytes[8..12], &1u32.to_le_bytes());
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut bad = declaration(1, BodyMotion::Dynamic);
    bad.angular_velocity = Some(AngularVelocity2D { angular: f64::NAN });
    assert!(plain.clone().with_physics_2d(vec![bad]).is_err());
    assert!(
        plain
            .clone()
            .with_physics_2d(vec![
                declaration(1, BodyMotion::Dynamic),
                declaration(1, BodyMotion::Fixed)
            ])
            .is_err()
    );
    let mut unknown = scene().to_bytes();
    unknown[8..12].copy_from_slice(&14u32.to_le_bytes());
    assert_eq!(
        CookedScene::from_bytes(&unknown).unwrap_err(),
        SceneError::Version(14)
    );
}

#[test]
fn no_op_planar_reinstallation_preserves_rotated_sleeping_body() {
    for sample in 1..=64 {
        let mut data = entity(1, [0., 3., 2.], None);
        let angle = f64::from(sample as f32 / 100.);
        data.transform.rotation = [0., 0., libm::sin(angle / 2.), libm::cos(angle / 2.)];
        let mut config = declaration(1, BodyMotion::Dynamic);
        config.body.as_mut().unwrap().gravity_scale = 0.;
        let mut world = NativeWorld::new(
            CookedScene::new(id(99), 64, vec![data])
                .unwrap()
                .with_physics_2d(vec![config])
                .unwrap(),
        );
        for _ in 0..256 {
            world.step().unwrap();
        }
        let resting = world.physics_2d_state(id(1)).unwrap();
        assert!(resting.sleeping);
        let unchanged = world.physics_2d_configuration(id(1)).unwrap();
        apply(
            &mut world,
            vec![StructuralCommand::SetPhysics2D {
                entity: id(1),
                physics: Some(unchanged),
            }],
        )
        .unwrap();
        assert_eq!(world.physics_2d_state(id(1)), Some(resting));
    }
}

fn rehash(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let len = (bytes.len() - 72) as u32;
    bytes[36..40].copy_from_slice(&len.to_le_bytes());
    let sum = Sha256::new()
        .chain_update(&bytes[..40])
        .chain_update(&bytes[72..])
        .finalize();
    bytes[40..72].copy_from_slice(&sum);
}
#[test]
fn v7_decoder_refuses_corrupt_tags_booleans_counts_ids_and_truncation_after_checksum() {
    let mut config = declaration(1, BodyMotion::Kinematic);
    config.angular_velocity = Some(AngularVelocity2D { angular: 0. });
    let scene = CookedScene::new(id(99), 64, vec![entity(1, [0.; 3], None)])
        .unwrap()
        .with_physics_2d(vec![config.clone()])
        .unwrap();
    let bytes = scene.to_bytes();
    let record = 196;
    assert_eq!(bytes.len(), record + 102);
    for offset in [16, 41, 42, 43, 76, 85] {
        let mut bad = bytes.clone();
        bad[record + offset] = 255;
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "tag {offset}");
    }
    for count in [0, 2, u32::MAX] {
        let mut bad = bytes.clone();
        bad[record - 4..record].copy_from_slice(&count.to_le_bytes());
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad[record..record + 16].copy_from_slice(&id(9).0);
    rehash(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
    for length in [72, record, record + 60, bytes.len() - 1] {
        let mut bad = bytes[..length].to_vec();
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "truncated {length}");
    }
    let mut next = config.clone();
    next.entity = id(2);
    let scene = CookedScene::new(
        id(99),
        64,
        vec![entity(1, [0.; 3], None), entity(2, [0.; 3], None)],
    )
    .unwrap()
    .with_physics_2d(vec![config, next])
    .unwrap();
    let mut bad = scene.to_bytes();
    let record = 300;
    bad[record + 102..record + 118].copy_from_slice(&id(1).0);
    rehash(&mut bad);
    assert!(matches!(
        CookedScene::from_bytes(&bad),
        Err(SceneError::Invalid("noncanonical 2D order"))
    ));
}
#[test]
fn independent_2d_and_3d_solvers_advance_once_in_one_world() {
    let scene = CookedScene::new(
        id(99),
        64,
        vec![
            entity(1, [0.; 3], Some([1., 0., 0.])),
            entity(2, [0.; 3], Some([2., 0., 1.])),
        ],
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: id(2),
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
        collider: Default::default(),
        angular_velocity: None,
    }])
    .unwrap()
    .with_physics_2d(vec![declaration(1, BodyMotion::Kinematic)])
    .unwrap();
    let mut world = NativeWorld::new(scene);
    world.step().unwrap();
    assert_eq!(
        world.inspect(id(1)).unwrap().transform.translation,
        [0.015625, 0., 0.]
    );
    assert_eq!(
        world.inspect(id(2)).unwrap().transform.translation,
        [0.03125, 0., 0.015625]
    );
    assert!(world.physics_2d_state(id(1)).is_some());
    assert!(world.physics_state(id(2)).is_some());
}
#[test]
fn solid_map_no_op_keeps_retained_storage_and_rejects_queued_nonzero_motion() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let (node, pointer, generation) = {
        let view = world.spatial_view().unwrap();
        let n = view.nodes().find(|n| n.id == id(1)).unwrap();
        (
            n.declaration.unwrap().clone(),
            n.tilemap.unwrap().collision().as_ptr(),
            view.binding_generation(),
        )
    };
    apply(
        &mut world,
        vec![
            StructuralCommand::SetSpatial {
                entity: id(1),
                spatial: Some(node),
            },
            StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(Velocity { linear: [0.; 3] }),
            },
        ],
    )
    .unwrap();
    let view = world.spatial_view().unwrap();
    assert_eq!(view.binding_generation(), generation);
    assert_eq!(
        view.nodes()
            .find(|n| n.id == id(1))
            .unwrap()
            .tilemap
            .unwrap()
            .collision()
            .as_ptr(),
        pointer
    );
    let before = world.snapshot();
    let physics = world.physics_2d_state(id(1));
    let mut queue = FrameCommands::default();
    queue
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    assert!(world.apply(&mut queue).is_err());
    assert_eq!(queue.len(), 1);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.physics_2d_state(id(1)), physics);
    world.step().unwrap();
    assert_eq!(world.physics_2d_state(id(1)), physics);
}
