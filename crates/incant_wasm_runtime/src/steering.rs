//! Stateless avoidance over copied caller-supplied world-space samples.
use crate::BoundaryError as Error;
use incant_runtime::{StableId, SteeringAgent, SteeringObstacle, SteeringQuery};
use incant_wasm_abi::{self as abi, steering as wire};

pub(crate) struct State {
    dt: f64,
    agents: usize,
    edges: usize,
    reply: Option<Vec<u8>>,
    pub rejected: bool,
}
impl State {
    pub fn new(dt: f64) -> Self {
        Self {
            dt,
            agents: 0,
            edges: 0,
            reply: None,
            rejected: false,
        }
    }
    pub fn reject(&mut self) {
        self.rejected = true;
        self.reply = None;
    }
    pub fn reply(&self) -> Result<&[u8], Error> {
        if self.rejected {
            return Err(Error::Abi("rejected steering transaction"));
        }
        self.reply
            .as_deref()
            .ok_or(Error::Abi("no steering result"))
    }
    pub fn steer(&mut self, bytes: &[u8]) -> Result<i32, Error> {
        self.reply = None;
        if self.rejected {
            return Err(Error::Abi("rejected steering transaction"));
        }
        // Check f64 before conversion so a sub-minimum clock cannot round into
        // the accepted f32 domain. This matches the TypeScript native boundary.
        if !self.dt.is_finite() || !(1. / 240. ..=1.).contains(&self.dt) {
            return Err(Error::Abi("steering time step"));
        }
        let input = wire::decode_request(bytes).map_err(|_| Error::Abi("steering frame"))?;
        charge(
            &mut self.agents,
            input.header.agents as usize,
            abi::STEERING_MAX_AGENTS,
        )?;
        charge(&mut self.edges, input.edges, abi::STEERING_MAX_EDGES)?;
        validate_numbers(&input, self.dt)?;
        let request = SteeringQuery {
            time_horizon: input.header.time_horizon as f32,
            obstacle_time_horizon: input.header.obstacle_time_horizon as f32,
            margin: input.header.margin as f32,
            agents: input
                .agents()
                .map(|a| SteeringAgent {
                    id: StableId(a.id),
                    position: a.position.map(|v| v as f32),
                    velocity: a.velocity.map(|v| v as f32),
                    preferred_velocity: a.preferred_velocity.map(|v| v as f32),
                    radius: a.radius as f32,
                    height: a.height as f32,
                    max_speed: a.max_speed as f32,
                    responsibility: a.responsibility as f32,
                })
                .collect(),
            obstacles: input
                .obstacles()
                .map(|o| SteeringObstacle {
                    id: StableId(o.header.id),
                    closed: o.header.closed != 0,
                    min_y: o.header.min_y as f32,
                    max_y: o.header.max_y as f32,
                    vertices: o.vertices().map(|p| p.map(|v| v as f32)).collect(),
                })
                .collect(),
        };
        let proposals = incant_nav::steer(&request, self.dt as f32)
            .map_err(|_| Error::Abi("invalid steering request"))?;
        let rows: Vec<_> = proposals
            .into_iter()
            .map(|v| abi::SteeringVelocity {
                id: v.id.0,
                velocity: v.velocity.map(f64::from),
                neighbors: v.neighbors,
                reserved: 0,
            })
            .collect();
        let mut reply =
            vec![0; 8 + abi::SteeringResultHeader::SIZE + rows.len() * abi::SteeringVelocity::SIZE];
        let len =
            wire::encode_result(&rows, &mut reply).map_err(|_| Error::Abi("steering result"))?;
        self.reply = Some(reply);
        Ok(len as i32)
    }
}
fn charge(count: &mut usize, delta: usize, max: usize) -> Result<(), Error> {
    *count = count
        .checked_add(delta)
        .filter(|v| *v <= max)
        .ok_or(Error::Limit("steering work quota"))?;
    Ok(())
}

// Match the TypeScript ingress domain before narrowing into the f32 solver.
// Finite-but-invalid f64s can otherwise round onto an accepted boundary.
fn validate_numbers(input: &wire::DecodedRequest<'_>, dt: f64) -> Result<(), Error> {
    fn scalar(value: f64, min: f64, max: f64) -> bool {
        value.is_finite() && (min..=max).contains(&value)
    }
    fn point(values: &[f64]) -> bool {
        values.iter().all(|v| scalar(*v, -10000., 10000.))
    }
    fn velocity(v: [f64; 2]) -> bool {
        v.iter().all(|n| n.is_finite()) && v[0].hypot(v[1]) <= 30.
    }
    let h = &input.header;
    if !scalar(h.time_horizon, dt, 10.)
        || !scalar(h.obstacle_time_horizon, dt, 10.)
        || !scalar(h.margin, 0., 1.)
        || input.agents().any(|a| {
            !point(&a.position)
                || !velocity(a.velocity)
                || !velocity(a.preferred_velocity)
                || !scalar(a.radius, 0.05, 4.)
                || !scalar(a.height, 0.1, 10.)
                || !scalar(a.max_speed, 0., 30.)
                || !scalar(a.responsibility, 0.1, 10.)
        })
        || input.obstacles().any(|o| {
            !scalar(o.header.min_y, -10000., 10000.)
                || !scalar(o.header.max_y, -10000., 10010.)
                || o.vertices().any(|v| !point(&v))
        })
    {
        return Err(Error::Abi("steering number outside domain"));
    }
    Ok(())
}
