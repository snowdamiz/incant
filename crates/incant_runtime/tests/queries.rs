use incant_runtime::{
    CookedEntity, CookedPhysics, CookedScene, NativeWorld, QueryError, RayQuery, StableId,
};
use incant_types::{BodyMotion, Collider, ColliderShape, RigidBody, Transform, Velocity};
use std::panic::{AssertUnwindSafe, catch_unwind};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            60,
            vec![CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform {
                    translation: [5., 0., 0.],
                    ..Default::default()
                },
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            }],
        )
        .unwrap()
        .with_physics(vec![CookedPhysics {
            entity: id(1),
            body: Some(RigidBody {
                motion: BodyMotion::Kinematic,
                ..Default::default()
            }),
            collider: Collider {
                shape: ColliderShape::Box {
                    half_extents: [0.5; 3],
                },
                ..Default::default()
            },
            angular_velocity: None,
        }])
        .unwrap(),
    )
}
fn ray() -> RayQuery<StableId> {
    RayQuery {
        scene_id: id(99),
        origin: [0.; 3],
        direction: [2., 0., 0.],
        max_distance: 100.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}

#[test]
fn queries_are_committed_revocable_and_do_not_change_solver_continuation() {
    let mut queried = world();
    let mut control = world();
    let query = ray();
    let old = queried
        .with_numeric_physics(|columns, queries| {
            columns.for_each_chunk(|chunk| {
                assert_eq!(queries.raycast(&query).unwrap().unwrap().distance, 4.5);
                chunk.transforms[0] = 50.;
                assert_eq!(queries.raycast(&query).unwrap().unwrap().distance, 4.5);
                chunk.transforms[0] = 5.;
            });
            queries
        })
        .unwrap();
    assert!(matches!(old.raycast(&query), Err(QueryError::Expired)));
    for _ in 0..120 {
        queried.step().unwrap();
        control.step().unwrap();
        queried
            .with_numeric_physics(|_, live| {
                assert!(live.raycast(&query).unwrap().is_some());
                assert!(matches!(old.raycast(&query), Err(QueryError::Expired)));
            })
            .unwrap();
        assert_eq!(queried.physics_state(id(1)), control.physics_state(id(1)));
    }
}
#[test]
fn error_and_unwind_restore_the_same_solver_and_expire_every_retained_handle() {
    let mut actual = world();
    let mut baseline = world();
    let mut retained = None;
    let result = actual
        .with_numeric_physics(|_, queries| {
            retained = Some(queries);
            Err::<(), _>("refused")
        })
        .unwrap();
    assert_eq!(result, Err("refused"));
    assert!(matches!(
        retained.take().unwrap().raycast(&ray()),
        Err(QueryError::Expired)
    ));
    assert!(
        catch_unwind(AssertUnwindSafe(|| actual.with_numeric_physics(
            |columns, queries| {
                retained = Some(queries);
                columns.for_each_chunk(|chunk| {
                    chunk.velocities.unwrap()[0] = 1.;
                });
                panic!("test cleanup");
            }
        )))
        .is_err()
    );
    assert!(matches!(
        retained.unwrap().raycast(&ray()),
        Err(QueryError::Expired)
    ));
    actual.step().unwrap();
    baseline.step().unwrap();
    assert_eq!(actual.physics_state(id(1)), baseline.physics_state(id(1)));
}
#[test]
fn empty_world_still_checks_invalid_queries_and_refuses_another_scene() {
    let mut world = NativeWorld::new(CookedScene::new(id(99), 60, vec![]).unwrap());
    world
        .with_numeric_physics(|_, queries| {
            let mut request = ray();
            assert!(queries.raycast(&request).unwrap().is_none());
            request.direction = [0.; 3];
            assert!(matches!(
                queries.raycast(&request),
                Err(QueryError::Physics(_))
            ));
            request = ray();
            request.scene_id = id(77);
            assert!(matches!(queries.raycast(&request), Err(QueryError::Scene)));
        })
        .unwrap();
    world.step().unwrap();
}
