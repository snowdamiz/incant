//! Opaque in-memory checkpoints. Persisted game-save admission is a separate
//! boundary: these values cannot be created from user bytes or caller flags.
use super::*;
use incant_physics_snapshot::{Budget, Error as RecordError, construction::AllocationBudget};
use rapier2d::pipeline::checked_restore::{
    WorkBudget,
    capture::{CaptureError, NativeFrame},
};

#[derive(Debug, Error, PartialEq)]
pub enum CheckpointError {
    #[error("2D checkpoint owner mismatch: {0}")]
    Owner(&'static str),
    #[error("native 2D checkpoint refused: {0:?}")]
    Native(CaptureError),
    #[error(transparent)]
    Record(#[from] RecordError),
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
    config: PreparedBody,
}
/// Exact completed native state with its stable entity identities, declarations
/// and published trigger observations. No Clone or raw-byte constructor exists.
pub struct PhysicsCheckpoint {
    native: NativeFrame,
    entries: Vec<SavedEntry>,
    overlaps: Vec<(StableId, StableId)>,
    events: Vec<TriggerEvent>,
}
fn copy_config(value: &PreparedBody) -> Result<PreparedBody, CheckpointError> {
    let config::Geometry::Primitive(shape) = &value.geometry else {
        return Err(CheckpointError::Owner(
            "tile geometry checkpoint unsupported",
        ));
    };
    Ok(PreparedBody {
        body: value.body.clone(),
        collider: value.collider.clone(),
        geometry: config::Geometry::Primitive(shape.clone()),
        state: value.state.clone(),
    })
}
fn charge_work(
    entries: usize,
    overlaps: usize,
    events: usize,
    work: &mut WorkBudget,
) -> Result<(), CheckpointError> {
    if entries > MAX_BODIES || overlaps > MAX_TRIGGER_PAIRS || events > MAX_TRIGGER_EVENTS {
        return Err(CheckpointError::Owner("collection limit"));
    }
    work.charge(1 + entries as u64 * 4 + overlaps as u64 * 2 + events as u64)
        .map_err(CaptureError::from)?;
    Ok(())
}

fn charge<T>(count: usize, budget: &mut Budget) -> Result<(), RecordError> {
    budget.charge(
        count
            .checked_mul(size_of::<T>())
            .ok_or(RecordError::Limit)?,
    )
}
// The pinned graph/map/filter iterator supplies a constant-time upper bound.
// Charge every candidate, including false intersections and filtered handles,
// before the outer join walks the graph. An absent bound is refused explicitly.
fn charge_native_pairs(
    runtime: &PhysicsWorld2D,
    work: &mut WorkBudget,
) -> Result<(), CheckpointError> {
    for world in std::iter::once(&runtime.world) {
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
impl PhysicsWorld2D {
    /// Capture only a healthy completed primitive world. One cumulative logical
    /// payload budget covers outer rows, native validation and encoded output;
    /// allocator bookkeeping/RSS is not represented by this budget.
    pub fn checkpoint(
        &mut self,
        budget: &mut Budget,
        work: &mut WorkBudget,
    ) -> Result<PhysicsCheckpoint, CheckpointError> {
        if self.failed {
            return Err(CheckpointError::Owner("failed runtime"));
        }
        charge_work(
            self.entries.len(),
            self.overlaps.len(),
            self.events.len(),
            work,
        )?;
        charge_native_pairs(self, work)?;
        checkpoint_validation::validate(self).map_err(CheckpointError::Owner)?;
        let mut entries = budget.vector(self.entries.len())?;
        for (&id, entry) in &self.entries {
            entries.push(SavedEntry {
                id,
                body: entry.body,
                collider: entry.collider,
                config: copy_config(&entry.config)?,
            });
        }
        let mut overlaps = budget.vector(self.overlaps.len())?;
        overlaps.extend(self.overlaps.iter().copied());
        let mut events = budget.vector(self.events.len())?;
        events.extend(self.events.iter().cloned());
        let native = self.world.inner.checkpoint(budget, work)?;
        Ok(PhysicsCheckpoint {
            native,
            entries,
            overlaps,
            events,
        })
    }
    /// Stage and validate every owner before replacing this runtime. Any refusal
    /// leaves all current bodies, identities and observations unchanged.
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
    ) -> Result<PhysicsWorld2D, CheckpointError> {
        if self.entries.len() > MAX_BODIES
            || self.overlaps.len() > MAX_TRIGGER_PAIRS
            || self.events.len() > MAX_TRIGGER_EVENTS
        {
            return Err(CheckpointError::Owner("collection limit"));
        }
        if !self.entries.windows(2).all(|w| w[0].id < w[1].id)
            || !self.overlaps.windows(2).all(|w| w[0] < w[1])
        {
            return Err(CheckpointError::Owner("identity or overlap ordering"));
        }
        charge_work(
            self.entries.len(),
            self.overlaps.len(),
            self.events.len(),
            work,
        )?;
        charge::<(StableId, Entry)>(self.entries.len(), budget)?;
        charge::<(ColliderHandle, StableId)>(self.entries.len(), budget)?;
        charge::<(StableId, StableId)>(self.overlaps.len(), budget)?;
        let mut events = budget.vector(self.events.len())?;
        events.extend(self.events.iter().cloned());
        let native = self.native.reconstruct(budget, work, allocations)?;
        let mut candidate = PhysicsWorld2D {
            world: NativeOwner { inner: native },
            entries: BTreeMap::new(),
            ids: HashMap::new(),
            overlaps: BTreeSet::new(),
            events,
            query_dirty: false,
            failed: false,
        };
        candidate
            .ids
            .try_reserve(self.entries.len())
            .map_err(|_| RecordError::Allocation)?;
        for row in &self.entries {
            candidate.entries.insert(
                row.id,
                Entry {
                    body: row.body,
                    collider: row.collider,
                    config: copy_config(&row.config)?,
                },
            );
            if candidate.ids.insert(row.collider, row.id).is_some() {
                return Err(CheckpointError::Owner("duplicate collider identity"));
            }
        }
        candidate.overlaps.extend(self.overlaps.iter().copied());
        charge_native_pairs(&candidate, work)?;
        checkpoint_validation::validate(&candidate).map_err(CheckpointError::Owner)?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests;
