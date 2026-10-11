use incant_physics_2d_runtime::{BodyState, PhysicsError, PhysicsWorld2D, PreparedBody, RayQuery};
use incant_types::{BodyMotion, Collider2D, ColliderShape2D, RigidBody, StableId};

fn id(value: u8) -> StableId {
    StableId([value; 16])
}
fn state(x: f64, y: f64) -> BodyState {
    BodyState {
        translation: [x, y],
        ..Default::default()
    }
}
fn body(motion: BodyMotion) -> RigidBody {
    RigidBody {
        motion,
        ..Default::default()
    }
}
fn declaration(motion: Option<BodyMotion>, collider: Collider2D, state: BodyState) -> PreparedBody {
    PreparedBody::new(motion.map(body), collider, state).unwrap()
}
fn box_shape(x: f64, y: f64) -> Collider2D {
    Collider2D {
        shape: ColliderShape2D::Box {
            half_extents: [x, y],
        },
        ..Default::default()
    }
}
fn ray(x: f64, y: f64) -> RayQuery {
    RayQuery {
        origin: [x, y],
        direction: [0., -1.],
        max_distance: 100.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}
fn tilemap() -> incant_2d::tilemap::PreparedTilemap<StableId> {
    incant_2d::Tilemap {
        tileset: id(20),
        size: [3, 2],
        cell_size: [1., 1.],
        layers: vec![incant_2d::TilemapLayer {
            id: id(21),
            cells: vec![
                Some(id(22)),
                None,
                Some(id(22)),
                Some(id(22)),
                None,
                Some(id(22)),
            ],
            layer: 0,
            order: 0,
            solid: true,
        }],
    }
    .prepare()
    .unwrap()
}

#[test]
fn collision_settles_and_repeated_unchanged_motion_preserves_sleep() {
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![
            (
                id(1),
                Some(declaration(None, box_shape(10., 0.5), state(0., -0.5))),
            ),
            (
                id(2),
                Some(declaration(
                    Some(BodyMotion::Dynamic),
                    Collider2D::default(),
                    state(0., 3.),
                )),
            ),
        ])
        .unwrap();
    for _ in 0..600 {
        let previous = world.state(id(2)).unwrap().clone();
        world.sync_motion(&[(id(2), previous.clone())]).unwrap();
        world.step(1. / 64.).unwrap();
    }
    let resting = world.state(id(2)).unwrap().clone();
    assert!((resting.translation[1] - 0.5).abs() < 0.02, "{resting:?}");
    assert!(resting.sleeping, "{resting:?}");
    for _ in 0..64 {
        world.sync_motion(&[(id(2), resting.clone())]).unwrap();
        world.step(1. / 64.).unwrap();
        assert_eq!(world.state(id(2)), Some(&resting));
    }
    let mut moved = resting.clone();
    moved.translation[1] += 2.;
    world.sync_motion(&[(id(2), moved.clone())]).unwrap();
    world.step(1. / 64.).unwrap();
    assert!(!world.state(id(2)).unwrap().sleeping);
    assert!(world.state(id(2)).unwrap().translation[1] < moved.translation[1]);
}

#[test]
fn tile_union_has_real_hole_and_collision_survives_pre_step_raycast() {
    let map = tilemap();
    assert_eq!(map.collision().len(), 2);
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![
            (
                id(1),
                Some(PreparedBody::tilemap(&map, state(0., 0.)).unwrap()),
            ),
            (
                id(2),
                Some(declaration(
                    Some(BodyMotion::Dynamic),
                    Collider2D {
                        shape: ColliderShape2D::Circle { radius: 0.2 },
                        ..Default::default()
                    },
                    state(0.5, 4.),
                )),
            ),
            (
                id(3),
                Some(declaration(
                    Some(BodyMotion::Dynamic),
                    Collider2D {
                        shape: ColliderShape2D::Circle { radius: 0.2 },
                        ..Default::default()
                    },
                    state(1.5, 4.),
                )),
            ),
        ])
        .unwrap();
    let mut q = ray(0.5, 3.);
    q.exclude_entity = Some(id(2));
    assert_eq!(world.raycast(&q).unwrap().unwrap().entity, id(1));
    let mut q = ray(1.5, 3.);
    q.exclude_entity = Some(id(3));
    assert!(world.raycast(&q).unwrap().is_none());
    for _ in 0..128 {
        world.step(1. / 64.).unwrap();
    }
    assert!((world.state(id(2)).unwrap().translation[1] - 2.2).abs() < 0.02);
    assert!(world.state(id(3)).unwrap().translation[1] < -10.);
    // The BVH path after stepping preserves the same hole.
    let mut q = ray(1.5, 3.);
    q.exclude_entity = Some(id(3));
    assert!(world.raycast(&q).unwrap().is_none());
}

#[test]
fn trigger_transitions_are_sorted_deduplicated_and_exit_after_removal() {
    let mut world = PhysicsWorld2D::default();
    let sensor = Collider2D {
        sensor: true,
        ..Default::default()
    };
    world
        .apply(vec![
            (
                id(3),
                Some(declaration(None, sensor.clone(), state(0., 0.))),
            ),
            (id(1), Some(declaration(None, sensor, state(0., 0.)))),
            (
                id(2),
                Some(declaration(None, Collider2D::default(), state(0., 0.))),
            ),
        ])
        .unwrap();
    world.step(1. / 64.).unwrap();
    assert_eq!(
        world
            .events()
            .iter()
            .map(|e| (e.first, e.second, e.entered))
            .collect::<Vec<_>>(),
        vec![
            (id(1), id(2), true),
            (id(1), id(3), true),
            (id(2), id(3), true)
        ]
    );
    world.step(1. / 64.).unwrap();
    assert!(world.events().is_empty());
    world.apply(vec![(id(3), None)]).unwrap();
    world.step(1. / 64.).unwrap();
    assert_eq!(
        world
            .events()
            .iter()
            .map(|e| (e.first, e.second, e.entered))
            .collect::<Vec<_>>(),
        vec![(id(1), id(3), false), (id(2), id(3), false)]
    );
}

#[test]
fn ray_filter_sensor_exclusion_and_stable_tie_work_before_and_after_step() {
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![
            (
                id(3),
                Some(declaration(
                    None,
                    Collider2D {
                        sensor: true,
                        ..Default::default()
                    },
                    state(0., 2.),
                )),
            ),
            (
                id(2),
                Some(declaration(
                    None,
                    Collider2D {
                        memberships: 2,
                        filter: 4,
                        ..Default::default()
                    },
                    state(0., 0.),
                )),
            ),
            (
                id(1),
                Some(declaration(
                    None,
                    Collider2D {
                        memberships: 2,
                        filter: 4,
                        ..Default::default()
                    },
                    state(0., 0.),
                )),
            ),
        ])
        .unwrap();
    for stepped in [false, true] {
        if stepped {
            world.step(1. / 64.).unwrap();
        }
        let mut q = ray(0., 4.);
        q.memberships = 4;
        q.filter = 2;
        let hit = world.raycast(&q).unwrap().unwrap();
        assert_eq!(hit.entity, id(1));
        assert_eq!(hit.distance, 3.5);
        assert_eq!(hit.normal, [0., 1.]);
        q.exclude_entity = Some(id(1));
        assert_eq!(world.raycast(&q).unwrap().unwrap().entity, id(2));
        q.memberships = 1;
        assert!(world.raycast(&q).unwrap().is_none());
        q.include_sensors = true;
        assert_eq!(world.raycast(&q).unwrap().unwrap().entity, id(3));
    }
}

#[test]
fn invalid_late_motion_does_not_publish_earlier_motion_and_is_retryable() {
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![
            (
                id(1),
                Some(declaration(
                    Some(BodyMotion::Kinematic),
                    Collider2D::default(),
                    state(0., 0.),
                )),
            ),
            (
                id(2),
                Some(declaration(None, Collider2D::default(), state(2., 0.))),
            ),
        ])
        .unwrap();
    let before = world.state(id(1)).unwrap().clone();
    let moved = state(7., 0.);
    let invalid = BodyState {
        velocity: [1., 0.],
        ..state(2., 0.)
    };
    assert!(
        world
            .sync_motion(&[(id(1), moved.clone()), (id(2), invalid.clone())])
            .is_err()
    );
    assert_eq!(world.state(id(1)), Some(&before));
    world.sync_motion(&[(id(1), moved.clone())]).unwrap();
    assert_eq!(world.state(id(1)).unwrap().translation, [7., 0.]);
}

#[test]
fn duplicate_structural_batch_is_atomic_and_replacement_keeps_exact_id() {
    let mut world = PhysicsWorld2D::default();
    world
        .apply(vec![(
            id(1),
            Some(declaration(None, Collider2D::default(), state(0., 0.))),
        )])
        .unwrap();
    assert_eq!(
        world.apply(vec![(id(1), None), (id(1), None)]),
        Err(PhysicsError::Limit)
    );
    assert!(world.state(id(1)).is_some());
    world
        .apply(vec![(
            id(1),
            Some(declaration(
                None,
                Collider2D {
                    shape: ColliderShape2D::Circle { radius: 2. },
                    ..Default::default()
                },
                state(0., 0.),
            )),
        )])
        .unwrap();
    assert_eq!(world.raycast(&ray(0., 4.)).unwrap().unwrap().distance, 2.);
    world.apply(vec![(id(1), None)]).unwrap();
    assert!(world.raycast(&ray(0., 4.)).unwrap().is_none());
}

#[test]
fn kinematic_capsule_advances_linear_and_angular_state_once() {
    let mut world = PhysicsWorld2D::default();
    let initial = BodyState {
        translation: [2., 3.],
        velocity: [4., 0.],
        angular_velocity: 2.,
        ..Default::default()
    };
    world
        .apply(vec![(
            id(1),
            Some(declaration(
                Some(BodyMotion::Kinematic),
                Collider2D {
                    shape: ColliderShape2D::Capsule {
                        half_height: 0.5,
                        radius: 0.25,
                    },
                    ..Default::default()
                },
                initial,
            )),
        )])
        .unwrap();
    world.step(1. / 64.).unwrap();
    let actual = world.state(id(1)).unwrap();
    assert_eq!(actual.translation, [2.0625, 3.]);
    // Pinned Rapier f32 angle integration differs by 6.36e-7 radians here.
    assert!((actual.rotation - 0.03125).abs() < 1e-6, "{actual:?}");
    assert_eq!(actual.velocity, [4., 0.]);
    assert_eq!(actual.angular_velocity, 2.);
}

#[test]
fn numerical_step_failure_is_terminal_but_bad_dt_is_retryable() {
    let mut world = PhysicsWorld2D::default();
    let initial = BodyState {
        translation: [1_000_000., 0.],
        velocity: [10000., 0.],
        ..Default::default()
    };
    world
        .apply(vec![(
            id(1),
            Some(declaration(
                Some(BodyMotion::Kinematic),
                Collider2D::default(),
                initial.clone(),
            )),
        )])
        .unwrap();
    assert_eq!(world.step(f64::NAN), Err(PhysicsError::InvalidStep));
    assert_eq!(world.state(id(1)), Some(&initial));
    assert_eq!(world.step(1. / 64.), Err(PhysicsError::Failed));
    assert!(world.state(id(1)).is_none());
    assert_eq!(world.states().count(), 0);
    assert!(world.events().is_empty());
    assert_eq!(world.raycast(&ray(0., 1.)), Err(PhysicsError::Failed));
    assert_eq!(world.apply(vec![]), Err(PhysicsError::Failed));
}

#[test]
fn malformed_geometry_material_and_fixed_motion_are_refused() {
    for collider in [
        Collider2D {
            shape: ColliderShape2D::Circle { radius: f64::NAN },
            ..Default::default()
        },
        Collider2D {
            shape: ColliderShape2D::Capsule {
                half_height: -1.,
                radius: 1.,
            },
            ..Default::default()
        },
        Collider2D {
            density: f64::INFINITY,
            ..Default::default()
        },
        Collider2D {
            friction: -1.,
            ..Default::default()
        },
        Collider2D {
            restitution: 1.1,
            ..Default::default()
        },
    ] {
        assert!(PreparedBody::new(None, collider, BodyState::default()).is_err());
    }
    assert!(
        PreparedBody::new(
            None,
            Collider2D::default(),
            BodyState {
                angular_velocity: 1.,
                ..Default::default()
            }
        )
        .is_err()
    );
    let world = PhysicsWorld2D::default();
    let mut q = ray(0., 1.);
    q.direction = [0.; 2];
    assert_eq!(world.raycast(&q), Err(PhysicsError::Query));
}
