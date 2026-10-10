//! Numerical regression of Claude's unchanged 0027 four-way plaza course.
//! Initial IDs/positions/goals come from rev2-A; routing matches steering_plaza.ts.
//! This runs no renderer and does not substitute for the actual public play review.
use glam::DVec2;
use incant_nav::{SteeringAgent, SteeringObstacle, SteeringQuery, steer};
use serde::Deserialize;

#[derive(Deserialize)]
struct Walker {
    id: String,
    position: [f64; 2],
    goal: [f64; 2],
}
fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let edge = b - a;
    let t = if edge.length_squared() == 0. {
        0.
    } else {
        ((p - a).dot(edge) / edge.length_squared()).clamp(0., 1.)
    };
    (p - a - edge * t).length()
}
fn preferred(p: DVec2, goal: DVec2) -> [f32; 2] {
    let to = goal - p;
    let distance = to.length();
    if distance < 1e-6 {
        return [0.; 2];
    }
    let mut direction = to / distance;
    let center = -p;
    let d = center.length();
    if to.dot(center) > 0. && distance_to_segment(DVec2::ZERO, p, goal) < 1.55 && d > 1e-6 {
        direction = if d > 1.55 {
            let beta = (1.55 / d).asin();
            let base = center.y.atan2(center.x);
            let candidates = [base + beta, base - beta].map(DVec2::from_angle);
            candidates
                .into_iter()
                .find(|u| u.perp_dot(center) > 0.)
                .unwrap_or(candidates[0])
        } else {
            let t = DVec2::new(center.y, -center.x) / d;
            if t.perp_dot(center) > 0. { t } else { -t }
        };
    }
    (direction * (distance * 60.).min(1.4)).as_vec2().to_array()
}
fn square(x: f64, z: f64, hx: f64, hz: f64) -> Vec<[f32; 2]> {
    [
        [x - hx, z - hz],
        [x + hx, z - hz],
        [x + hx, z + hz],
        [x - hx, z + hz],
    ]
    .map(|p| p.map(|v| v as f32))
    .into()
}
#[test]
fn plaza_crossing_reaches_and_retains_all_goals_without_body_or_obstacle_overlap() {
    let mut walkers: Vec<Walker> =
        serde_json::from_str(include_str!("fixtures/plaza-walkers.json")).unwrap();
    let mut query = SteeringQuery {
        agents: walkers
            .iter()
            .map(|w| SteeringAgent {
                id: w.id.clone(),
                position: [w.position[0] as f32, 0., w.position[1] as f32],
                velocity: [0.; 2],
                preferred_velocity: [0.; 2],
                radius: 0.25,
                height: 1.8,
                max_speed: 1.4,
                responsibility: 1.,
            })
            .collect(),
        obstacles: [
            (
                0.,
                0.9,
                (0..8)
                    .map(|i| {
                        DVec2::from_angle(
                            std::f64::consts::FRAC_PI_8 + i as f64 * std::f64::consts::FRAC_PI_4,
                        )
                        .as_vec2()
                        .to_array()
                    })
                    .collect(),
            ),
            (0., 2.35, square(-5.5, -4.5, 0.15, 0.15)),
            (0., 2.35, square(-5.5, 4.5, 0.15, 0.15)),
            (2.1, 2.35, square(-5.5, 0., 0.15, 4.65)),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (min_y, max_y, vertices))| SteeringObstacle {
            id: format!("{:026}", 1000 + i),
            vertices,
            closed: true,
            min_y,
            max_y,
        })
        .collect(),
        time_horizon: 2.,
        obstacle_time_horizon: 1.,
        margin: 0.05,
    };
    let mut minimum = f64::MAX;
    let mut remaining = 0_f64;
    let mut arrived = 0;
    for tick in 0..2700 {
        for (walker, agent) in walkers.iter().zip(&mut query.agents) {
            agent.position = [walker.position[0] as f32, 0., walker.position[1] as f32];
            agent.preferred_velocity = preferred(walker.position.into(), walker.goal.into());
        }
        let output = steer(&query, 1. / 60.).unwrap();
        for ((walker, agent), proposal) in walkers.iter_mut().zip(&mut query.agents).zip(output) {
            assert_eq!(walker.id, proposal.id);
            agent.velocity = proposal.velocity;
            for axis in 0..2 {
                // Engine Transform integration is f64; steering inputs/outputs are f32.
                walker.position[axis] += f64::from(proposal.velocity[axis]) / 60.;
            }
        }
        remaining = 0.;
        arrived = 0;
        for (i, a) in walkers.iter().enumerate() {
            let p = DVec2::from_array(a.position);
            let distance = (p - DVec2::from_array(a.goal)).length();
            remaining = remaining.max(distance);
            arrived += usize::from(distance < 0.05);
            for b in &walkers[i + 1..] {
                let separation = (p - DVec2::from_array(b.position)).length();
                minimum = minimum.min(separation);
                assert!(
                    separation >= 0.5,
                    "tick {tick}: body separation {separation}"
                );
            }
            for obstacle in &query.obstacles[..3] {
                let vertices: Vec<_> = obstacle
                    .vertices
                    .iter()
                    .map(|v| DVec2::new(v[0].into(), v[1].into()))
                    .collect();
                let mut inside = true;
                let mut distance = f64::MAX;
                for (a, b) in vertices
                    .iter()
                    .zip(vertices.iter().cycle().skip(1))
                    .take(vertices.len())
                {
                    inside &= (*b - *a).perp_dot(p - *a) >= 0.;
                    distance = distance.min(distance_to_segment(p, *a, *b));
                }
                assert!(
                    !inside && distance >= 0.249,
                    "tick {tick}: obstacle distance {distance}"
                );
            }
        }
        if tick >= 2640 {
            assert_eq!(arrived, 100, "tick {tick}: remaining {remaining}");
        }
    }
    eprintln!("plaza: arrived {arrived}, max remaining {remaining}, min separation {minimum}");
}
