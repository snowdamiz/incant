//! Run with `cargo run -p incant_nav --example steering_budget --release`.
use incant_nav::{SteeringAgent, SteeringObstacle, SteeringQuery, steer};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn main() {
    let mut cases = vec![];
    for (name, count, coincident, obstacles) in [
        ("crossing_100", 100, false, false),
        ("overlap_100", 100, true, false),
        ("limit_128_agents_128_edges", 128, true, true),
    ] {
        let query = SteeringQuery {
            agents: (0..count)
                .map(|i| SteeringAgent {
                    id: format!("{i:026}"),
                    position: if coincident {
                        [0.; 3]
                    } else {
                        [(i % 10) as f32 - 4.5, 0., (i / 10) as f32 - 4.5]
                    },
                    velocity: [0.; 2],
                    preferred_velocity: [if i % 2 == 0 { 2. } else { -2. }, 0.],
                    radius: 0.25,
                    height: 1.8,
                    max_speed: 2.,
                    responsibility: 1.,
                })
                .collect(),
            obstacles: if obstacles {
                (0..32)
                    .map(|i| {
                        let x = (i % 8) as f32 * 3. - 12.;
                        let z = (i / 8) as f32 * 3. - 6.;
                        SteeringObstacle {
                            id: format!("{:026}", i + 1000),
                            vertices: vec![[x, z], [x + 1., z], [x + 1., z + 1.], [x, z + 1.]],
                            closed: true,
                            min_y: 0.,
                            max_y: 3.,
                        }
                    })
                    .collect()
            } else {
                vec![]
            },
            time_horizon: 2.,
            obstacle_time_horizon: 2.,
            margin: 0.05,
        };
        let mut times = vec![];
        for i in 0..120 {
            let start = Instant::now();
            black_box(steer(black_box(&query), 1. / 60.).expect("valid benchmark batch"));
            let ms = start.elapsed().as_secs_f64() * 1000.;
            if i >= 20 {
                times.push(ms);
            }
        }
        times.sort_by(f64::total_cmp);
        cases.push(json!({"name":name,"agents":count,"obstacle_edges":if obstacles {128}else{0},"samples":times.len(),"p50_ms":times[49],"p95_ms":times[94],"max_ms":times[99]}));
    }
    println!(
        "{}",
        json!({"arch":std::env::consts::ARCH,"os":std::env::consts::OS,"cases":cases,"device_gate":false})
    );
}
