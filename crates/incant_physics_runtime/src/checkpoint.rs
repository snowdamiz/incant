//! Opaque checkpoints plus a separate bounded persisted native admission path.
//! Whole-session/script/host composition is required independently.
use super::*;
use incant_physics_snapshot::{
    Budget, Error as RecordError, MAX_ITEMS, construction::AllocationBudget,
};
use incant_types::{ColliderPart, ColliderShape, StableId};
use rapier3d::pipeline::checked_restore::{
    WorkBudget,
    capture::{CaptureError, NativeFrame},
};

#[derive(Debug, Error, PartialEq)]
pub enum CheckpointError {
    #[error("physics checkpoint owner mismatch: {0}")]
    Owner(&'static str),
    #[error("native physics checkpoint refused: {0:?}")]
    Native(CaptureError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error("persisted native physics refused: {0:?}")]
    Persisted(rapier3d::pipeline::checked_restore::persisted::PersistedError),
}
impl From<CaptureError> for CheckpointError {
    fn from(error: CaptureError) -> Self {
        Self::Native(error)
    }
}
struct SavedEntry {
    id: StableId,
    body: Option<RigidBodyHandle>,
    collider: ColliderHandle,
    config: PhysicsSpec,
}
struct SavedScene {
    id: StableId,
    native: NativeFrame,
    entries: Vec<SavedEntry>,
}
/// Complete supported scene owners plus stable IDs and committed observations.
/// The capability stays in memory and has no Clone or bytes-to-value constructor.
pub struct PhysicsCheckpoint {
    scenes: Vec<SavedScene>,
    overlaps: Vec<(StableId, StableId, StableId)>,
    events: Vec<TriggerEvent<StableId>>,
}
fn copy_config(value: &PhysicsSpec, budget: &mut Budget) -> Result<PhysicsSpec, CheckpointError> {
    let shape = match &value.collider.shape {
        ColliderShape::Compound { parts } => {
            if !(1..=64).contains(&parts.len()) {
                return Err(CheckpointError::Owner("compound part limit"));
            }
            let mut copied = budget.vector(parts.len())?;
            for part in parts {
                if part.id.len() != 26 {
                    return Err(CheckpointError::Owner("compound part identity length"));
                }
                budget.charge(part.id.len())?;
                let mut id = String::new();
                id.try_reserve_exact(part.id.len())
                    .map_err(|_| RecordError::Allocation)?;
                id.push_str(&part.id);
                copied.push(ColliderPart {
                    id,
                    translation: part.translation,
                    rotation: part.rotation,
                    shape: part.shape.clone(),
                });
            }
            ColliderShape::Compound { parts: copied }
        }
        primitive => primitive.clone(),
    };
    Ok(PhysicsSpec {
        body: value.body.clone(),
        collider: DocCollider {
            shape,
            density: value.collider.density,
            friction: value.collider.friction,
            restitution: value.collider.restitution,
            sensor: value.collider.sensor,
            memberships: value.collider.memberships,
            filter: value.collider.filter,
        },
        state: value.state.clone(),
    })
}
// Precharge child/identity visits before declaration validation or deep copying.
// The entry walk itself is already charged by charge_work. No shape construction
// or native acceleration rebuild occurs at this outer boundary.
fn charge_geometry<'a>(
    configs: impl Iterator<Item = &'a PhysicsSpec>,
    work: &mut WorkBudget,
) -> Result<(), CheckpointError> {
    let mut total = 0usize;
    for config in configs {
        if let ColliderShape::Compound { parts } = &config.collider.shape {
            if !(1..=64).contains(&parts.len()) {
                return Err(CheckpointError::Owner("compound part limit"));
            }
            total = total
                .checked_add(parts.len())
                .filter(|n| *n <= MAX_ITEMS)
                .ok_or(CheckpointError::Owner("compound aggregate part limit"))?;
            // Each valid ID has exactly 26 bytes; refuse longer strings before
            // any character walk, comparison or clone.
            work.charge(parts.len() as u64 * 32)
                .map_err(CaptureError::from)?;
            if parts.iter().any(|p| p.id.len() != 26) {
                return Err(CheckpointError::Owner("compound part identity length"));
            }
        }
    }
    Ok(())
}
fn charge<T>(count: usize, budget: &mut Budget) -> Result<(), RecordError> {
    budget.charge(
        count
            .checked_mul(size_of::<T>())
            .ok_or(RecordError::Limit)?,
    )
}
fn charge_work(
    scenes: usize,
    entries: usize,
    overlaps: usize,
    events: usize,
    work: &mut WorkBudget,
) -> Result<(), CheckpointError> {
    if scenes > MAX_ITEMS
        || entries > 1_000_000
        || overlaps > MAX_TRIGGER_PAIRS
        || events > MAX_TRIGGER_EVENTS
    {
        return Err(CheckpointError::Owner("collection limit"));
    }
    work.charge(1 + scenes as u64 + entries as u64 * 4 + overlaps as u64 * 2 + events as u64)
        .map_err(CaptureError::from)?;
    Ok(())
}
fn total_entries(mut counts: impl Iterator<Item = usize>) -> Result<usize, CheckpointError> {
    counts.try_fold(0usize, |a, n| {
        a.checked_add(n)
            .filter(|n| *n <= 1_000_000)
            .ok_or(CheckpointError::Owner("entry limit"))
    })
}
// The pinned graph/map/filter iterator supplies a constant-time upper bound.
// Charge every candidate, including false intersections and filtered handles,
// before the outer join walks the graph. An absent bound is refused explicitly.
fn charge_native_pairs(
    runtime: &PhysicsRuntime<StableId>,
    work: &mut WorkBudget,
) -> Result<(), CheckpointError> {
    for world in runtime.scenes.values().map(|scene| &scene.world) {
        let count = world
            .inner
            .get()?
            .intersection_pairs()
            .size_hint()
            .1
            .filter(|count| *count <= incant_physics_snapshot::MAX_ITEMS)
            .ok_or(CheckpointError::Owner("native pair visit bound"))?;
        work.charge(count as u64).map_err(CaptureError::from)?;
    }
    Ok(())
}
impl PhysicsRuntime<StableId> {
    /// Borrow declarations and cached observations in `(scene, entity)` order.
    /// This does not synchronize pending inputs or acquire mutable native access.
    /// It neither requires nor grants completed-step checkpoint authority; the
    /// separate `checkpoint` operation validates the native owner and its joins.
    pub fn checkpoint_configurations(
        &self,
    ) -> Result<impl Iterator<Item = (StableId, StableId, &PhysicsSpec)>, CheckpointError> {
        if self.failed {
            return Err(CheckpointError::Owner("failed runtime"));
        }
        Ok(self.scenes.iter().flat_map(|(&scene, world)| {
            world
                .entries
                .iter()
                .map(move |(&entity, entry)| (scene, entity, &entry.config))
        }))
    }

    /// Capture healthy completed supported worlds under shared payload/work
    /// budgets. Active/retained character query caches remain refused. Compound
    /// children and IDs are charged cumulatively; native admission independently
    /// validates their acceleration state. Logical bytes exclude allocator overhead.
    pub fn checkpoint(
        &mut self,
        budget: &mut Budget,
        work: &mut WorkBudget,
    ) -> Result<PhysicsCheckpoint, CheckpointError> {
        if self.failed {
            return Err(CheckpointError::Owner("failed runtime"));
        }
        let count = total_entries(self.scenes.values().map(|s| s.entries.len()))?;
        charge_work(
            self.scenes.len(),
            count,
            self.overlaps.len(),
            self.events.len(),
            work,
        )?;
        charge_geometry(
            self.scenes
                .values()
                .flat_map(|s| s.entries.values().map(|e| &e.config)),
            work,
        )?;
        charge_native_pairs(self, work)?;
        checkpoint_validation::validate(self).map_err(CheckpointError::Owner)?;
        let mut scenes = budget.vector(self.scenes.len())?;
        for (&id, scene) in &mut self.scenes {
            let mut entries = budget.vector(scene.entries.len())?;
            for (&id, entry) in &scene.entries {
                entries.push(SavedEntry {
                    id,
                    body: entry.body,
                    collider: entry.collider,
                    config: copy_config(&entry.config, budget)?,
                });
            }
            scenes.push(SavedScene {
                id,
                entries,
                native: scene.world.inner.checkpoint(budget, work)?,
            });
        }
        let mut overlaps = budget.vector(self.overlaps.len())?;
        overlaps.extend(self.overlaps.iter().copied());
        let mut events = budget.vector(self.events.len())?;
        events.extend(self.events.iter().cloned());
        Ok(PhysicsCheckpoint {
            scenes,
            overlaps,
            events,
        })
    }
    /// Publish only after every scene and outer identity/event join succeeds.
    /// A late scene refusal leaves this complete runtime unchanged.
    pub fn restore_checkpoint(
        &mut self,
        checkpoint: &PhysicsCheckpoint,
        budget: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<(), CheckpointError> {
        let candidate = checkpoint.reconstruct(budget, work, allocations)?;
        *self = candidate;
        Ok(())
    }
}
impl PhysicsCheckpoint {
    pub fn reconstruct(
        &self,
        budget: &mut Budget,
        work: &mut WorkBudget,
        allocations: &mut AllocationBudget,
    ) -> Result<PhysicsRuntime<StableId>, CheckpointError> {
        let count = total_entries(self.scenes.iter().map(|s| s.entries.len()))?;
        charge_work(
            self.scenes.len(),
            count,
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
        if !self.scenes.windows(2).all(|w| w[0].id < w[1].id)
            || !self.overlaps.windows(2).all(|w| w[0] < w[1])
        {
            return Err(CheckpointError::Owner("scene or overlap ordering"));
        }
        charge::<(StableId, SceneWorld<StableId>)>(self.scenes.len(), budget)?;
        charge::<(StableId, StableId, StableId)>(self.overlaps.len(), budget)?;
        let mut events = budget.vector(self.events.len())?;
        events.extend(self.events.iter().cloned());
        let mut candidate = PhysicsRuntime {
            scenes: BTreeMap::new(),
            overlaps: BTreeSet::new(),
            events,
            failed: false,
        };
        for saved in &self.scenes {
            if !saved.entries.windows(2).all(|w| w[0].id < w[1].id) {
                return Err(CheckpointError::Owner("entity ordering"));
            }
            charge::<(StableId, Entry)>(saved.entries.len(), budget)?;
            charge::<(ColliderHandle, StableId)>(saved.entries.len(), budget)?;
            let native = saved.native.reconstruct(budget, work, allocations)?;
            let mut scene = SceneWorld {
                world: NativeOwner { inner: native },
                entries: BTreeMap::new(),
                ids: HashMap::new(),
                query_dirty: false,
                character: None,
            };
            scene
                .ids
                .try_reserve(saved.entries.len())
                .map_err(|_| RecordError::Allocation)?;
            for row in &saved.entries {
                scene.entries.insert(
                    row.id,
                    Entry {
                        body: row.body,
                        collider: row.collider,
                        config: copy_config(&row.config, budget)?,
                    },
                );
                if scene.ids.insert(row.collider, row.id).is_some() {
                    return Err(CheckpointError::Owner("duplicate collider identity"));
                }
            }
            candidate.scenes.insert(saved.id, scene);
        }
        candidate.overlaps.extend(self.overlaps.iter().copied());
        charge_native_pairs(&candidate, work)?;
        checkpoint_validation::validate(&candidate).map_err(CheckpointError::Owner)?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests;

mod persisted;
mod wire;

impl From<rapier3d::pipeline::checked_restore::persisted::PersistedError> for CheckpointError {
    fn from(error: rapier3d::pipeline::checked_restore::persisted::PersistedError) -> Self {
        Self::Persisted(error)
    }
}
