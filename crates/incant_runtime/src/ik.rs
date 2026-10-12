//! Immutable IK declarations and atomically replaced numeric constraint state.
use crate::{CookedSpatial, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId};
use bevy_ecs::prelude::Component;
pub use incant_types::{AnimationIkSample, MAX_ANIMATION_IK, MAX_SCENE_ANIMATION_IK};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq)]
pub struct NativeIk {
    pub mesh_fingerprint: [u8; 32],
    pub initial: Arc<[AnimationIkSample]>,
}
impl NativeIk {
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.initial.is_empty() || self.initial.len() > MAX_ANIMATION_IK {
            return Err(SceneError::Invalid("IK constraint count"));
        }
        for (i, c) in self.initial.iter().enumerate() {
            c.validate().map_err(SceneError::Invalid)?;
            if self.initial[..i].iter().any(|old| old.id == c.id) {
                return Err(SceneError::Invalid("duplicate IK constraint"));
            }
        }
        Ok(())
    }
    pub(crate) fn validate_state(&self, state: &[AnimationIkSample]) -> Result<(), SceneError> {
        if state.len() != self.initial.len() {
            return Err(SceneError::Invalid("IK state count"));
        }
        for (actual, initial) in state.iter().zip(self.initial.iter()) {
            actual.validate().map_err(SceneError::Invalid)?;
            if actual.id != initial.id || actual.chain != initial.chain {
                return Err(SceneError::Invalid("IK state binding"));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IkControl {
    pub constraint: StableId,
    pub target: [f64; 3],
    pub pole: [f64; 3],
    pub weight: f64,
}
#[derive(Component)]
pub(crate) struct IkState(pub Arc<[AnimationIkSample]>);
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IkSnapshot {
    pub entity: StableId,
    pub sample: AnimationIkSample,
}
impl NativeWorld {
    pub fn ik_state(&self, entity: StableId) -> Option<&Arc<[AnimationIkSample]>> {
        if self.physics_failed {
            return None;
        }
        self.world
            .get::<IkState>(*self.entities.get(&entity)?)
            .map(|s| &s.0)
    }
    /// Reuses caller capacity. The global constraint cap also bounds snapshot
    /// rows and their byte size (4096 * size_of::<IkSnapshot>()).
    pub fn snapshot_ik(&self, rows: &mut Vec<IkSnapshot>) -> Result<(), RuntimeError> {
        self.ensure_usable()?;
        rows.clear();
        for id in &self.spatial_ids {
            if let Some(state) = self.ik_state(*id) {
                if state.len() > MAX_SCENE_ANIMATION_IK.saturating_sub(rows.len()) {
                    rows.clear();
                    return Err(SceneError::Invalid("IK snapshot capacity").into());
                }
                rows.extend(state.iter().map(|sample| IkSnapshot {
                    entity: *id,
                    sample: *sample,
                }));
            }
        }
        Ok(())
    }
    pub(crate) fn prepare_ik_controls(
        &self,
        commands: &FrameCommands,
        changes: &BTreeMap<StableId, Option<CookedSpatial>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<BTreeMap<StableId, Arc<[AnimationIkSample]>>, RuntimeError> {
        let mut states: BTreeMap<StableId, Arc<[AnimationIkSample]>> = BTreeMap::new();
        let mut count = 0usize;
        for command in commands.iter() {
            let crate::StructuralCommand::ControlIk { entity, control } = command else {
                continue;
            };
            if !exists(*entity) {
                return Err(RuntimeError::MissingEntity(*entity));
            }
            let old = self
                .entities
                .get(entity)
                .and_then(|h| self.world.get::<CookedSpatial>(*h))
                .and_then(|s| s.ik.as_ref());
            let selected = changes
                .get(entity)
                .map_or(old, |s| s.as_ref().and_then(|s| s.ik.as_ref()))
                .ok_or(SceneError::Invalid("IK control requires final IK"))?;
            if !states.contains_key(entity) {
                count = count
                    .checked_add(selected.initial.len())
                    .ok_or(SceneError::Invalid("IK control capacity"))?;
                if count > MAX_SCENE_ANIMATION_IK {
                    return Err(SceneError::Invalid("IK control capacity").into());
                }
                let initial = if old == Some(selected) {
                    self.ik_state(*entity).expect("installed IK")
                } else {
                    &selected.initial
                };
                states.insert(*entity, initial.clone());
            }
            let state = states.get_mut(entity).expect("prepared IK");
            let sample = Arc::make_mut(state)
                .iter_mut()
                .find(|s| s.id == control.constraint)
                .ok_or(SceneError::Invalid("IK constraint identity"))?;
            sample.target = control.target;
            sample.pole = control.pole;
            sample.weight = control.weight;
            sample.validate().map_err(SceneError::Invalid)?;
        }
        Ok(states)
    }
    pub(crate) fn install_ik_states(
        &mut self,
        states: BTreeMap<StableId, Arc<[AnimationIkSample]>>,
    ) {
        for (id, state) in states {
            self.world
                .entity_mut(self.entities[&id])
                .insert(IkState(state));
        }
    }
}
