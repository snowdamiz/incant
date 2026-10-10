use glam::{DMat4, DQuat, DVec3};
use incant_types::{Transform, Velocity};
use std::collections::BTreeMap;
use thiserror::Error;

pub use incant_types::StableId;

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
    pub(crate) physics: Vec<crate::CookedPhysics>,
    pub(crate) topology: Vec<(usize, Option<usize>)>,
}

pub const MAX_ENTITIES: usize = 1_000_000;

#[derive(Debug, Error, PartialEq)]
pub enum SceneError {
    #[error("invalid scene data: {0}")]
    Invalid(&'static str),
    #[error("unsupported cooked-scene version {0}")]
    Version(u32),
    #[error("cooked scene integrity check failed")]
    Integrity,
    #[error("duplicate runtime ID {0:?}")]
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
            if entity.id == id || indices.insert(entity.id, index).is_some() {
                return Err(SceneError::Duplicate(entity.id));
            }
            validate_values(entity)?;
        }
        let parents = entities.iter().map(|e| (e.id, e.parent)).collect();
        let topology =
            crate::hierarchy::prepare(&parents, |id| matrix(&entities[indices[&id]].transform))?
                .into_iter()
                .map(|node| (indices[&node.id], node.parent))
                .collect();
        Ok(Self {
            id,
            tick_rate,
            entities,
            topology,
            physics: Vec::new(),
        })
    }

    /// Add canonical, bounded physics declarations at the load/cook boundary.
    /// Existing CookedEntity literals and version-1 scenes remain compatible.
    pub fn with_physics(
        mut self,
        mut physics: Vec<crate::CookedPhysics>,
    ) -> Result<Self, SceneError> {
        physics.sort_unstable_by_key(|p| p.entity);
        if physics.len() > self.entities.len()
            || physics.windows(2).any(|p| p[0].entity == p[1].entity)
        {
            return Err(SceneError::Invalid(
                "duplicate or excessive physics records",
            ));
        }
        let entities: BTreeMap<_, _> = self.entities.iter().map(|e| (e.id, e)).collect();
        let mut size = 72 + self.entities.len() * 128 + 4;
        for declaration in &mut physics {
            let entity = entities
                .get(&declaration.entity)
                .ok_or(SceneError::Invalid("physics entity reference"))?;
            declaration.validate(entity)?;
            if let incant_types::ColliderShape::Compound { parts } = &mut declaration.collider.shape
            {
                parts.sort_by(|a, b| a.id.cmp(&b.id));
            }
            size += crate::physics_binary::encoded_size(declaration);
            if size > crate::binary::MAX_SCENE_BYTES {
                return Err(SceneError::Invalid("file size"));
            }
        }
        self.physics = physics;
        Ok(self)
    }
    pub fn physics(&self) -> &[crate::CookedPhysics] {
        &self.physics
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

pub(crate) fn validate_values(entity: &CookedEntity) -> Result<(), SceneError> {
    validate_components(entity.id, &entity.transform, entity.velocity.as_ref())
}

pub(crate) fn validate_components(
    id: StableId,
    t: &Transform,
    velocity: Option<&Velocity>,
) -> Result<(), SceneError> {
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
        || velocity.is_some_and(|v| !v.linear.iter().all(|x| x.is_finite()))
    {
        return Err(SceneError::Component(id));
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
