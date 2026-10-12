//! Bounded copied local-avoidance requests. No allocation, native handles or
//! implicit world reads cross this wire; IDs correlate proposals with inputs.
use crate::{
    STEERING_MAX_AGENTS, STEERING_MAX_EDGES, STEERING_MAX_OBSTACLES, STEERING_MAX_REQUEST_BYTES,
    STEERING_MAX_RESULT_BYTES, STEERING_MAX_VERTICES, STEERING_REQUEST_MAGIC,
    STEERING_RESULT_MAGIC, SteeringAgent, SteeringHeader, SteeringObstacleHeader,
    SteeringResultHeader, SteeringVelocity,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Malformed,
}
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub struct Obstacle<'a> {
    pub id: [u8; 16],
    pub vertices: &'a [[f64; 2]],
    pub closed: bool,
    pub min_y: f64,
    pub max_y: f64,
}
#[derive(Clone, Copy)]
pub struct Request<'a> {
    pub agents: &'a [SteeringAgent],
    pub obstacles: &'a [Obstacle<'a>],
    pub time_horizon: f64,
    pub obstacle_time_horizon: f64,
    pub margin: f64,
}
fn vertices_valid(n: usize, closed: bool) -> bool {
    if closed {
        (3..=STEERING_MAX_VERTICES).contains(&n)
    } else {
        n == 2
    }
}
fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// Validate the whole logical frame before modifying caller-owned scratch.
pub fn encode_request(request: Request<'_>, out: &mut [u8]) -> Result<usize> {
    if request.agents.len() > STEERING_MAX_AGENTS
        || request.obstacles.len() > STEERING_MAX_OBSTACLES
    {
        return Err(Error::Bounds);
    }
    let header = SteeringHeader {
        agents: request.agents.len() as u32,
        obstacles: request.obstacles.len() as u32,
        time_horizon: request.time_horizon,
        obstacle_time_horizon: request.obstacle_time_horizon,
        margin: request.margin,
    };
    if !header.is_finite() || request.agents.iter().any(|a| !a.is_finite()) {
        return Err(Error::Malformed);
    }
    let mut len = 8 + SteeringHeader::SIZE + request.agents.len() * SteeringAgent::SIZE;
    let mut edges = 0;
    for obstacle in request.obstacles {
        if !vertices_valid(obstacle.vertices.len(), obstacle.closed)
            || !finite(&[obstacle.min_y, obstacle.max_y])
            || obstacle.vertices.iter().any(|v| !finite(v))
        {
            return Err(Error::Malformed);
        }
        edges += if obstacle.closed {
            obstacle.vertices.len()
        } else {
            1
        };
        if edges > STEERING_MAX_EDGES {
            return Err(Error::Bounds);
        }
        len += SteeringObstacleHeader::SIZE + obstacle.vertices.len() * 16;
    }
    if len > STEERING_MAX_REQUEST_BYTES || out.len() < len {
        return Err(Error::Bounds);
    }
    let mut writer = Writer {
        bytes: &mut out[..len],
        position: 0,
    };
    writer.put(STEERING_REQUEST_MAGIC);
    writer.put(&header.encode());
    for agent in request.agents {
        writer.put(&agent.encode());
    }
    for obstacle in request.obstacles {
        writer.put(
            &SteeringObstacleHeader {
                id: obstacle.id,
                min_y: obstacle.min_y,
                max_y: obstacle.max_y,
                vertices: obstacle.vertices.len() as u32,
                closed: u8::from(obstacle.closed),
                reserved: [0; 3],
            }
            .encode(),
        );
        for vertex in obstacle.vertices {
            for value in vertex {
                writer.put(&value.to_le_bytes());
            }
        }
    }
    Ok(len)
}
struct Writer<'a> {
    bytes: &'a mut [u8],
    position: usize,
}
impl Writer<'_> {
    fn put(&mut self, bytes: &[u8]) {
        self.bytes[self.position..self.position + bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
    }
}

pub struct DecodedRequest<'a> {
    pub header: SteeringHeader,
    pub edges: usize,
    agents: &'a [u8],
    obstacles: &'a [u8],
}
impl DecodedRequest<'_> {
    pub fn agents(&self) -> impl ExactSizeIterator<Item = SteeringAgent> + '_ {
        self.agents
            .as_chunks::<{ SteeringAgent::SIZE }>()
            .0
            .iter()
            .map(|b| SteeringAgent::decode(b).expect("checked agent"))
    }
    pub fn obstacles(&self) -> impl Iterator<Item = DecodedObstacle<'_>> {
        let mut remaining = self.obstacles;
        core::iter::from_fn(move || {
            if remaining.is_empty() {
                return None;
            }
            let header = SteeringObstacleHeader::decode(remaining).expect("checked obstacle");
            let end = SteeringObstacleHeader::SIZE + header.vertices as usize * 16;
            let vertices = &remaining[SteeringObstacleHeader::SIZE..end];
            remaining = &remaining[end..];
            Some(DecodedObstacle {
                header,
                bytes: vertices,
            })
        })
    }
}
pub struct DecodedObstacle<'a> {
    pub header: SteeringObstacleHeader,
    bytes: &'a [u8],
}
impl DecodedObstacle<'_> {
    pub fn vertices(&self) -> impl ExactSizeIterator<Item = [f64; 2]> + '_ {
        self.bytes.as_chunks::<16>().0.iter().map(|b| vertex(b))
    }
}
fn vertex(bytes: &[u8]) -> [f64; 2] {
    [
        f64::from_le_bytes(bytes[..8].try_into().unwrap()),
        f64::from_le_bytes(bytes[8..16].try_into().unwrap()),
    ]
}
pub fn decode_request(bytes: &[u8]) -> Result<DecodedRequest<'_>> {
    if bytes.len() > STEERING_MAX_REQUEST_BYTES {
        return Err(Error::Bounds);
    }
    if bytes.get(..8) != Some(STEERING_REQUEST_MAGIC) {
        return Err(Error::Malformed);
    }
    let header = SteeringHeader::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    if !header.is_finite() {
        return Err(Error::Malformed);
    }
    if header.agents as usize > STEERING_MAX_AGENTS
        || header.obstacles as usize > STEERING_MAX_OBSTACLES
    {
        return Err(Error::Bounds);
    }
    let offset = 8 + SteeringHeader::SIZE;
    let end = offset + header.agents as usize * SteeringAgent::SIZE;
    let agents = bytes.get(offset..end).ok_or(Error::Malformed)?;
    if agents
        .as_chunks::<{ SteeringAgent::SIZE }>()
        .0
        .iter()
        .any(|b| !SteeringAgent::decode(b).unwrap().is_finite())
    {
        return Err(Error::Malformed);
    }
    let obstacles = &bytes[end..];
    let mut remaining = obstacles;
    let mut edges = 0;
    for _ in 0..header.obstacles {
        let h = SteeringObstacleHeader::decode(remaining).ok_or(Error::Malformed)?;
        if !h.is_finite()
            || h.closed > 1
            || h.reserved != [0; 3]
            || !vertices_valid(h.vertices as usize, h.closed != 0)
        {
            return Err(Error::Malformed);
        }
        edges += if h.closed != 0 {
            h.vertices as usize
        } else {
            1
        };
        if edges > STEERING_MAX_EDGES {
            return Err(Error::Bounds);
        }
        let end = SteeringObstacleHeader::SIZE + h.vertices as usize * 16;
        let vertices = remaining
            .get(SteeringObstacleHeader::SIZE..end)
            .ok_or(Error::Malformed)?;
        if vertices
            .as_chunks::<16>()
            .0
            .iter()
            .any(|b| !finite(&vertex(b)))
        {
            return Err(Error::Malformed);
        }
        remaining = &remaining[end..];
    }
    if !remaining.is_empty() {
        return Err(Error::Malformed);
    }
    Ok(DecodedRequest {
        header,
        edges,
        agents,
        obstacles,
    })
}

#[derive(Clone, Copy)]
pub struct Velocities<'a> {
    bytes: &'a [u8],
}
impl Velocities<'_> {
    pub fn len(&self) -> usize {
        self.bytes.len() / SteeringVelocity::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    /// Canonical ID order, independent of the input's insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = SteeringVelocity> + '_ {
        self.bytes
            .as_chunks::<{ SteeringVelocity::SIZE }>()
            .0
            .iter()
            .map(|b| SteeringVelocity::decode(b).expect("checked result"))
    }
}
pub fn decode_result(bytes: &[u8]) -> Result<Velocities<'_>> {
    if bytes.len() > STEERING_MAX_RESULT_BYTES {
        return Err(Error::Bounds);
    }
    if bytes.get(..8) != Some(STEERING_RESULT_MAGIC) {
        return Err(Error::Malformed);
    }
    let h = SteeringResultHeader::decode(&bytes[8..]).ok_or(Error::Malformed)?;
    if h.agents as usize > STEERING_MAX_AGENTS || h.reserved != 0 {
        return Err(Error::Malformed);
    }
    let offset = 8 + SteeringResultHeader::SIZE;
    if bytes.len() != offset + h.agents as usize * SteeringVelocity::SIZE {
        return Err(Error::Malformed);
    }
    let result = Velocities {
        bytes: &bytes[offset..],
    };
    let mut previous = None;
    for v in result.iter() {
        if !v.is_finite()
            || v.reserved != 0
            || v.neighbors as usize >= STEERING_MAX_AGENTS
            || previous.is_some_and(|id| id >= v.id)
        {
            return Err(Error::Malformed);
        }
        previous = Some(v.id);
    }
    Ok(result)
}

/// Host output encoder. Identity sorting and numerical validity are checked
/// before publishing any bytes, independently of the native solver.
pub fn encode_result(rows: &[SteeringVelocity], out: &mut [u8]) -> Result<usize> {
    if rows.len() > STEERING_MAX_AGENTS {
        return Err(Error::Bounds);
    }
    let mut previous = None;
    for v in rows {
        if !v.is_finite()
            || v.reserved != 0
            || v.neighbors as usize >= STEERING_MAX_AGENTS
            || previous.is_some_and(|id| id >= v.id)
        {
            return Err(Error::Malformed);
        }
        previous = Some(v.id);
    }
    let len = 8 + SteeringResultHeader::SIZE + rows.len() * SteeringVelocity::SIZE;
    if out.len() < len {
        return Err(Error::Bounds);
    }
    let mut writer = Writer {
        bytes: &mut out[..len],
        position: 0,
    };
    writer.put(STEERING_RESULT_MAGIC);
    writer.put(
        &SteeringResultHeader {
            agents: rows.len() as u32,
            reserved: 0,
        }
        .encode(),
    );
    for v in rows {
        writer.put(&v.encode());
    }
    Ok(len)
}
