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
    pub(crate) localization: crate::NativeLocalization,
    pub(crate) id: StableId,
    pub(crate) tick_rate: u32,
    pub(crate) entities: Vec<CookedEntity>,
    pub(crate) physics: Vec<crate::CookedPhysics>,
    pub(crate) physics_2d: Vec<crate::CookedPhysics2D>,
    pub(crate) audio: Vec<crate::CookedAudio>,
    pub(crate) spatial: Vec<crate::CookedSpatial>,
    pub(crate) visual_assets: Vec<crate::VisualAssetRef>,
    pub(crate) navigation: Vec<crate::CookedNavigation>,
    pub(crate) prepared_navigation: crate::navigation_runtime::NavigationRuntime,
    pub(crate) topology: Vec<(usize, Option<usize>)>,
}

pub const MAX_ENTITIES: usize = 1_000_000;

#[derive(Debug, Error, PartialEq)]
pub enum SceneError {
    #[error("native localization: {0}")]
    Localization(String),
    #[error("native navigation: {0}")]
    Navigation(String),
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
            localization: Default::default(),
            id,
            tick_rate,
            entities,
            topology,
            physics: Vec::new(),
            physics_2d: Vec::new(),
            audio: Vec::new(),
            spatial: Vec::new(),
            visual_assets: Vec::new(),
            navigation: Vec::new(),
            prepared_navigation: Default::default(),
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
        let mut size = 72
            + self.entities.len() * 128
            + 16
            + crate::navigation_binary::size(&self.navigation)
            + self.spatial_bytes()
            + self
                .audio
                .iter()
                .map(crate::audio_binary::encoded_size)
                .sum::<usize>();
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
        self.validate_physics_2d()?;
        self.prepare_navigation()?;
        Ok(self)
    }
    /// Canonical checked audio nodes, separate from mixer/device state.
    pub fn with_audio(mut self, mut audio: Vec<crate::CookedAudio>) -> Result<Self, SceneError> {
        let ids: std::collections::BTreeSet<_> = self.entities.iter().map(|e| e.id).collect();
        crate::audio::validate(&audio, |id| ids.contains(&id))?;
        audio.sort_unstable_by_key(|node| node.entity);
        let size = 72
            + self.localization_bytes()
            + self.entities.len() * 128
            + 16
            + crate::navigation_binary::size(&self.navigation)
            + self.spatial_bytes()
            + self
                .physics
                .iter()
                .map(crate::physics_binary::encoded_size)
                .sum::<usize>()
            + audio
                .iter()
                .map(crate::audio_binary::encoded_size)
                .sum::<usize>();
        if size > crate::binary::MAX_SCENE_BYTES {
            return Err(SceneError::Invalid("file size"));
        }
        self.audio = audio;
        self.validate_physics_2d()?;
        self.prepare_navigation()?;
        Ok(self)
    }
    pub fn audio(&self) -> &[crate::CookedAudio] {
        &self.audio
    }

    /// Install canonical immutable startup asset references and checked render
    /// declarations. Actual asset loading must match these fingerprints/kinds;
    /// GPU expansion/precision and material capabilities remain host checks.
    pub fn with_spatial(
        mut self,
        mut assets: Vec<crate::VisualAssetRef>,
        mut nodes: Vec<crate::CookedSpatial>,
    ) -> Result<Self, SceneError> {
        let ids: std::collections::BTreeSet<_> = self.entities.iter().map(|e| e.id).collect();
        crate::spatial::validate_assets(&assets, self.id, |id| ids.contains(&id))?;
        assets.sort_unstable_by_key(|asset| asset.id);
        crate::spatial::validate(&nodes, &assets, |id| ids.contains(&id))?;
        nodes.sort_unstable_by_key(|node| node.entity);
        for node in &mut nodes {
            if let Some(map) = &mut node.tilemap {
                map.layers.sort_unstable_by_key(|layer| layer.id);
            }
        }
        self.visual_assets = assets;
        self.spatial = nodes;
        let size = 72
            + self.localization_bytes()
            + self.entities.len() * 128
            + 16
            + crate::navigation_binary::size(&self.navigation)
            + self.spatial_bytes()
            + self
                .physics
                .iter()
                .map(crate::physics_binary::encoded_size)
                .sum::<usize>()
            + self
                .audio
                .iter()
                .map(crate::audio_binary::encoded_size)
                .sum::<usize>();
        if size > crate::binary::MAX_SCENE_BYTES {
            return Err(SceneError::Invalid("file size"));
        }
        self.validate_physics_2d()?;
        self.prepare_navigation()?;
        Ok(self)
    }
    pub fn spatial(&self) -> &[crate::CookedSpatial] {
        &self.spatial
    }
    pub fn visual_assets(&self) -> &[crate::VisualAssetRef] {
        &self.visual_assets
    }
    fn spatial_bytes(&self) -> usize {
        usize::from(
            !self.navigation.is_empty() || self.spatial.iter().any(|n| n.flipbook.is_some()),
        ) * 4
            + self.visual_assets.len() * crate::spatial_binary::ASSET_BYTES
            + self
                .spatial
                .iter()
                .map(crate::spatial_binary::encoded_size)
                .sum::<usize>()
    }

    /// Canonical planar declarations; v1–6 bytes remain unchanged when absent.
    pub fn with_physics_2d(
        mut self,
        mut declarations: Vec<crate::CookedPhysics2D>,
    ) -> Result<Self, SceneError> {
        declarations.sort_unstable_by_key(|p| p.entity);
        if declarations.len() > incant_physics_2d_runtime::MAX_BODIES
            || declarations.windows(2).any(|p| p[0].entity == p[1].entity)
        {
            return Err(SceneError::Invalid("duplicate or excessive 2D records"));
        }
        self.physics_2d = declarations;
        self.validate_physics_2d()?;
        self.prepare_navigation()?;
        Ok(self)
    }
    pub fn physics_2d(&self) -> &[crate::CookedPhysics2D] {
        &self.physics_2d
    }
    fn validate_physics_2d(&self) -> Result<(), SceneError> {
        let entities: BTreeMap<_, _> = self.entities.iter().map(|e| (e.id, e)).collect();
        let mut ids = std::collections::BTreeSet::new();
        let mut shapes = 0usize;
        for declaration in &self.physics_2d {
            let entity = entities
                .get(&declaration.entity)
                .ok_or(SceneError::Invalid("2D entity reference"))?;
            declaration.validate(entity)?;
            ids.insert(declaration.entity);
            shapes += 1;
        }
        for node in &self.spatial {
            if let Some(map) = &node.tilemap {
                // Checked load/cook boundary only; runtime retains this data.
                let prepared = map
                    .prepare()
                    .map_err(|_| SceneError::Component(node.entity))?;
                if prepared.collision().is_empty() {
                    continue;
                }
                if !ids.insert(node.entity) {
                    return Err(SceneError::Invalid(
                        "solid tilemap and explicit 2D collider require separate entities",
                    ));
                }
                let entity = entities[&node.entity];
                crate::physics_2d::validate_motion(
                    entity.id,
                    entity.parent.is_some(),
                    &entity.transform,
                    entity.velocity.as_ref(),
                    None,
                    None,
                )?;
                shapes += prepared.collision().len();
            }
        }
        if ids.len() > incant_physics_2d_runtime::MAX_BODIES
            || shapes > incant_physics_2d_runtime::MAX_SHAPES
        {
            return Err(SceneError::Invalid("2D collision capacity"));
        }
        if self.physics.iter().any(|p| ids.contains(&p.entity)) {
            return Err(SceneError::Invalid(
                "2D and 3D colliders require separate entities",
            ));
        }
        let size = 72
            + self.localization_bytes()
            + self.entities.len() * 128
            + 20
            + crate::navigation_binary::size(&self.navigation)
            + self.spatial_bytes()
            + self
                .physics
                .iter()
                .map(crate::physics_binary::encoded_size)
                .sum::<usize>()
            + self
                .physics_2d
                .iter()
                .map(crate::physics_2d_binary::encoded_size)
                .sum::<usize>()
            + self
                .audio
                .iter()
                .map(crate::audio_binary::encoded_size)
                .sum::<usize>();
        if size > crate::binary::MAX_SCENE_BYTES {
            return Err(SceneError::Invalid("file size"));
        }
        Ok(())
    }

    fn localization_bytes(&self) -> usize {
        if self.localization.is_default() {
            0
        } else {
            crate::localization_binary::size(&self.localization) + 8
        }
    }
    pub fn with_localization(
        mut self,
        localization: crate::NativeLocalization,
    ) -> Result<Self, SceneError> {
        self.localization = localization;
        self.validate_physics_2d()?;
        Ok(self)
    }
    pub fn localization(&self) -> &crate::NativeLocalization {
        &self.localization
    }

    pub fn with_navigation(
        mut self,
        mut nodes: Vec<crate::CookedNavigation>,
    ) -> Result<Self, SceneError> {
        crate::navigation::canonicalize(&mut nodes)?;
        self.navigation = nodes;
        self.validate_physics_2d()?;
        self.prepare_navigation()?;
        Ok(self)
    }
    pub fn navigation(&self) -> &[crate::CookedNavigation] {
        &self.navigation
    }
    fn prepare_navigation(&mut self) -> Result<(), SceneError> {
        if self.navigation.is_empty() {
            self.prepared_navigation = Default::default();
            return Ok(());
        }
        let by_id: BTreeMap<_, _> = self.entities.iter().map(|e| (e.id, e)).collect();
        let mut globals = BTreeMap::new();
        let mut ordered = Vec::with_capacity(self.topology.len());
        for &(index, parent) in &self.topology {
            let entity = &self.entities[index];
            let local = matrix(&entity.transform);
            let global = parent.map_or(local, |p| ordered[p] * local);
            ordered.push(global);
            globals.insert(entity.id, global);
        }
        self.prepared_navigation = self.prepared_navigation.prepare(
            self.navigation
                .iter()
                .cloned()
                .map(std::sync::Arc::new)
                .collect(),
            |id| {
                let e = *by_id.get(&id)?;
                let physics = self.physics.iter().find(|p| p.entity == id);
                let planar = self.physics_2d.iter().find(|p| p.entity == id);
                let mesh = self
                    .spatial
                    .iter()
                    .find(|p| p.entity == id)
                    .and_then(|n| n.mesh.as_ref())
                    .and_then(|m| self.visual_assets.iter().find(|a| a.id == m.mesh))
                    .copied();
                Some(crate::navigation_runtime::Input {
                    parent: e.parent,
                    world: globals[&id],
                    stationary: e.velocity.as_ref().is_none_or(|v| v.linear == [0.; 3])
                        && physics
                            .and_then(|p| p.body.as_ref())
                            .is_none_or(|b| b.motion == incant_types::BodyMotion::Fixed)
                        && planar
                            .and_then(|p| p.body.as_ref())
                            .is_none_or(|b| b.motion == incant_types::BodyMotion::Fixed),
                    collider: physics.map(|p| &p.collider),
                    mesh,
                })
            },
        )?;
        Ok(())
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
