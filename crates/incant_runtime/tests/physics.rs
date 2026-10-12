use incant_runtime::{
    BodyState, CookedEntity, CookedPhysics, CookedScene, FrameCommands, NativeWorld, RuntimeError,
    SaveError, SceneError, StableId, StructuralCommand,
};
use incant_types::{
    AngularVelocity, BodyMotion, Collider, ColliderPart, ColliderShape, PrimitiveColliderShape,
    RigidBody, Transform, Velocity,
};
use sha2::{Digest, Sha256};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn entity(n: u128, position: [f64; 3]) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: position,
            ..Default::default()
        },
        velocity: None,
    }
}
fn physics(n: u128, motion: Option<BodyMotion>) -> CookedPhysics {
    CookedPhysics {
        entity: id(n),
        body: motion.map(|motion| RigidBody {
            motion,
            ..Default::default()
        }),
        collider: Collider::default(),
        angular_velocity: None,
    }
}
fn scene(entities: Vec<CookedEntity>, physics: Vec<CookedPhysics>) -> CookedScene {
    CookedScene::new(id(999), 60, entities)
        .unwrap()
        .with_physics(physics)
        .unwrap()
}
fn apply(world: &mut NativeWorld, commands: Vec<StructuralCommand>) -> Result<(), RuntimeError> {
    let mut buffer = FrameCommands::default();
    for command in commands {
        buffer.push(command).unwrap();
    }
    world.apply(&mut buffer)
}
fn states(world: &NativeWorld, ids: &[u128]) -> Vec<BodyState> {
    ids.iter()
        .map(|n| world.physics_state(id(*n)).unwrap())
        .collect()
}
fn rehash(bytes: &mut [u8]) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..40]);
    hash.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&hash.finalize());
}

#[test]
fn cooked_physics_round_trip_is_canonical_and_version_one_is_retained() {
    let plain = scene(vec![entity(1, [0.; 3])], vec![]).to_bytes();
    assert_eq!(&plain[8..12], &1_u32.to_le_bytes());
    assert_eq!(CookedScene::from_bytes(&plain).unwrap().to_bytes(), plain);
    let mut compound = physics(2, Some(BodyMotion::Kinematic));
    compound.angular_velocity = Some(AngularVelocity {
        angular: [0., 0., 1.],
    });
    compound.collider.shape = ColliderShape::Compound {
        parts: vec![
            ColliderPart {
                id: "0000000000000000000000000a".into(),
                translation: [1., 0., 0.],
                rotation: [0., 0., 0., 1.],
                shape: PrimitiveColliderShape::Sphere { radius: 0.2 },
            },
            ColliderPart {
                id: "0000000000000000000000000A".into(),
                translation: [-1., 0., 0.],
                rotation: [0., 0., 0., 1.],
                shape: PrimitiveColliderShape::Capsule {
                    half_height: 0.1,
                    radius: 0.2,
                },
            },
        ],
    };
    let cooked = scene(
        vec![entity(2, [0., 3., 0.]), entity(1, [0.; 3])],
        vec![compound, physics(1, None)],
    );
    let bytes = cooked.to_bytes();
    assert_eq!(&bytes[8..12], &2_u32.to_le_bytes());
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.physics(), cooked.physics());
    assert_eq!(loaded.to_bytes(), bytes);
    let mut first = NativeWorld::new(cooked);
    let mut second = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..30 {
        first.step().unwrap();
        second.step().unwrap();
        assert_eq!(states(&first, &[1, 2]), states(&second, &[1, 2]));
    }
}

#[test]
fn invalid_physics_records_tags_pairs_and_versions_fail_closed() {
    let entities = vec![entity(1, [0.; 3]), entity(2, [0.; 3])];
    for declarations in [
        vec![physics(3, None)],
        vec![physics(1, None), physics(1, None)],
        vec![CookedPhysics {
            angular_velocity: Some(AngularVelocity { angular: [0.; 3] }),
            ..physics(1, None)
        }],
    ] {
        assert!(
            CookedScene::new(id(999), 60, entities.clone())
                .unwrap()
                .with_physics(declarations)
                .is_err()
        );
    }
    let bytes = scene(entities, vec![physics(1, None), physics(2, None)]).to_bytes();
    // Header + two transform-only entity records + physics count.
    let record = 72 + 2 * 104 + 4;
    for (offset, value) in [
        (record + 16, 9),
        (record + 17, 2),
        (record + 18 + 24, 2),
        (record + 18 + 24 + 1 + 8, 9),
    ] {
        let mut invalid = bytes.clone();
        invalid[offset] = value;
        rehash(&mut invalid);
        assert!(
            CookedScene::from_bytes(&invalid).is_err(),
            "tag at {offset}"
        );
    }
    let mut invalid = bytes.clone();
    invalid[record..record + 16].copy_from_slice(&id(3).0);
    rehash(&mut invalid);
    assert!(CookedScene::from_bytes(&invalid).is_err());
    let mut invalid = bytes.clone();
    invalid[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        CookedScene::from_bytes(&invalid).unwrap_err(),
        SceneError::Version(u32::MAX)
    );
    for len in [72, record, bytes.len() - 1] {
        assert!(CookedScene::from_bytes(&bytes[..len]).is_err());
    }
}

#[test]
fn native_physics_moves_once_falls_collides_and_stays_asleep() {
    let mut moving = entity(3, [0., 4., 0.]);
    moving.velocity = Some(Velocity {
        linear: [3., 0., 0.],
    });
    let mut kine = physics(3, Some(BodyMotion::Kinematic));
    kine.body.as_mut().unwrap().gravity_scale = 0.;
    let mut floor = physics(1, None);
    floor.collider.shape = ColliderShape::Box {
        half_extents: [20., 0.5, 20.],
    };
    let mut world = NativeWorld::new(scene(
        vec![
            entity(1, [0., -0.5, 0.]),
            entity(2, [0., 3., 0.]),
            moving,
            entity(4, [0., 6., 0.]),
        ],
        vec![floor, physics(2, Some(BodyMotion::Dynamic)), kine],
    ));
    apply(
        &mut world,
        vec![StructuralCommand::SetVelocity {
            entity: id(4),
            velocity: Some(Velocity {
                linear: [3., 0., 0.],
            }),
        }],
    )
    .unwrap();
    world.step().unwrap();
    assert!((world.inspect(id(3)).unwrap().transform.translation[0] - 0.05).abs() < 1e-6);
    assert!((world.inspect(id(4)).unwrap().transform.translation[0] - 0.05).abs() < 1e-12);
    assert!(world.physics_state(id(2)).unwrap().translation[1] < 3.);
    for _ in 0..599 {
        world.step().unwrap();
    }
    let sleeping = world.physics_state(id(2)).unwrap();
    assert!(sleeping.sleeping);
    assert!((sleeping.translation[1] - 0.5).abs() < 0.02);
    assert_eq!(
        world.physics_state(id(1)).unwrap().translation,
        [0., -0.5, 0.]
    );
    for _ in 0..30 {
        world.with_numeric_columns(|columns| columns.for_each_chunk(|_| {}));
        world.step().unwrap();
        assert_eq!(world.physics_state(id(2)).unwrap(), sleeping);
    }
    assert!(
        world.inspect(id(2)).unwrap().velocity.is_some(),
        "body supplies missing zero motion at load"
    );
}

#[test]
fn structural_physics_changes_are_prevalidated_and_keep_other_bodies() {
    let mut body = physics(1, Some(BodyMotion::Kinematic));
    body.body.as_mut().unwrap().gravity_scale = 0.;
    let mut world = NativeWorld::new(scene(
        vec![entity(1, [0.; 3]), entity(2, [10., 0., 0.])],
        vec![body],
    ));
    let before = world.snapshot();
    let old = world.physics_state(id(1)).unwrap();
    let bad = vec![
        StructuralCommand::SetVelocity {
            entity: id(2),
            velocity: Some(Velocity {
                linear: [5., 0., 0.],
            }),
        },
        StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(2)),
        },
    ];
    let mut buffer = FrameCommands::default();
    for c in bad {
        buffer.push(c).unwrap();
    }
    assert!(world.apply(&mut buffer).is_err());
    assert_eq!(buffer.len(), 2);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.physics_state(id(1)).unwrap(), old);
    assert!(
        apply(
            &mut world,
            vec![StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: None
            }]
        )
        .is_err()
    );
    assert!(
        apply(
            &mut world,
            vec![StructuralCommand::SetPhysics {
                entity: id(2),
                physics: Some(physics(3, None))
            }]
        )
        .is_err()
    );
    apply(
        &mut world,
        vec![
            StructuralCommand::Spawn(entity(3, [2., 0., 0.])),
            StructuralCommand::SetPhysics {
                entity: id(3),
                physics: Some(physics(3, Some(BodyMotion::Kinematic))),
            },
            StructuralCommand::SetAngularVelocity {
                entity: id(3),
                velocity: Some(AngularVelocity {
                    angular: [0., 1., 0.],
                }),
            },
        ],
    )
    .unwrap();
    world.step().unwrap();
    assert_ne!(
        world.physics_state(id(3)).unwrap().rotation,
        [0., 0., 0., 1.]
    );
    assert_eq!(world.physics_state(id(1)).unwrap(), old);
    let mut replacement = physics(3, None);
    replacement.collider.shape = ColliderShape::Sphere { radius: 2. };
    apply(
        &mut world,
        vec![StructuralCommand::SetPhysics {
            entity: id(3),
            physics: Some(replacement),
        }],
    )
    .unwrap();
    assert_eq!(
        world.physics_state(id(3)).unwrap().angular_velocity,
        [0.; 3]
    );
    apply(
        &mut world,
        vec![
            StructuralCommand::Despawn(id(3)),
            StructuralCommand::SetPhysics {
                entity: id(1),
                physics: None,
            },
            StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            },
        ],
    )
    .unwrap();
    world.step().unwrap();
    assert!(world.physics_state(id(1)).is_none());
    assert!(world.inspect(id(3)).is_none());
    assert!((world.inspect(id(1)).unwrap().transform.translation[0] - 1. / 60.).abs() < 1e-12);
}

#[test]
fn typed_motion_input_is_checked_as_a_whole_before_solver_changes() {
    let mut world = NativeWorld::new(scene(
        vec![entity(1, [0.; 3]), entity(2, [4., 0., 0.])],
        vec![
            physics(1, Some(BodyMotion::Kinematic)),
            physics(2, Some(BodyMotion::Kinematic)),
        ],
    ));
    world.for_each_moving_mut(|id, t, v| {
        if id == StableId(1_u128.to_be_bytes()) {
            t.translation[0] = 10.;
        } else {
            v.linear[0] = 10001.;
        }
    });
    let before = world.snapshot();
    assert!(world.step().is_err());
    assert_eq!(world.tick(), 0);
    assert_eq!(world.snapshot(), before);
    world.for_each_moving_mut(|_, _, v| v.linear = [0.; 3]);
    world.step().unwrap();
    assert_eq!(
        world.physics_state(id(1)).unwrap().translation,
        [10., 0., 0.]
    );
}

#[test]
fn solver_hierarchy_overflow_never_publishes_and_is_terminal() {
    let mut child = entity(2, [f64::MAX * 0.95, f64::MAX * 0.95, 0.]);
    child.parent = Some(id(1));
    let mut rotating = physics(1, Some(BodyMotion::Kinematic));
    rotating.angular_velocity = Some(AngularVelocity {
        angular: [0., 0., 10.],
    });
    let mut world = NativeWorld::new(scene(vec![entity(1, [0.; 3]), child], vec![rotating]));
    let before = world.snapshot();
    assert!(world.step().is_err());
    assert_eq!(world.tick(), 0);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.step(), Err(RuntimeError::PhysicsFailed));
    assert_eq!(world.save_bytes(), Err(SaveError::PhysicsUnsupported));
    assert!(world.physics_state(id(1)).is_none());
}

#[test]
fn sensor_events_preserve_exact_ids_and_despawn_exit_order() {
    let mut sensor = physics(3, None);
    sensor.collider.sensor = true;
    let mut world = NativeWorld::new(scene(
        vec![entity(3, [0.; 3]), entity(1, [0.; 3]), entity(2, [0.; 3])],
        vec![sensor, physics(2, None), physics(1, None)],
    ));
    world.step().unwrap();
    let events = world.physics_events();
    assert_eq!(events.len(), 2);
    assert_eq!(
        (events[0].first, events[0].second, events[0].entered),
        (id(1), id(3), true)
    );
    assert_eq!(
        (events[1].first, events[1].second, events[1].entered),
        (id(2), id(3), true)
    );
    apply(&mut world, vec![StructuralCommand::Despawn(id(3))]).unwrap();
    world.step().unwrap();
    assert_eq!(world.physics_events().len(), 2);
    assert!(world.physics_events().iter().all(|e| !e.entered));
    world.step().unwrap();
    assert!(world.physics_events().is_empty());
}

#[test]
fn physics_saves_fail_explicitly_even_after_removal() {
    let plain = NativeWorld::new(scene(vec![entity(1, [0.; 3])], vec![]))
        .save_bytes()
        .unwrap();
    let mut world = NativeWorld::new(scene(vec![entity(1, [0.; 3])], vec![physics(1, None)]));
    assert_eq!(world.save_bytes(), Err(SaveError::PhysicsUnsupported));
    assert_eq!(
        world.restore_bytes(&plain),
        Err(SaveError::PhysicsUnsupported)
    );
    apply(
        &mut world,
        vec![StructuralCommand::SetPhysics {
            entity: id(1),
            physics: None,
        }],
    )
    .unwrap();
    assert_eq!(world.save_bytes(), Err(SaveError::PhysicsUnsupported));
}

#[test]
fn sensor_pair_overflow_is_bounded_and_cannot_publish_a_partial_tick() {
    let count = 364;
    let entities = (1..=count).map(|n| entity(n, [0.; 3])).collect();
    let declarations = (1..=count)
        .map(|n| {
            let mut p = physics(n, None);
            p.collider.sensor = true;
            p
        })
        .collect();
    let mut world = NativeWorld::new(scene(entities, declarations));
    let before = world.snapshot();
    assert!(world.step().is_err());
    assert_eq!(world.tick(), 0);
    assert_eq!(world.snapshot(), before);
    assert!(world.physics_events().is_empty());
    assert_eq!(world.step(), Err(RuntimeError::PhysicsFailed));
}

#[test]
fn angular_reset_requires_a_body_and_rejects_the_whole_batch() {
    for has_collider in [false, true] {
        let mut world = NativeWorld::new(scene(
            vec![entity(1, [0.; 3]), entity(2, [4., 0., 0.])],
            if has_collider {
                vec![physics(1, None)]
            } else {
                vec![]
            },
        ));
        let before = world.snapshot();
        for velocity in [None, Some(AngularVelocity { angular: [0.; 3] })] {
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetVelocity {
                    entity: id(2),
                    velocity: Some(Velocity {
                        linear: [5., 0., 0.],
                    }),
                })
                .unwrap();
            commands
                .push(StructuralCommand::SetAngularVelocity {
                    entity: id(1),
                    velocity,
                })
                .unwrap();
            assert!(world.apply(&mut commands).is_err());
            assert_eq!(commands.len(), 2);
            assert_eq!(world.snapshot(), before);
            assert_eq!(world.tick(), 0);
        }
    }
    let mut world = NativeWorld::new(scene(
        vec![entity(1, [0.; 3])],
        vec![physics(1, Some(BodyMotion::Dynamic))],
    ));
    assert!(
        apply(
            &mut world,
            vec![
                StructuralCommand::SetPhysics {
                    entity: id(1),
                    physics: None
                },
                StructuralCommand::SetAngularVelocity {
                    entity: id(1),
                    velocity: None
                }
            ]
        )
        .is_err()
    );
    assert!(
        world.physics_state(id(1)).is_some(),
        "failed batch retains the body"
    );
    apply(
        &mut world,
        vec![StructuralCommand::SetAngularVelocity {
            entity: id(1),
            velocity: None,
        }],
    )
    .unwrap();
    assert_eq!(
        world.physics_state(id(1)).unwrap().angular_velocity,
        [0.; 3]
    );
}

#[test]
fn mixed_motion_commands_use_final_values_and_remain_atomic() {
    let mut world = NativeWorld::new(scene(
        vec![entity(1, [0.; 3]), entity(2, [5., 0., 0.])],
        vec![
            physics(1, Some(BodyMotion::Kinematic)),
            physics(2, Some(BodyMotion::Fixed)),
        ],
    ));
    let before = states(&world, &[1, 2]);
    assert!(
        apply(
            &mut world,
            vec![
                StructuralCommand::SetAngularVelocity {
                    entity: id(1),
                    velocity: Some(AngularVelocity {
                        angular: [0., 3., 0.]
                    })
                },
                StructuralCommand::SetVelocity {
                    entity: id(2),
                    velocity: Some(Velocity {
                        linear: [1., 0., 0.]
                    })
                },
            ]
        )
        .is_err()
    );
    assert_eq!(states(&world, &[1, 2]), before);
    apply(
        &mut world,
        vec![
            StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(Velocity {
                    linear: [8., 0., 0.],
                }),
            },
            StructuralCommand::SetAngularVelocity {
                entity: id(1),
                velocity: Some(AngularVelocity {
                    angular: [0., 3., 0.],
                }),
            },
            StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(Velocity {
                    linear: [2., 0., 0.],
                }),
            },
            StructuralCommand::SetAngularVelocity {
                entity: id(1),
                velocity: None,
            },
        ],
    )
    .unwrap();
    world.step().unwrap();
    let state = world.physics_state(id(1)).unwrap();
    assert!((state.translation[0] - 2. / 60.).abs() < 1e-7);
    assert_eq!(state.angular_velocity, [0.; 3]);
    assert_eq!(state.rotation, [0., 0., 0., 1.]);
    assert_eq!(world.physics_state(id(2)).unwrap(), before[1]);
}
