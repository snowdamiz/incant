use glam::{DMat4, DQuat, DVec3};
use incant_types::{Transform, Velocity};
use std::collections::{BTreeMap, VecDeque};
use thiserror::Error;

/// The canonical 128 bits of an authored ULID; no string parsing during play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(pub [u8; 16]);

#[derive(Debug, Clone, PartialEq)]
pub struct CookedEntity {
    pub id: StableId,
    pub parent: Option<StableId>,
    pub transform: Transform,
    pub velocity: Option<Velocity>,
}

/// Fully checked, canonical archetype order. Construction/loading validates the
/// entire scene before any running ECS world can be published.
#[derive(Debug, Clone)]
pub struct CookedScene {
    pub(crate) id: StableId,
    pub(crate) tick_rate: u32,
    pub(crate) entities: Vec<CookedEntity>,
    pub(crate) topology: Vec<(usize, Option<usize>)>,
}

pub const MAX_ENTITIES: usize = 1_000_000;

#[derive(Debug, Error, PartialEq)]
pub enum SceneError {
    #[error("invalid cooked scene: {0}")]
    Invalid(&'static str),
    #[error("unsupported cooked-scene version {0}")]
    Version(u32),
    #[error("cooked scene integrity check failed")]
    Integrity,
    #[error("duplicate runtime entity {0:?}")]
    Duplicate(StableId),
    #[error("invalid transform or velocity on {0:?}")]
    Component(StableId),
    #[error("invalid parent reference on {0:?}")]
    Parent(StableId),
}

impl CookedScene {
    pub fn new(
        id: StableId,
        tick_rate: u32,
        mut entities: Vec<CookedEntity>,
    ) -> Result<Self, SceneError> {
        if !(1..=240).contains(&tick_rate) || entities.len() > MAX_ENTITIES {
            return Err(SceneError::Invalid("tick rate or entity limit"));
        }
        // Contiguous Transform-only and Transform+Velocity archetypes, with
        // stable IDs making equivalent authoring input byte-identical.
        entities.sort_unstable_by_key(|e| (e.velocity.is_some(), e.id));
        let mut indices = BTreeMap::new();
        for (index, entity) in entities.iter().enumerate() {
            if indices.insert(entity.id, index).is_some() {
                return Err(SceneError::Duplicate(entity.id));
            }
            validate_values(entity)?;
        }
        let mut children = vec![Vec::new(); entities.len()];
        let mut pending = VecDeque::new();
        for (index, entity) in entities.iter().enumerate() {
            if let Some(parent) = entity.parent {
                let parent = indices
                    .get(&parent)
                    .copied()
                    .ok_or(SceneError::Parent(entity.id))?;
                children[parent].push(index);
            } else {
                pending.push_back((index, None));
            }
        }
        let mut topology: Vec<(usize, Option<usize>)> = Vec::with_capacity(entities.len());
        let mut globals: Vec<DMat4> = Vec::with_capacity(entities.len());
        while let Some((index, parent)) = pending.pop_front() {
            let local = matrix(&entities[index].transform);
            let global = parent.map_or(local, |parent| globals[parent] * local);
            if !global.is_finite() {
                return Err(SceneError::Component(entities[index].id));
            }
            let slot = topology.len();
            topology.push((index, parent));
            globals.push(global);
            pending.extend(children[index].iter().map(|child| (*child, Some(slot))));
        }
        if topology.len() != entities.len() {
            return Err(SceneError::Invalid("parent cycle"));
        }
        Ok(Self {
            id,
            tick_rate,
            entities,
            topology,
        })
    }

    pub fn id(&self) -> StableId {
        self.id
    }
    pub fn tick_rate(&self) -> u32 {
        self.tick_rate
    }
    pub fn entities(&self) -> &[CookedEntity] {
        &self.entities
    }
}

fn validate_values(entity: &CookedEntity) -> Result<(), SceneError> {
    let t = &entity.transform;
    let norm: f64 = t.rotation.iter().map(|v| v * v).sum();
    if !t
        .translation
        .iter()
        .chain(&t.rotation)
        .chain(&t.scale)
        .all(|v| v.is_finite())
        || !norm.is_finite()
        || (norm - 1.).abs() > 1e-4
        || t.scale.iter().any(|v| v.abs() < 1e-9)
        || entity
            .velocity
            .as_ref()
            .is_some_and(|v| !v.linear.iter().all(|x| x.is_finite()))
    {
        return Err(SceneError::Component(entity.id));
    }
    Ok(())
}

pub(crate) fn matrix(t: &Transform) -> DMat4 {
    DMat4::from_scale_rotation_translation(
        DVec3::from_array(t.scale),
        DQuat::from_array(t.rotation).normalize(),
        DVec3::from_array(t.translation),
    )
}
