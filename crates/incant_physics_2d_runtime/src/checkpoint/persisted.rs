//! Exact typed declarations/events around stricter native persisted admission.
use super::wire::{Reader, Writer};
use super::*;
use incant_types::{Collider2D, ColliderShape2D};
use rapier2d::pipeline::checked_restore::capture::CheckpointOwner;
const MAGIC: &[u8; 8] = b"INPHY2D1";

impl PhysicsCheckpoint {
    pub fn persisted_bytes(
        &self,
        payload: &mut Budget,
        work: &mut WorkBudget,
    ) -> Result<Vec<u8>, CheckpointError> {
        charge_work(
            self.entries.len(),
            self.overlaps.len(),
            self.events.len(),
            work,
        )?;
        charge_geometry(self.entries.iter().map(|e| &e.config), work)?;
        let native = self.native.persisted_bytes(payload, work)?;
        wire::encode(payload, |out| {
            out.raw(MAGIC)?;
            out.blob(&native)?;
            out.count(self.entries.len())?;
            for row in &self.entries {
                out.id(row.id)?;
                out.boolean(row.body.is_some())?;
                if let Some(body) = row.body {
                    out.handle(body.into_raw_parts())?;
                }
                out.handle(row.collider.into_raw_parts())?;
                write_config(out, &row.config)?;
            }
            out.count(self.overlaps.len())?;
            for &(first, second) in &self.overlaps {
                out.id(first)?;
                out.id(second)?;
            }
            out.count(self.events.len())?;
            for event in &self.events {
                out.id(event.first)?;
                out.id(event.second)?;
                out.boolean(event.entered)?;
            }
            Ok(())
        })
    }
}
impl PhysicsWorld2D {
    /// Construct a private candidate, never a historical completed-step token.
    /// Every failure leaves the destination and input bytes unchanged.
    pub fn from_persisted_bytes(
        bytes: &[u8],
        payload: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<Self, CheckpointError> {
        let mut input = Reader::new(bytes, MAGIC)?;
        let native_bytes = input.blob()?;
        let count = input.count(MAX_BODIES, 28)?;
        charge_work(count, 0, 0, work)?;
        charge::<(StableId, Entry)>(count, payload)?;
        charge::<(ColliderHandle, StableId)>(count, payload)?;
        let native = CheckpointOwner::from_persisted(native_bytes, payload, work, allocations)?;
        let mut candidate = Self {
            world: NativeOwner { inner: native },
            entries: BTreeMap::new(),
            ids: HashMap::new(),
            overlaps: BTreeSet::new(),
            events: Vec::new(),
            query_dirty: false,
            failed: false,
        };
        candidate
            .ids
            .try_reserve(count)
            .map_err(|_| RecordError::Allocation)?;
        let mut previous = None;
        let mut shapes = 0usize;
        for _ in 0..count {
            let id = input.id()?;
            if previous.is_some_and(|p| p >= id) {
                return Err(CheckpointError::Owner("entity order"));
            }
            previous = Some(id);
            let body = if input.boolean()? {
                let (index, generation) = input.handle()?;
                Some(RigidBodyHandle::from_raw_parts(index, generation))
            } else {
                None
            };
            let (index, generation) = input.handle()?;
            let collider = ColliderHandle::from_raw_parts(index, generation);
            let config = read_config(&mut input, payload, work, &mut shapes)?;
            if candidate.ids.insert(collider, id).is_some() {
                return Err(CheckpointError::Owner("duplicate collider identity"));
            }
            candidate.entries.insert(
                id,
                Entry {
                    body,
                    collider,
                    config,
                },
            );
        }
        let count = input.count(MAX_TRIGGER_PAIRS, 32)?;
        charge_work(0, count, 0, work)?;
        charge::<(StableId, StableId)>(count, payload)?;
        let mut previous = None;
        for _ in 0..count {
            let row = (input.id()?, input.id()?);
            if previous.is_some_and(|p| p >= row) {
                return Err(CheckpointError::Owner("overlap order"));
            }
            previous = Some(row);
            candidate.overlaps.insert(row);
        }
        let count = input.count(MAX_TRIGGER_EVENTS, 33)?;
        charge_work(0, 0, count, work)?;
        candidate.events = payload.vector(count)?;
        for _ in 0..count {
            candidate.events.push(TriggerEvent {
                first: input.id()?,
                second: input.id()?,
                entered: input.boolean()?,
            });
        }
        input.finish()?;
        charge_native_pairs(&candidate, work)?;
        checkpoint_validation::validate(&candidate).map_err(CheckpointError::Owner)?;
        Ok(candidate)
    }
}
fn write_shape(out: &mut Writer, shape: &ColliderShape2D) -> Result<(), CheckpointError> {
    match shape {
        ColliderShape2D::Box { half_extents } => {
            out.byte(0)?;
            out.floats(half_extents)
        }
        ColliderShape2D::Circle { radius } => {
            out.byte(1)?;
            out.scalar(*radius)
        }
        ColliderShape2D::Capsule {
            half_height,
            radius,
        } => {
            out.byte(2)?;
            out.scalar(*half_height)?;
            out.scalar(*radius)
        }
    }
}
fn read_shape(input: &mut Reader<'_>) -> Result<ColliderShape2D, CheckpointError> {
    match input.byte()? {
        0 => Ok(ColliderShape2D::Box {
            half_extents: input.floats()?,
        }),
        1 => Ok(ColliderShape2D::Circle {
            radius: input.scalar()?,
        }),
        2 => Ok(ColliderShape2D::Capsule {
            half_height: input.scalar()?,
            radius: input.scalar()?,
        }),
        _ => Err(CheckpointError::Owner("planar primitive shape tag")),
    }
}
fn write_config(out: &mut Writer, value: &PreparedBody) -> Result<(), CheckpointError> {
    out.body(&value.body)?;
    write_shape(out, &value.collider.shape)?;
    out.scalar(value.collider.density)?;
    out.scalar(value.collider.friction)?;
    out.scalar(value.collider.restitution)?;
    out.boolean(value.collider.sensor)?;
    out.u32(value.collider.memberships)?;
    out.u32(value.collider.filter)?;
    match &value.geometry {
        config::Geometry::Primitive(shape) => {
            out.byte(0)?;
            write_shape(out, shape)?;
        }
        config::Geometry::Tiles(parts) => {
            out.byte(1)?;
            out.count(parts.len())?;
            for (center, half) in parts {
                out.floats(center)?;
                out.floats(half)?;
            }
        }
    }
    out.floats(&value.state.translation)?;
    out.scalar(value.state.rotation)?;
    out.floats(&value.state.velocity)?;
    out.scalar(value.state.angular_velocity)?;
    out.boolean(value.state.sleeping)
}
fn read_config(
    input: &mut Reader<'_>,
    payload: &mut Budget,
    work: &mut WorkBudget,
    shapes: &mut usize,
) -> Result<PreparedBody, CheckpointError> {
    let body = input.body()?;
    let collider = Collider2D {
        shape: read_shape(input)?,
        density: input.scalar()?,
        friction: input.scalar()?,
        restitution: input.scalar()?,
        sensor: input.boolean()?,
        memberships: input.u32()?,
        filter: input.u32()?,
    };
    let geometry = match input.byte()? {
        0 => {
            *shapes = shapes
                .checked_add(1)
                .filter(|n| *n <= MAX_SHAPES)
                .ok_or(CheckpointError::Owner("aggregate shape limit"))?;
            work.charge(4).map_err(CaptureError::from)?;
            config::Geometry::Primitive(read_shape(input)?)
        }
        1 => {
            let count = input.count(MAX_SHAPES, 32)?;
            if count == 0 {
                return Err(CheckpointError::Owner("empty tile geometry"));
            }
            *shapes = shapes
                .checked_add(count)
                .filter(|n| *n <= MAX_SHAPES)
                .ok_or(CheckpointError::Owner("aggregate shape limit"))?;
            work.charge(count as u64 * 4).map_err(CaptureError::from)?;
            let mut parts = payload.vector(count)?;
            for _ in 0..count {
                parts.push((input.floats()?, input.floats()?));
            }
            config::Geometry::Tiles(parts)
        }
        _ => return Err(CheckpointError::Owner("planar geometry tag")),
    };
    let state = BodyState {
        translation: input.floats()?,
        rotation: input.scalar()?,
        velocity: input.floats()?,
        angular_velocity: input.scalar()?,
        sleeping: input.boolean()?,
    };
    // Geometry/order is retained literally. The joined validator independently
    // checks tile ownership/default material and concrete native parts.
    Ok(PreparedBody {
        body,
        collider,
        geometry,
        state,
    })
}

#[cfg(test)]
mod tests;
