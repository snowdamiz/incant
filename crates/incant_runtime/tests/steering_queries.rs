#[path = "support/navigation_queries.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::*;
use incant_runtime::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn request() -> SteeringQuery {
    SteeringQuery {
        agents: vec![SteeringAgent {
            id: id(100),
            position: [0.; 3],
            velocity: [0.; 2],
            preferred_velocity: [3., 4.],
            radius: 0.25,
            height: 1.8,
            max_speed: 2.,
            responsibility: 1.,
        }],
        obstacles: vec![],
        time_horizon: 2.,
        obstacle_time_horizon: 2.,
        margin: 0.05,
    }
}

#[test]
fn pure_proposals_do_not_mutate_world_and_retained_capability_never_reactivates() {
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    let q = request();
    let mut retained = None;
    for panic in [false, true] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            world
                .with_numeric_physics(|_, queries| {
                    let result = queries.steer_agents(&q, 1. / 60.).unwrap();
                    assert_eq!(result[0].id, id(100)); // sample identity need not be an entity
                    assert!((result[0].velocity[0] - 1.2).abs() < 1e-6);
                    assert!((result[0].velocity[1] - 1.6).abs() < 1e-6);
                    retained = Some(queries.clone());
                    for dt in [f64::NAN, f64::INFINITY, 0., 1. / 241., 1.00001] {
                        assert!(queries.steer_agents(&q, dt).is_err());
                    }
                    if panic {
                        panic!("query unwind");
                    }
                    result
                })
                .unwrap()
        }));
        assert_eq!(result.is_err(), panic);
        let old = retained.take().unwrap();
        assert!(matches!(
            old.steer_agents(&q, 1. / 60.),
            Err(QueryError::Expired)
        ));
        world
            .with_numeric_physics(|_, fresh| {
                assert!(matches!(
                    old.steer_agents(&q, 1. / 60.),
                    Err(QueryError::Expired)
                ));
                assert!(fresh.steer_agents(&q, 1. / 60.).is_ok());
            })
            .unwrap();
        assert_eq!(world.snapshot(), before);
    }
}
