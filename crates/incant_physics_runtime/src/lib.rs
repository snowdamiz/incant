//! Document-free fixed-step physics. Ordered IDs are selected by the owning
//! runtime; solver handles, contacts and sleeping state never cross this boundary.
// A solver variant is a build choice, never a runtime toggle. Rapier itself also
// rejects this combination; keep the local diagnostic at our public boundary.
#[cfg(all(feature = "deterministic", feature = "fast-simd8"))]
compile_error!("deterministic and fast-simd8 physics variants are mutually exclusive");

/// Build identity; Cargo feature admission is also checked by the exporter.
pub const DETERMINISTIC: bool = cfg!(feature = "deterministic");
pub const SIMD_LANES: u8 = if cfg!(feature = "fast-simd8") { 8 } else { 4 };

mod navigation;
pub use navigation::navigation_geometry;
mod character;
mod character_frame;
mod character_queries;
mod queries;
mod shapes;
pub use character::{
    CharacterMovement, CharacterOptions, CharacterQuery, CharacterStep, PreparedCharacterMovement,
};
pub use character_frame::MAX_CHARACTER_QUERY_COLLIDERS;
use incant_types::{BodyMotion, Collider as DocCollider, RigidBody as DocBody};
pub use queries::{RayHit, RayQuery};
use rapier3d::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PhysicsError {
    #[error("physics step must be finite and between 1/240 and 1 seconds")]
    InvalidStep,
    #[error("physics state exceeded the supported numeric range; restart this play session")]
    NumericRange,
    #[error("physics collection limit exceeded")]
    Limit,
    #[error("invalid typed physics component: {0}")]
    Component(String),
    #[error("invalid physics query: {0}")]
    Query(String),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BodyState {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
    pub velocity: [f64; 3],
    pub angular_velocity: [f64; 3],
    pub sleeping: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "TriggerEvent")]
pub struct TriggerEvent<I = String> {
    pub scene_id: I,
    /// Stable entity IDs in lexical order, at least one is a sensor.
    pub first: I,
    pub second: I,
    pub entered: bool,
}
/// Owned typed body configuration. Construct/load through PreparedPhysics to
/// validate before changing a live solver. No handles or serialized documents.
#[derive(Debug, Clone)]
pub struct PhysicsSpec {
    pub body: Option<DocBody>,
    pub collider: DocCollider,
    pub state: BodyState,
}

/// Validated owned declaration for one structural installation. Construction is
/// the only fallible step; installing it never revalidates a running world.
pub struct PreparedBody(PhysicsSpec);
impl PreparedBody {
    pub fn new(mut spec: PhysicsSpec) -> Result<Self, PhysicsError> {
        spec.validate()?;
        if let incant_types::ColliderShape::Compound { parts } = &mut spec.collider.shape {
            parts.sort_by(|a, b| a.id.cmp(&b.id));
        }
        Ok(Self(spec))
    }
}

pub const MAX_TRIGGER_PAIRS: usize = 65_536;
pub const MAX_TRIGGER_EVENTS: usize = MAX_TRIGGER_PAIRS * 2;

pub struct PreparedPhysics<I: Ord>(BTreeMap<I, BTreeMap<I, PhysicsSpec>>);
impl<I: Ord> PreparedPhysics<I> {
    pub fn new(mut scenes: BTreeMap<I, BTreeMap<I, PhysicsSpec>>) -> Result<Self, PhysicsError> {
        let mut count = 0usize;
        for scene in scenes.values_mut() {
            count = count.checked_add(scene.len()).ok_or(PhysicsError::Limit)?;
            if count > 1_000_000 {
                return Err(PhysicsError::Limit);
            }
            for spec in scene.values_mut() {
                spec.validate()?;
                if let incant_types::ColliderShape::Compound { parts } = &mut spec.collider.shape {
                    parts.sort_by(|a, b| a.id.cmp(&b.id));
                }
            }
        }
        Ok(Self(scenes))
    }
}

impl PhysicsSpec {
    pub fn validate(&self) -> Result<(), PhysicsError> {
        use incant_types::physics_validation as check;
        if let Some(body) = &self.body {
            check::rigid_body(body).map_err(PhysicsError::Component)?;
        }
        check::collider(&self.collider).map_err(PhysicsError::Component)?;
        self.validate_state(&self.state)?;
        Ok(())
    }
    pub fn validate_state(&self, state: &BodyState) -> Result<(), PhysicsError> {
        use incant_types::{AngularVelocity, Transform, Velocity, physics_validation as check};
        let norm: f64 = state.rotation.iter().map(|v| v * v).sum();
        if !state.rotation.iter().all(|v| v.is_finite())
            || !norm.is_finite()
            || (norm - 1.).abs() > 1e-4
        {
            return Err(PhysicsError::Component(
                "rotation must be a unit quaternion".into(),
            ));
        }
        let angular = AngularVelocity {
            angular: state.angular_velocity,
        };
        check::motion(
            false,
            &Transform {
                translation: state.translation,
                rotation: state.rotation,
                scale: [1.; 3],
            },
            self.body.as_ref(),
            Some(&Velocity {
                linear: state.velocity,
            }),
            self.body.as_ref().map(|_| &angular),
        )
        .map_err(PhysicsError::Component)?;
        if self.body.is_none() && state.angular_velocity != [0.; 3] {
            return Err(PhysicsError::Component(
                "fixed bodies cannot have angular velocity".into(),
            ));
        }
        Ok(())
    }
}
struct Entry {
    body: Option<RigidBodyHandle>,
    collider: ColliderHandle,
    config: PhysicsSpec,
}
struct SceneWorld<I: Ord> {
    world: PhysicsWorld,
    entries: BTreeMap<I, Entry>,
    ids: HashMap<ColliderHandle, I>,
    query_dirty: bool,
    character: Option<character_frame::CharacterFrame>,
}
pub struct PhysicsRuntime<I: Ord> {
    scenes: BTreeMap<I, SceneWorld<I>>,
    overlaps: BTreeSet<(I, I, I)>,
    events: Vec<TriggerEvent<I>>,
    failed: bool,
}
impl<I: Ord> Default for SceneWorld<I> {
    fn default() -> Self {
        Self {
            world: PhysicsWorld::default(),
            entries: BTreeMap::new(),
            ids: HashMap::new(),
            query_dirty: false,
            character: None,
        }
    }
}
impl<I: Ord> Default for PhysicsRuntime<I> {
    fn default() -> Self {
        Self {
            scenes: BTreeMap::new(),
            overlaps: BTreeSet::new(),
            events: vec![],
            failed: false,
        }
    }
}
impl<I: Ord + Clone> PhysicsRuntime<I> {
    /// Unchanged bodies retain their handles, sleep state and solver warm-start
    /// data. Changes to other components never reconstruct the physics world.
    /// A failed runtime cannot be revived by synchronizing declarations.
    pub fn sync(&mut self, prepared: PreparedPhysics<I>) {
        if self.failed {
            return;
        }
        self.scenes.retain(|id, _| prepared.0.contains_key(id));
        for (scene_id, specs) in prepared.0 {
            let scene = self.scenes.entry(scene_id).or_default();
            let removed: Vec<_> = scene
                .entries
                .keys()
                .filter(|id| !specs.contains_key(*id))
                .cloned()
                .collect();
            let mut changed = !removed.is_empty();
            for id in removed {
                scene.remove(&id);
            }
            for (id, spec) in specs {
                let rebuild = scene.entries.get(&id).is_none_or(|entry| {
                    entry.config.body != spec.body || entry.config.collider != spec.collider
                });
                if rebuild {
                    scene.remove(&id);
                    scene.insert(id, spec);
                    changed = true;
                    continue;
                }
                scene
                    .update_motion(&id, &spec.state)
                    .expect("prepared entry/state");
            }
            scene.query_dirty |= changed;
        }
    }
    /// Update only committed motion, preserving unchanged bodies, contacts and
    /// sleep state. No configuration/world clone or text/JSON conversion occurs.
    /// Input validation completes before this one body's values change.
    pub fn sync_motion(
        &mut self,
        scene: &I,
        id: &I,
        state: &BodyState,
    ) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        self.scenes
            .get_mut(scene)
            .ok_or_else(|| PhysicsError::Query("scene does not exist".into()))?
            .update_motion(id, state)
    }

    /// Apply a previously checked structural delta, preserving unrelated bodies.
    /// The owner bounds its entity collection before committing this operation.
    pub fn install(&mut self, scene: I, id: I, body: PreparedBody) {
        if self.failed {
            return;
        }
        let scene = self.scenes.entry(scene).or_default();
        let spec = body.0;
        if scene.entries.get(&id).is_some_and(|entry| {
            entry.config.body == spec.body && entry.config.collider == spec.collider
        }) {
            scene
                .update_motion(&id, &spec.state)
                .expect("prepared state");
        } else {
            scene.remove(&id);
            scene.insert(id, spec);
            scene.query_dirty = true;
        }
    }

    pub fn remove(&mut self, scene: &I, id: &I) {
        if self.failed {
            return;
        }
        if let Some(scene) = self.scenes.get_mut(scene) {
            scene.remove(id);
            scene.query_dirty = true;
        }
    }

    pub fn state(&self, scene: &I, id: &I) -> Option<&BodyState> {
        if self.failed {
            return None;
        }
        Some(&self.scenes.get(scene)?.entries.get(id)?.config.state)
    }

    /// Invalid dt is retryable. Once advancement starts, a late error or unwind
    /// poisons the whole runtime, including scenes already advanced this tick.
    pub fn step(&mut self, dt: f64) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) {
            return Err(PhysicsError::InvalidStep);
        }
        // Stay poisoned until every scene and trigger observation is published.
        // A caller that catches an unwind must not reuse a partially advanced
        // solver or read stale/partial observations as a successful frame.
        self.failed = true;
        let mut overlaps = BTreeSet::new();
        for (scene_id, scene) in &mut self.scenes {
            if let Some(cache) = &mut scene.character {
                cache.ready = false;
            }
            scene.world.integration_parameters.dt = dt as f32;
            scene.world.step();
            scene.query_dirty = false;
            if !scene.world.quarantine().bodies().is_empty()
                || !scene.world.quarantine().colliders().is_empty()
            {
                self.failed = true;
                return Err(PhysicsError::NumericRange);
            }
            for entry in scene.entries.values_mut() {
                if let Some(body) = entry.body {
                    let body = &scene.world.bodies[body];
                    let state = BodyState {
                        translation: body.translation().to_array().map(f64::from),
                        rotation: body.rotation().to_array().map(f64::from),
                        velocity: body.linvel().to_array().map(f64::from),
                        angular_velocity: body.angvel().to_array().map(f64::from),
                        sleeping: body.is_sleeping(),
                    };
                    if entry.config.validate_state(&state).is_err() {
                        self.failed = true;
                        return Err(PhysicsError::NumericRange);
                    }
                    entry.config.state = state;
                }
            }
            for (a, _, b, _, intersecting) in scene.world.intersection_pairs() {
                if !intersecting {
                    continue;
                }
                let (a, b) = (&scene.ids[&a], &scene.ids[&b]);
                let (a, b) = if a < b { (a, b) } else { (b, a) };
                overlaps.insert((scene_id.clone(), a.clone(), b.clone()));
                if overlaps.len() > MAX_TRIGGER_PAIRS {
                    self.failed = true;
                    return Err(PhysicsError::Limit);
                }
            }
        }
        self.events = overlaps
            .difference(&self.overlaps)
            .map(|(s, a, b)| TriggerEvent {
                scene_id: s.clone(),
                first: a.clone(),
                second: b.clone(),
                entered: true,
            })
            .chain(
                self.overlaps
                    .difference(&overlaps)
                    .map(|(s, a, b)| TriggerEvent {
                        scene_id: s.clone(),
                        first: a.clone(),
                        second: b.clone(),
                        entered: false,
                    }),
            )
            .collect();
        self.events.sort();
        self.overlaps = overlaps;
        self.failed = false;
        Ok(())
    }
    pub fn state_iter(&self) -> impl Iterator<Item = (&I, &BodyState)> {
        self.scenes
            .values()
            .filter(|_| !self.failed)
            .flat_map(|scene| {
                scene
                    .entries
                    .iter()
                    .map(|(id, entry)| (id, &entry.config.state))
            })
    }
    pub fn states(&self) -> BTreeMap<I, BodyState> {
        self.state_iter()
            .map(|(id, state)| (id.clone(), state.clone()))
            .collect()
    }
    pub fn events(&self) -> &[TriggerEvent<I>] {
        if self.failed { &[] } else { &self.events }
    }
}
impl<I: Ord + Clone> SceneWorld<I> {
    fn update_motion(&mut self, id: &I, state: &BodyState) -> Result<(), PhysicsError> {
        if let Some(cache) = &mut self.character {
            cache.ready = false;
        }
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| PhysicsError::Query("entity does not exist".into()))?;
        entry.config.validate_state(state)?;
        // Compare published f64 values before f32 normalization/quantization so
        // merely echoing unchanged ECS columns never wakes a sleeping body.
        let previous = &entry.config.state;
        if state.translation != previous.translation || state.rotation != previous.rotation {
            if let Some(body) = entry.body {
                self.world.bodies[body].set_position(pose(state), true);
            } else {
                self.world.colliders[entry.collider].set_position(pose(state));
            }
            self.query_dirty = true;
        }
        if let Some(body) = entry.body {
            let body = &mut self.world.bodies[body];
            if state.velocity != previous.velocity {
                body.set_linvel(vector(state.velocity), true);
            }
            if state.angular_velocity != previous.angular_velocity {
                body.set_angvel(vector(state.angular_velocity), true);
            }
        }
        entry.config.state = state.clone();
        entry.config.state.sleeping = entry
            .body
            .is_some_and(|body| self.world.bodies[body].is_sleeping());
        Ok(())
    }
    fn remove(&mut self, id: &I) {
        if let Some(entry) = self.entries.remove(id) {
            self.character = None;
            self.ids.remove(&entry.collider);
            if let Some(body) = entry.body {
                self.world.remove_body(body);
            } else {
                self.world.remove_collider(entry.collider);
            }
        }
    }
    fn insert(&mut self, id: I, mut spec: PhysicsSpec) {
        self.character = None;
        let shape = ColliderBuilder::new(shapes::build(&spec.collider.shape))
            .density(spec.collider.density as f32)
            .friction(spec.collider.friction as f32)
            .restitution(spec.collider.restitution as f32)
            .sensor(spec.collider.sensor)
            .collision_groups(groups(spec.collider.memberships, spec.collider.filter))
            .active_collision_types(ActiveCollisionTypes::all());
        let (body, collider) = if let Some(config) = &spec.body {
            let body = match config.motion {
                BodyMotion::Fixed => RigidBodyBuilder::fixed(),
                BodyMotion::Dynamic => RigidBodyBuilder::dynamic(),
                BodyMotion::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
            }
            .pose(pose(&spec.state))
            .linvel(vector(spec.state.velocity))
            .angvel(vector(spec.state.angular_velocity))
            .gravity_scale(config.gravity_scale as f32)
            .linear_damping(config.linear_damping as f32)
            .angular_damping(config.angular_damping as f32)
            .can_sleep(config.can_sleep)
            .ccd_enabled(config.ccd);
            let (b, c) = self.world.insert(body, shape);
            (Some(b), c)
        } else {
            (
                None,
                self.world
                    .insert_collider(shape.position(pose(&spec.state)), None),
            )
        };
        spec.state.sleeping = body.is_some_and(|body| self.world.bodies[body].is_sleeping());
        self.ids.insert(collider, id.clone());
        self.entries.insert(
            id,
            Entry {
                body,
                collider,
                config: spec,
            },
        );
    }
}
fn vector(value: [f64; 3]) -> Vector {
    Vector::from_array(value.map(|v| v as f32))
}
fn pose(state: &BodyState) -> Pose {
    Pose::from_parts(
        vector(state.translation),
        Rotation::from_array(state.rotation.map(|v| v as f32)).normalize(),
    )
}
fn groups(memberships: u32, filter: u32) -> InteractionGroups {
    InteractionGroups::new(
        Group::from_bits_retain(memberships),
        Group::from_bits_retain(filter),
        InteractionTestMode::And,
    )
}
