use incant_nav::{
    SteeringAgent, SteeringId, SteeringObstacle, SteeringQuery, SteeringVelocity, steer,
};
use incant_types::StableId;

fn text(id: StableId) -> String {
    String::from_utf8(id.canonical_bytes().to_vec()).unwrap()
}
fn legacy(q: &SteeringQuery<StableId>) -> SteeringQuery {
    SteeringQuery {
        agents: q
            .agents
            .iter()
            .map(|a| SteeringAgent {
                id: text(a.id),
                position: a.position,
                velocity: a.velocity,
                preferred_velocity: a.preferred_velocity,
                radius: a.radius,
                height: a.height,
                max_speed: a.max_speed,
                responsibility: a.responsibility,
            })
            .collect(),
        obstacles: q
            .obstacles
            .iter()
            .map(|o| SteeringObstacle {
                id: text(o.id),
                vertices: o.vertices.clone(),
                closed: o.closed,
                min_y: o.min_y,
                max_y: o.max_y,
            })
            .collect(),
        time_horizon: q.time_horizon,
        obstacle_time_horizon: q.obstacle_time_horizon,
        margin: q.margin,
    }
}
fn query() -> SteeringQuery<StableId> {
    SteeringQuery {
        agents: [u128::MAX, 1, u128::MAX / 2, 0]
            .into_iter()
            .map(|n| SteeringAgent {
                id: StableId(n.to_be_bytes()),
                position: [0.; 3],
                velocity: [0.; 2],
                preferred_velocity: [0.; 2],
                radius: 0.25,
                height: 1.8,
                max_speed: 2.,
                responsibility: 1.,
            })
            .collect(),
        obstacles: vec![],
        time_horizon: 2.,
        obstacle_time_horizon: 2.,
        margin: 0.05,
    }
}

#[test]
fn fixed_ids_keep_exact_canonical_order_and_singular_overlap_velocities() {
    assert_eq!(text(StableId([0; 16])), "00000000000000000000000000");
    assert_eq!(text(StableId([255; 16])), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
    let mut q = query();
    for dt in [1. / 240., 1. / 60., 1.] {
        let typed = steer(&q, dt).unwrap();
        let strings = steer(&legacy(&q), dt).unwrap();
        for (a, b) in typed.iter().zip(&strings) {
            assert_eq!(text(a.id), b.id);
            assert_eq!(a.velocity, b.velocity);
            assert_eq!(a.neighbors, b.neighbors);
        }
        assert!(typed.windows(2).all(|p| p[0].id < p[1].id));
        q.agents.reverse();
        assert_eq!(steer(&q, dt).unwrap(), typed);
    }
}

#[test]
fn native_and_string_proposals_match_across_obstacles_and_successive_motion() {
    let mut q = query();
    q.agents.truncate(2);
    q.agents[0].position = [-3., 0., 0.];
    q.agents[0].preferred_velocity = [1., 0.];
    q.agents[1].position = [3., 0., 0.];
    q.agents[1].preferred_velocity = [-1., 0.];
    q.obstacles.push(SteeringObstacle {
        id: StableId(2_u128.to_be_bytes()),
        vertices: vec![[-1., -1.], [1., -1.]],
        closed: false,
        min_y: 0.,
        max_y: 2.,
    });
    for _ in 0..240 {
        let typed = steer(&q, 1. / 60.).unwrap();
        let strings = steer(&legacy(&q), 1. / 60.).unwrap();
        for (a, b) in typed.iter().zip(&strings) {
            assert_eq!(text(a.id), b.id);
            assert_eq!(a.velocity, b.velocity);
            assert_eq!(a.neighbors, b.neighbors);
        }
        for a in &mut q.agents {
            let v = typed.iter().find(|v| v.id == a.id).unwrap();
            a.velocity = v.velocity;
            a.position[0] += v.velocity[0] / 60.;
            a.position[2] += v.velocity[1] / 60.;
        }
    }
    q.agents[1].id = q.agents[0].id;
    assert!(steer(&q, 1. / 60.).is_err());
    q.agents[1].id = q.obstacles[0].id;
    assert!(steer(&q, 1. / 60.).is_err());
}

#[test]
fn generic_runtime_identity_keeps_the_existing_authoring_schemas_exact() {
    for (schema, published) in [
        (
            schemars::schema_for!(SteeringAgent),
            include_str!("../../../schemas/SteeringAgent.schema.json"),
        ),
        (
            schemars::schema_for!(SteeringObstacle),
            include_str!("../../../schemas/SteeringObstacle.schema.json"),
        ),
        (
            schemars::schema_for!(SteeringQuery),
            include_str!("../../../schemas/SteeringQuery.schema.json"),
        ),
        (
            schemars::schema_for!(SteeringVelocity),
            include_str!("../../../schemas/SteeringVelocity.schema.json"),
        ),
    ] {
        assert_eq!(
            serde_json::to_value(schema).unwrap(),
            serde_json::from_str::<serde_json::Value>(published).unwrap()
        );
    }
}
