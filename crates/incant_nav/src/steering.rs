//! Stateless reciprocal local avoidance. Output velocities are proposals; the
//! caller applies ordinary movement commands and remains responsible for physics.
use crate::{NavigationError, invalid, limit};
use dodgy_2d::{Agent, AvoidanceOptions, Obstacle, Vec2};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SteeringAgent {
    pub id: String,
    /// World-space feet, with +Y up. Avoidance operates in the horizontal XZ plane.
    pub position: [f32; 3],
    pub velocity: [f32; 2],
    pub preferred_velocity: [f32; 2],
    pub radius: f32,
    pub height: f32,
    pub max_speed: f32,
    pub responsibility: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SteeringObstacle {
    pub id: String,
    /// Convex closed polygon, or a two-point one-sided segment. Solid is on the
    /// left of each directed edge: CCW polygons block their interior; CW enclose.
    pub vertices: Vec<[f32; 2]>,
    pub closed: bool,
    pub min_y: f32,
    pub max_y: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SteeringQuery {
    #[schemars(length(max = 128))]
    pub agents: Vec<SteeringAgent>,
    #[serde(default)]
    #[schemars(length(max = 32))]
    pub obstacles: Vec<SteeringObstacle>,
    pub time_horizon: f32,
    pub obstacle_time_horizon: f32,
    /// Additional clearance in meters on each agent, including against obstacles.
    /// Numerical/infeasible ORCA constraints do not guarantee physical separation.
    #[serde(default = "default_margin")]
    pub margin: f32,
}
fn default_margin() -> f32 {
    0.05
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SteeringVelocity {
    pub id: String,
    pub velocity: [f32; 2],
    pub neighbors: u32,
}
fn scalar(n: f32, min: f32, max: f32) -> bool {
    n.is_finite() && (min..=max).contains(&n)
}
fn id_valid(id: &str) -> bool {
    id.len() == 26
        && id.as_bytes()[0] <= b'7'
        && id
            .bytes()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c))
}
fn vector(p: [f32; 2]) -> Vec2 {
    Vec2::from_array(p)
}
fn horizontal(p: [f32; 3]) -> Vec2 {
    Vec2::new(p[0], p[2])
}
fn overlap(a: f32, b: f32, c: f32, d: f32) -> bool {
    a < d && c < b
}
impl SteeringQuery {
    pub fn validate(&self, time_step: f32) -> Result<(), NavigationError> {
        if !scalar(time_step, 1. / 240., 1.)
            || !scalar(self.time_horizon, time_step, 10.)
            || !scalar(self.obstacle_time_horizon, time_step, 10.)
            || !scalar(self.margin, 0., 1.)
        {
            return Err(invalid(
                "steering step must be 1/240..1 s; horizons must be step..10 s; margin must be 0..1 m",
            ));
        }
        if self.agents.len() > 128 || self.obstacles.len() > 32 {
            return Err(limit("steering exceeds 128 agents or 32 obstacles"));
        }
        let mut ids = BTreeSet::new();
        for a in &self.agents {
            if !id_valid(&a.id) || !ids.insert(&a.id) {
                return Err(invalid("steering requires distinct stable ULIDs"));
            }
            if !a.position.iter().all(|v| scalar(*v, -10000., 10000.))
                || !vector(a.velocity).is_finite()
                || vector(a.velocity).length() > 30.
                || !vector(a.preferred_velocity).is_finite()
                || vector(a.preferred_velocity).length() > 30.
                || !scalar(a.radius, 0.05, 4.)
                || !scalar(a.height, 0.1, 10.)
                || !scalar(a.max_speed, 0., 30.)
                || !scalar(a.responsibility, 0.1, 10.)
            {
                return Err(invalid(
                    "invalid steering agent position, velocity, size, speed or responsibility",
                ));
            }
        }
        let mut edges = 0;
        for o in &self.obstacles {
            if !id_valid(&o.id) || !ids.insert(&o.id) {
                return Err(invalid("steering requires distinct stable ULIDs"));
            }
            let n = o.vertices.len();
            if (o.closed && !(3..=32).contains(&n)) || (!o.closed && n != 2) {
                return Err(invalid(
                    "steering obstacle must be a convex 3..32 vertex polygon or two-point segment",
                ));
            }
            edges += if o.closed { n } else { 1 };
            if edges > 128 {
                return Err(limit("steering exceeds 128 obstacle edges"));
            }
            if !scalar(o.min_y, -10000., 10000.)
                || !scalar(o.max_y, -10000., 10010.)
                || o.min_y >= o.max_y
                || !o
                    .vertices
                    .iter()
                    .flatten()
                    .all(|v| scalar(*v, -10000., 10000.))
            {
                return Err(invalid("invalid steering obstacle bounds"));
            }
            let count = if o.closed { n } else { 1 };
            let mut winding = 0_f32;
            for i in 0..count {
                let a = vector(o.vertices[i]);
                let b = vector(o.vertices[(i + 1) % n]);
                let edge = b - a;
                if edge.length_squared() < 1e-6 {
                    return Err(invalid("steering obstacle edge is too short"));
                }
                if o.closed {
                    let turn = edge.perp_dot(vector(o.vertices[(i + 2) % n]) - b);
                    if turn.abs() < 1e-6 {
                        return Err(invalid("steering polygon has a degenerate corner"));
                    }
                    if winding == 0. {
                        winding = turn.signum();
                    }
                    // Every vertex must be on the same side of every edge. This
                    // also rejects self-crossing stars with uniform local turns.
                    if o.vertices
                        .iter()
                        .any(|p| edge.perp_dot(vector(*p) - a) * winding < -1e-5)
                    {
                        return Err(invalid("steering polygon is not convex and simple"));
                    }
                }
            }
        }
        Ok(())
    }
}
/// All agents read the same input snapshot; output order is canonical by ID.
/// No positions, velocities or documents are mutated by this function.
pub fn steer(
    query: &SteeringQuery,
    time_step: f32,
) -> Result<Vec<SteeringVelocity>, NavigationError> {
    query.validate(time_step)?;
    let mut agents: Vec<_> = query.agents.iter().collect();
    agents.sort_by(|a, b| a.id.cmp(&b.id));
    let mut obstacles: Vec<_> = query.obstacles.iter().collect();
    obstacles.sort_by(|a, b| a.id.cmp(&b.id));
    let mut output = Vec::with_capacity(agents.len());
    for input in &agents {
        let origin = horizontal(input.position);
        let agent = Agent {
            position: Vec2::ZERO,
            velocity: vector(input.velocity),
            radius: input.radius + query.margin,
            avoidance_responsibility: input.responsibility,
        };
        let mut neighbors = vec![];
        let preferred = vector(input.preferred_velocity).clamp_length_max(input.max_speed);
        let mut crossing = false;
        for other in &agents {
            if input.id == other.id
                || !overlap(
                    input.position[1],
                    input.position[1] + input.height,
                    other.position[1],
                    other.position[1] + other.height,
                )
            {
                continue;
            }
            let mut position = horizontal(other.position) - origin;
            let range = (input.max_speed.max(vector(input.velocity).length())
                + other.max_speed.max(vector(other.velocity).length()))
                * query.time_horizon
                + input.radius
                + other.radius
                + 2. * query.margin;
            if position.length_squared() > range * range {
                continue;
            }
            // A perfectly reciprocal head-on crowd can satisfy ORCA by stopping
            // forever. Choose a 45-degree passing preference at unchanged speed when
            // requested velocities predict a collision. The hand is consistent
            // for every moving agent. This is an objective
            // preference only: obstacle/agent constraints still project it into
            // the feasible velocity region below.
            let relative_preferred =
                preferred - vector(other.preferred_velocity).clamp_length_max(other.max_speed);
            let closing = position.dot(relative_preferred);
            if closing > 0. {
                let encounter = closing / relative_preferred.length_squared();
                let separation = position - relative_preferred * encounter;
                let clearance = input.radius + other.radius + 2. * query.margin;
                crossing |= encounter <= query.time_horizon
                    && separation.length_squared() < clearance * clearance;
            }
            // The backend chooses a random normal for a zero-length overlap
            // vector. Replace only that singular case with an antisymmetric,
            // stable pair direction. Local coordinates retain the tiny offset.
            let w = agent.velocity - vector(other.velocity) - position / time_step;
            if position.length_squared()
                <= (input.radius + other.radius + 2. * query.margin).powi(2)
                && w.length_squared() < 1e-10
            {
                let direction = pair_direction(&input.id, &other.id);
                position -= direction * (time_step * 0.001);
                let adjusted = agent.velocity - vector(other.velocity) - position / time_step;
                if !adjusted.length_recip().is_finite() {
                    return Err(invalid("unresolved steering overlap singularity"));
                }
            }
            neighbors.push(Cow::Owned(Agent {
                position,
                velocity: vector(other.velocity),
                radius: other.radius + query.margin,
                avoidance_responsibility: other.responsibility,
            }));
        }
        let obstacles: Vec<_> = obstacles
            .iter()
            .filter(|o| {
                overlap(
                    input.position[1],
                    input.position[1] + input.height,
                    o.min_y,
                    o.max_y,
                )
            })
            .map(|o| {
                let vertices = o.vertices.iter().map(|p| vector(*p) - origin).collect();
                Cow::Owned(if o.closed {
                    Obstacle::Closed { vertices }
                } else {
                    Obstacle::Open { vertices }
                })
            })
            .collect();
        let preferred = if crossing {
            (preferred + preferred.perp()) * std::f32::consts::FRAC_1_SQRT_2
        } else {
            preferred
        };
        let velocity = agent.compute_avoiding_velocity(
            &neighbors,
            &obstacles,
            preferred,
            input.max_speed,
            time_step,
            &AvoidanceOptions {
                obstacle_margin: input.radius + query.margin,
                time_horizon: query.time_horizon,
                obstacle_time_horizon: query.obstacle_time_horizon,
            },
        );
        if !velocity.is_finite() {
            return Err(invalid(format!(
                "avoidance solver returned an invalid velocity: {} {:?} max {}",
                input.id, velocity, input.max_speed
            )));
        }
        output.push(SteeringVelocity {
            id: input.id.clone(),
            // Floating-point LP intersections can drift outside the speed circle.
            // Reapply the caller's hard speed cap; physics still owns collision.
            velocity: velocity.clamp_length_max(input.max_speed).to_array(),
            neighbors: neighbors.len() as u32,
        });
    }
    Ok(output)
}
fn pair_direction(a: &str, b: &str) -> Vec2 {
    let (low, high, sign) = if a < b { (a, b, 1.) } else { (b, a, -1.) };
    let hash = low
        .bytes()
        .chain(high.bytes())
        .fold(2166136261_u32, |hash, b| {
            (hash ^ u32::from(b)).wrapping_mul(16777619)
        });
    // Eight fixed unit directions avoid platform trigonometric differences.
    let diagon = std::f32::consts::FRAC_1_SQRT_2;
    let directions = [
        Vec2::X,
        Vec2::new(diagon, diagon),
        Vec2::Y,
        Vec2::new(-diagon, diagon),
        -Vec2::X,
        Vec2::new(-diagon, -diagon),
        -Vec2::Y,
        Vec2::new(diagon, -diagon),
    ];
    directions[(hash % 8) as usize] * sign
}
