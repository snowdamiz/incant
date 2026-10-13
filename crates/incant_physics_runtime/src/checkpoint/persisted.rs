//! Typed outer state surrounding stricter persisted native admission.
use super::wire::{Reader, Writer};
use super::*;
use incant_types::PrimitiveColliderShape;
use rapier3d::pipeline::checked_restore::capture::CheckpointOwner;
const MAGIC: &[u8; 8] = b"INPHY3D1";

impl PhysicsCheckpoint {
    /// Validate the stricter native file domain and retain the original native
    /// envelope bytes. Every temporary/output allocation shares `payload`.
    pub fn persisted_bytes(
        &self,
        payload: &mut Budget,
        work: &mut WorkBudget,
    ) -> Result<Vec<u8>, CheckpointError> {
        let entries = total_entries(self.scenes.iter().map(|s| s.entries.len()))?;
        charge_work(
            self.scenes.len(),
            entries,
            self.overlaps.len(),
            self.events.len(),
            work,
        )?;
        charge_geometry(
            self.scenes
                .iter()
                .flat_map(|s| s.entries.iter().map(|e| &e.config)),
            work,
        )?;
        let mut native = payload.vector(self.scenes.len())?;
        for scene in &self.scenes {
            native.push(scene.native.persisted_bytes(payload, work)?);
        }
        wire::encode(payload, |out| {
            out.raw(MAGIC)?;
            out.count(self.scenes.len())?;
            for (scene, bytes) in self.scenes.iter().zip(&native) {
                out.id(scene.id)?;
                out.blob(bytes)?;
                out.count(scene.entries.len())?;
                for row in &scene.entries {
                    out.id(row.id)?;
                    out.boolean(row.body.is_some())?;
                    if let Some(body) = row.body {
                        out.handle(body.into_raw_parts())?;
                    }
                    out.handle(row.collider.into_raw_parts())?;
                    write_config(out, &row.config)?;
                }
            }
            out.count(self.overlaps.len())?;
            for &(scene, first, second) in &self.overlaps {
                out.id(scene)?;
                out.id(first)?;
                out.id(second)?;
            }
            out.count(self.events.len())?;
            for event in &self.events {
                out.id(event.scene_id)?;
                out.id(event.first)?;
                out.id(event.second)?;
                out.boolean(event.entered)?;
            }
            Ok(())
        })
    }
}
impl PhysicsRuntime<StableId> {
    /// Stage all native owners, exact outer declarations and event history.
    /// No destination is mutated; a late refusal drops the whole candidate.
    pub fn from_persisted_bytes(
        bytes: &[u8],
        payload: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<Self, CheckpointError> {
        let mut input = Reader::new(bytes, MAGIC)?;
        let scenes = input.count(MAX_ITEMS, 24)?;
        charge_work(scenes, 0, 0, 0, work)?;
        charge::<(StableId, SceneWorld<StableId>)>(scenes, payload)?;
        let mut candidate = Self::default();
        let mut previous_scene = None;
        let mut entry_count = 0usize;
        let mut part_count = 0usize;
        for _ in 0..scenes {
            let id = input.id()?;
            if previous_scene.is_some_and(|p| p >= id) {
                return Err(CheckpointError::Owner("scene order"));
            }
            previous_scene = Some(id);
            let native_bytes = input.blob()?;
            let count = input.count(1_000_000, 28)?;
            entry_count = entry_count
                .checked_add(count)
                .filter(|n| *n <= 1_000_000)
                .ok_or(CheckpointError::Owner("entry limit"))?;
            charge_work(0, count, 0, 0, work)?;
            charge::<(StableId, Entry)>(count, payload)?;
            charge::<(ColliderHandle, StableId)>(count, payload)?;
            let native = CheckpointOwner::from_persisted(native_bytes, payload, work, allocations)?;
            let mut scene = SceneWorld {
                world: NativeOwner { inner: native },
                entries: BTreeMap::new(),
                ids: HashMap::new(),
                query_dirty: false,
                character: None,
            };
            scene
                .ids
                .try_reserve(count)
                .map_err(|_| RecordError::Allocation)?;
            let mut previous = None;
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
                let config = read_config(&mut input, payload, work, &mut part_count)?;
                config
                    .validate()
                    .map_err(|_| CheckpointError::Owner("invalid persisted declaration"))?;
                if scene.ids.insert(collider, id).is_some() {
                    return Err(CheckpointError::Owner("duplicate collider identity"));
                }
                scene.entries.insert(
                    id,
                    Entry {
                        body,
                        collider,
                        config,
                    },
                );
            }
            candidate.scenes.insert(id, scene);
        }
        let count = input.count(MAX_TRIGGER_PAIRS, 48)?;
        charge_work(0, 0, count, 0, work)?;
        charge::<(StableId, StableId, StableId)>(count, payload)?;
        let mut previous = None;
        for _ in 0..count {
            let row = (input.id()?, input.id()?, input.id()?);
            if previous.is_some_and(|p| p >= row) {
                return Err(CheckpointError::Owner("overlap order"));
            }
            previous = Some(row);
            candidate.overlaps.insert(row);
        }
        let count = input.count(MAX_TRIGGER_EVENTS, 49)?;
        charge_work(0, 0, 0, count, work)?;
        candidate.events = payload.vector(count)?;
        for _ in 0..count {
            candidate.events.push(TriggerEvent {
                scene_id: input.id()?,
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
fn write_primitive(
    out: &mut Writer,
    shape: &PrimitiveColliderShape,
) -> Result<(), CheckpointError> {
    match shape {
        PrimitiveColliderShape::Box { half_extents } => {
            out.byte(0)?;
            out.floats(half_extents)
        }
        PrimitiveColliderShape::Sphere { radius } => {
            out.byte(1)?;
            out.scalar(*radius)
        }
        PrimitiveColliderShape::Capsule {
            half_height,
            radius,
        } => {
            out.byte(2)?;
            out.scalar(*half_height)?;
            out.scalar(*radius)
        }
    }
}
fn read_primitive(input: &mut Reader<'_>) -> Result<PrimitiveColliderShape, CheckpointError> {
    match input.byte()? {
        0 => Ok(PrimitiveColliderShape::Box {
            half_extents: input.floats()?,
        }),
        1 => Ok(PrimitiveColliderShape::Sphere {
            radius: input.scalar()?,
        }),
        2 => Ok(PrimitiveColliderShape::Capsule {
            half_height: input.scalar()?,
            radius: input.scalar()?,
        }),
        _ => Err(CheckpointError::Owner("primitive shape tag")),
    }
}
fn write_config(out: &mut Writer, value: &PhysicsSpec) -> Result<(), CheckpointError> {
    out.body(&value.body)?;
    match &value.collider.shape {
        ColliderShape::Box { half_extents } => {
            out.byte(0)?;
            out.floats(half_extents)?;
        }
        ColliderShape::Sphere { radius } => {
            out.byte(1)?;
            out.scalar(*radius)?;
        }
        ColliderShape::Capsule {
            half_height,
            radius,
        } => {
            out.byte(2)?;
            out.scalar(*half_height)?;
            out.scalar(*radius)?;
        }
        ColliderShape::Compound { parts } => {
            out.byte(3)?;
            out.count(parts.len())?;
            for p in parts {
                out.raw(p.id.as_bytes())?;
                out.floats(&p.translation)?;
                out.floats(&p.rotation)?;
                write_primitive(out, &p.shape)?;
            }
        }
    }
    out.scalar(value.collider.density)?;
    out.scalar(value.collider.friction)?;
    out.scalar(value.collider.restitution)?;
    out.boolean(value.collider.sensor)?;
    out.u32(value.collider.memberships)?;
    out.u32(value.collider.filter)?;
    out.floats(&value.state.translation)?;
    out.floats(&value.state.rotation)?;
    out.floats(&value.state.velocity)?;
    out.floats(&value.state.angular_velocity)?;
    out.boolean(value.state.sleeping)
}
fn read_config(
    input: &mut Reader<'_>,
    payload: &mut Budget,
    work: &mut WorkBudget,
    part_count: &mut usize,
) -> Result<PhysicsSpec, CheckpointError> {
    let body = input.body()?;
    let shape = match input.byte()? {
        0 => ColliderShape::Box {
            half_extents: input.floats()?,
        },
        1 => ColliderShape::Sphere {
            radius: input.scalar()?,
        },
        2 => ColliderShape::Capsule {
            half_height: input.scalar()?,
            radius: input.scalar()?,
        },
        3 => {
            let count = input.count(64, 26 + 7 * 8 + 9)?;
            if count == 0 {
                return Err(CheckpointError::Owner("empty compound"));
            }
            *part_count = part_count
                .checked_add(count)
                .filter(|n| *n <= MAX_ITEMS)
                .ok_or(CheckpointError::Owner("compound aggregate part limit"))?;
            work.charge(count as u64 * 32).map_err(CaptureError::from)?;
            let mut parts = payload.vector(count)?;
            for _ in 0..count {
                parts.push(ColliderPart {
                    id: input.text26(payload)?,
                    translation: input.floats()?,
                    rotation: input.floats()?,
                    shape: read_primitive(input)?,
                });
            }
            ColliderShape::Compound { parts }
        }
        _ => return Err(CheckpointError::Owner("collider shape tag")),
    };
    Ok(PhysicsSpec {
        body,
        collider: DocCollider {
            shape,
            density: input.scalar()?,
            friction: input.scalar()?,
            restitution: input.scalar()?,
            sensor: input.boolean()?,
            memberships: input.u32()?,
            filter: input.u32()?,
        },
        state: BodyState {
            translation: input.floats()?,
            rotation: input.floats()?,
            velocity: input.floats()?,
            angular_velocity: input.floats()?,
            sleeping: input.boolean()?,
        },
    })
}

#[cfg(test)]
mod tests;
