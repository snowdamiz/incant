#[path = "support/physics_queries_2d.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn planar_queries_see_committed_tiles_and_filters_and_restore_both_solvers() {
    let mut world = NativeWorld::new(scene());
    let mut control = NativeWorld::new(scene());
    let old = world
        .with_numeric_physics(|columns, cap| {
            let query = ray();
            let hit = cap.raycast_2d(&query).unwrap().unwrap();
            assert_eq!(hit.entity, id(1));
            assert_eq!(hit.distance, 2.);
            assert_eq!(hit.point, [0.5, 1.]);
            assert_eq!(hit.normal, [0., 1.]);
            assert!(
                cap.raycast_2d(&RayQuery2D {
                    origin: [1.5, 3.],
                    ..query.clone()
                })
                .unwrap()
                .is_none()
            );
            assert!(
                cap.raycast_2d(&RayQuery2D {
                    filter: 0,
                    ..query.clone()
                })
                .unwrap()
                .is_none()
            );
            assert!(
                cap.raycast_2d(&RayQuery2D {
                    memberships: 0,
                    ..query.clone()
                })
                .unwrap()
                .is_none()
            );
            assert_eq!(
                cap.raycast_2d(&RayQuery2D {
                    include_sensors: true,
                    ..query.clone()
                })
                .unwrap()
                .unwrap()
                .entity,
                id(3)
            );
            columns.for_each_chunk(|chunk| {
                let Some(i) = chunk.ids.iter().position(|&id| id == fixture::id(1)) else {
                    return;
                };
                chunk.transforms[i * TRANSFORM_STRIDE + 1] = 100.;
                assert_eq!(cap.raycast_2d(&query).unwrap().unwrap(), hit);
                chunk.transforms[i * TRANSFORM_STRIDE + 1] = 0.;
            });
            columns.validate().unwrap();
            cap
        })
        .unwrap();
    assert!(matches!(old.raycast_2d(&ray()), Err(QueryError::Expired)));
    assert!(matches!(old.trigger_events_2d(), Err(QueryError::Expired)));
    for _ in 0..120 {
        world.step().unwrap();
        control.step().unwrap();
        world
            .with_numeric_physics(|_, cap| {
                assert!(cap.raycast_2d(&ray()).unwrap().is_some());
                assert!(matches!(old.trigger_events_2d(), Err(QueryError::Expired)));
            })
            .unwrap();
        assert_eq!(world.snapshot(), control.snapshot());
        assert_eq!(
            world.physics_2d_state(id(2)),
            control.physics_2d_state(id(2))
        );
        assert_eq!(world.physics_state(id(4)), control.physics_state(id(4)));
        assert_eq!(world.physics_2d_events(), control.physics_2d_events());
    }
}

#[test]
fn error_and_unwind_restore_both_solvers_and_revoke_every_planar_handle() {
    let mut world = NativeWorld::new(scene());
    let mut control = NativeWorld::new(scene());
    let mut retained = None;
    let result = world
        .with_numeric_physics(|_, cap| {
            retained = Some(cap);
            Err::<(), _>("refused")
        })
        .unwrap();
    assert_eq!(result, Err("refused"));
    assert!(matches!(
        retained.take().unwrap().raycast_2d(&ray()),
        Err(QueryError::Expired)
    ));
    assert!(
        catch_unwind(AssertUnwindSafe(|| world.with_numeric_physics(
            |columns, cap| {
                retained = Some(cap);
                columns.for_each_chunk(|_| ());
                panic!("cleanup");
            }
        )))
        .is_err()
    );
    assert!(matches!(
        retained.unwrap().trigger_events_2d(),
        Err(QueryError::Expired)
    ));
    world.step().unwrap();
    control.step().unwrap();
    assert_eq!(world.snapshot(), control.snapshot());
    assert_eq!(
        world.physics_2d_state(id(2)),
        control.physics_2d_state(id(2))
    );
    assert_eq!(world.physics_state(id(4)), control.physics_state(id(4)));
    assert_eq!(world.physics_2d_events(), control.physics_2d_events());
}

#[test]
fn events_are_owned_nondraining_and_empty_world_still_validates_rays() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let before = world.physics_2d_events().to_vec();
    assert_eq!(
        before,
        vec![TriggerEvent2D {
            first: id(2),
            second: id(3),
            entered: true
        }]
    );
    world
        .with_numeric_physics(|_, cap| {
            let mut events = cap.trigger_events_2d().unwrap();
            events[0].entered = false;
            assert_eq!(cap.trigger_events_2d().unwrap(), before);
        })
        .unwrap();
    let mut empty = NativeWorld::new(CookedScene::new(id(99), 64, vec![]).unwrap());
    empty
        .with_numeric_physics(|_, cap| {
            assert!(cap.trigger_events_2d().unwrap().is_empty());
            assert!(cap.raycast_2d(&ray()).unwrap().is_none());
            assert!(matches!(
                cap.raycast_2d(&RayQuery2D {
                    direction: [0.; 2],
                    ..ray()
                }),
                Err(QueryError::Physics2D(_))
            ));
        })
        .unwrap();
}
