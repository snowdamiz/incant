//! A real bounded ORCA batch, including the overlapping-agent singular case.
use incant_core::{Engine, SteeringAgent, SteeringQuery};
use incant_doc::Project;
use serde_json::{Value, json};

pub(super) fn check() -> Result<Value, String> {
    let engine = Engine::new(&Project::empty("Steering smoke")).map_err(|e| e.to_string())?;
    let solve = engine.steerer();
    let mut query = SteeringQuery {
        agents: (0..100)
            .map(|i| SteeringAgent {
                id: format!("{i:026}"),
                position: [(i % 10) as f32 - 4.5, 0., (i / 10) as f32 - 4.5],
                velocity: [0.; 2],
                preferred_velocity: [1., 0.],
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
    };
    query.agents[1].position = query.agents[0].position;
    let before = solve(query.clone()).map_err(|e| e.to_string())?;
    query.agents.reverse();
    let after = solve(query).map_err(|e| e.to_string())?;
    if before != after
        || before.len() != 100
        || before.iter().any(|v| {
            !v.velocity.iter().all(|x| x.is_finite())
                || v.velocity[0].hypot(v.velocity[1]) > 2.000_001
        })
        || !before.iter().any(|v| v.neighbors > 0)
    {
        return Err("100-agent steering was invalid or input-order dependent".into());
    }
    Ok(
        json!({"agents":100,"stable_id_order":true,"overlap_recovery_finite":true,"visual_gate":false}),
    )
}
