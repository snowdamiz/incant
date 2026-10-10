use incant_nav::{SteeringAgent, SteeringObstacle, SteeringQuery, steer};

fn agent(id: usize, x: f32, z: f32) -> SteeringAgent {
    SteeringAgent {
        id: format!("{id:026}"),
        position: [x, 0., z],
        velocity: [0.; 2],
        preferred_velocity: [1., 0.],
        radius: 0.25,
        height: 1.8,
        max_speed: 2.,
        responsibility: 1.,
    }
}
fn query(agents: Vec<SteeringAgent>) -> SteeringQuery {
    SteeringQuery {
        agents,
        obstacles: vec![],
        time_horizon: 2.,
        obstacle_time_horizon: 2.,
        margin: 0.05,
    }
}
fn advance(q: &mut SteeringQuery, dt: f32) {
    let output = steer(q, dt).unwrap();
    for a in &mut q.agents {
        let v = &output.iter().find(|v| v.id == a.id).unwrap().velocity;
        a.velocity = *v;
        a.position[0] += v[0] * dt;
        a.position[2] += v[1] * dt;
    }
}
#[test]
fn free_motion_is_speed_limited_and_unrelated_floors_do_not_interfere() {
    let mut a = agent(1, 0., 0.);
    a.preferred_velocity = [6., 8.];
    let mut b = a.clone();
    b.id = agent(2, 0., 0.).id;
    b.position[1] = 3.;
    let mut q = query(vec![a, b]);
    q.agents.push(agent(3, 100., 0.));
    let out = steer(&q, 1. / 60.).unwrap();
    assert_eq!(out[0].neighbors, 0);
    assert_eq!(out[1].neighbors, 0);
    assert!((out[0].velocity[0] - 1.2).abs() < 1e-6);
    assert!((out[0].velocity[1] - 1.6).abs() < 1e-6);
    q.agents[0].max_speed = 0.;
    assert_eq!(steer(&q, 1. / 60.).unwrap()[0].velocity, [0., 0.]);
}
#[test]
fn overlapping_agents_have_repeatable_antisymmetric_escape_without_randomness() {
    for base in [0., 9999.] {
        for dt in [1. / 240., 1. / 60., 1.] {
            let mut q = query(vec![agent(1, base, base), agent(2, base, base)]);
            for a in &mut q.agents {
                a.preferred_velocity = [0.; 2];
            }
            let out = steer(&q, dt).unwrap();
            assert!(out[0].velocity[0].hypot(out[0].velocity[1]) > 0.);
            for axis in 0..2 {
                assert!((out[0].velocity[axis] + out[1].velocity[axis]).abs() < 1e-6);
            }
            for _ in 0..20 {
                assert_eq!(steer(&q, dt).unwrap(), out);
            }
            q.agents.reverse();
            assert_eq!(steer(&q, dt).unwrap(), out);
        }
    }
    // Distinct overlapping positions can also have zero relative cut-off velocity.
    let mut q = query(vec![agent(1, 0., 0.), agent(2, 0.125, 0.)]);
    q.agents[0].velocity = [7.5, 0.];
    let out = steer(&q, 1. / 60.).unwrap();
    for _ in 0..20 {
        assert_eq!(steer(&q, 1. / 60.).unwrap(), out);
    }
    // Infeasible dense overlap remains finite, bounded and deterministic.
    let q = query((1..=100).map(|i| agent(i, 0., 0.)).collect());
    let out = steer(&q, 1. / 60.).unwrap();
    assert_eq!(out.len(), 100);
    assert_eq!(out, steer(&q, 1. / 60.).unwrap());
}
#[test]
fn one_hundred_crossing_agents_keep_clearance_and_reach_opposite_goals() {
    for (stagger, rotation) in [0., 4.].into_iter().flat_map(|stagger| {
        [0., 0.0001, 0.03, 0.17, 0.7, 1.2]
            .into_iter()
            .map(move |rotation| (stagger, rotation))
    }) {
        let mut q = query(
            (0..100)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 100. + rotation;
                    agent(
                        i,
                        (20. + (i % 5) as f32 * stagger) * angle.cos(),
                        (20. + (i % 5) as f32 * stagger) * angle.sin(),
                    )
                })
                .collect(),
        );
        let goals: Vec<_> = q
            .agents
            .iter()
            .map(|a| [-a.position[0], -a.position[2]])
            .collect();
        let dt = 1. / 30.;
        let mut minimum = f32::MAX;
        for tick in 0..1500 {
            for (a, goal) in q.agents.iter_mut().zip(&goals) {
                let d = [goal[0] - a.position[0], goal[1] - a.position[2]];
                let length = d[0].hypot(d[1]);
                let speed = (length / dt).min(a.max_speed);
                a.preferred_velocity = if length < 1e-5 {
                    [0.; 2]
                } else {
                    [d[0] / length * speed, d[1] / length * speed]
                };
            }
            if tick % 300 == 0 {
                let before = steer(&q, dt).unwrap();
                let mut reversed = q.clone();
                reversed.agents.reverse();
                assert_eq!(before, steer(&reversed, dt).unwrap());
            }
            advance(&mut q, dt);
            for (i, a) in q.agents.iter().enumerate() {
                for b in &q.agents[i + 1..] {
                    let distance =
                        (a.position[0] - b.position[0]).hypot(a.position[2] - b.position[2]);
                    minimum = minimum.min(distance);
                    assert!(
                        distance >= a.radius + b.radius - 0.002,
                        "tick {tick}: separation {distance}"
                    );
                }
            }
        }
        let remaining = q
            .agents
            .iter()
            .zip(&goals)
            .map(|(a, g)| (a.position[0] - g[0]).hypot(a.position[2] - g[1]))
            .fold(0., f32::max);
        eprintln!("100 agents: min separation={minimum}, max goal distance={remaining}");
        if remaining >= 0.1 {
            for (a, g) in q.agents.iter().zip(&goals).take(10) {
                eprintln!(
                    "stalled sample: {} pos {:?} velocity {:?} preferred {:?} goal {:?}",
                    a.id, a.position, a.velocity, a.preferred_velocity, g
                );
            }
        }
        assert!(
            remaining < 0.1,
            "stagger {stagger}, rotation {rotation}: remaining {remaining}"
        );
    }
}

fn wall() -> SteeringObstacle {
    SteeringObstacle {
        id: agent(999, 0., 0.).id,
        vertices: vec![[0., -5.], [1., -5.], [1., 5.], [0., 5.]],
        closed: true,
        min_y: 0.,
        max_y: 3.,
    }
}
#[test]
fn obstacles_stop_motion_at_radius_and_height_filtering_preserves_stacked_motion() {
    let mut q = query(vec![agent(1, -3., 0.)]);
    q.obstacles.push(wall());
    for _ in 0..600 {
        advance(&mut q, 1. / 60.);
        assert!(q.agents[0].position[0] <= -0.249);
    }
    assert!(q.agents[0].position[0] > -0.33);
    q.agents[0].position = [-3., 4., 0.];
    assert_eq!(steer(&q, 1. / 60.).unwrap()[0].velocity, [1., 0.]);
    // Clockwise winding encloses the agent; open segments block only their left.
    q.agents[0].position = [0.5, 0., 0.];
    q.obstacles[0].vertices.reverse();
    for _ in 0..300 {
        advance(&mut q, 1. / 60.);
        assert!(q.agents[0].position[0] <= 0.751);
    }
    q.obstacles[0].closed = false;
    q.obstacles[0].vertices = vec![[0., 5.], [0., -5.]];
    q.agents[0].position = [-1., 0., 0.];
    q.agents[0].velocity = [0.; 2];
    assert!(steer(&q, 1. / 60.).unwrap()[0].velocity[0] < 1.);
    q.agents[0].position = [1., 0., 0.];
    assert_eq!(steer(&q, 1. / 60.).unwrap()[0].velocity, [1., 0.]);
}
#[test]
fn invalid_or_unbounded_requests_fail_before_producing_any_batch() {
    let valid = query(vec![agent(1, 0., 0.)]);
    for dt in [0., f32::NAN, 1. / 1000., 2.] {
        assert!(steer(&valid, dt).is_err());
    }
    let mut q = valid.clone();
    q.agents.push(q.agents[0].clone());
    assert!(steer(&q, 1. / 60.).is_err());
    q = valid.clone();
    q.agents[0].position[0] = f32::INFINITY;
    assert!(steer(&q, 1. / 60.).is_err());
    q = valid.clone();
    q.agents[0].responsibility = 0.;
    assert!(steer(&q, 1. / 60.).is_err());
    q = valid.clone();
    q.agents = (0..129).map(|i| agent(i, 0., 0.)).collect();
    assert!(steer(&q, 1. / 60.).is_err());
    for vertices in [
        vec![[0., 0.], [1., 0.], [0., 0.]],
        vec![[0., 0.], [1., 1.], [0., 1.], [1., 0.]],
        vec![[0., 0.], [2., 0.], [1., 0.5], [2., 1.], [0., 1.]],
        vec![[0., 0.], [1., 0.], [2., 0.], [2., 1.], [0., 1.]],
    ] {
        q = valid.clone();
        let mut obstacle = wall();
        obstacle.vertices = vertices;
        q.obstacles.push(obstacle);
        assert!(steer(&q, 1. / 60.).is_err());
    }
}
