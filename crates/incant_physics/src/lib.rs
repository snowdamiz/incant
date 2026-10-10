//! Fixed-step, scene-isolated physics. Documents and scripts see stable entity
//! IDs only; solver handles, contacts and sleeping state belong to the session.
mod character;
mod character_queries;
mod navigation;
mod queries;
pub use character::{CharacterMovement, CharacterOptions, CharacterQuery, CharacterStep};
use incant_doc::{
    AngularVelocity, BodyMotion, Collider as DocCollider, ColliderShape, DocumentError, Project,
    RigidBody as DocBody, Transform, Velocity,
};
pub use navigation::navigation_geometry;
pub use queries::{RayHit, RayQuery};
use rapier3d::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhysicsError {
    #[error("physics step must be finite and between 1/240 and 1 seconds")]
    InvalidStep,
    #[error("physics state exceeded the supported numeric range; restart this play session")]
    NumericRange,
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
pub struct TriggerEvent {
    pub scene_id: String,
    /// Stable entity IDs in lexical order, at least one is a sensor.
    pub first: String,
    pub second: String,
    pub entered: bool,
}
struct Spec {
    body: Option<DocBody>,
    collider: DocCollider,
    state: BodyState,
}
/// Successfully decoded input; constructing this validates the entire project
/// before a live world can be changed.
pub struct PreparedPhysics(BTreeMap<String, BTreeMap<String, Spec>>);
impl PreparedPhysics {
    pub fn new(project: &Project) -> Result<Self, DocumentError> {
        Self::from_validated(project.validated()?)
    }
    pub fn from_validated(
        validated: incant_doc::ValidatedProject<'_>,
    ) -> Result<Self, DocumentError> {
        let project = validated.project();
        let mut scenes = BTreeMap::new();
        for scene in project.scenes.values() {
            let mut entities = BTreeMap::new();
            for entity in scene.entities.values() {
                let Some(collider) = entity.components.get("Collider") else {
                    continue;
                };
                let decode = |key: &str| entity.components.get(key).cloned();
                let transform: Transform =
                    serde_json::from_value(entity.components["Transform"].clone())?;
                let velocity: Option<Velocity> =
                    decode("Velocity").map(serde_json::from_value).transpose()?;
                let angular: Option<AngularVelocity> = decode("AngularVelocity")
                    .map(serde_json::from_value)
                    .transpose()?;
                entities.insert(
                    entity.id.clone(),
                    Spec {
                        body: decode("RigidBody")
                            .map(serde_json::from_value)
                            .transpose()?,
                        collider: serde_json::from_value(collider.clone())?,
                        state: BodyState {
                            translation: transform.translation,
                            rotation: transform.rotation,
                            velocity: velocity.map_or([0.; 3], |v| v.linear),
                            angular_velocity: angular.map_or([0.; 3], |v| v.angular),
                            sleeping: false,
                        },
                    },
                );
            }
            scenes.insert(scene.id.clone(), entities);
        }
        Ok(Self(scenes))
    }
}
struct Entry {
    body: Option<RigidBodyHandle>,
    collider: ColliderHandle,
    config: Spec,
}
#[derive(Default)]
struct SceneWorld {
    world: PhysicsWorld,
    entries: BTreeMap<String, Entry>,
    ids: HashMap<ColliderHandle, String>,
    query_dirty: bool,
}
#[derive(Default)]
pub struct PhysicsRuntime {
    scenes: BTreeMap<String, SceneWorld>,
    overlaps: BTreeSet<(String, String, String)>,
    events: Vec<TriggerEvent>,
    failed: bool,
}
impl PhysicsRuntime {
    /// Unchanged bodies retain their handles, sleep state and solver warm-start
    /// data. Changes to other components never reconstruct the physics world.
    pub fn sync(&mut self, prepared: PreparedPhysics) {
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
            for (id, mut spec) in specs {
                let rebuild = scene.entries.get(&id).is_none_or(|entry| {
                    entry.config.body != spec.body || entry.config.collider != spec.collider
                });
                if rebuild {
                    scene.remove(&id);
                    scene.insert(id, spec);
                    changed = true;
                    continue;
                }
                let entry = scene.entries.get_mut(&id).expect("existing physics entry");
                // Compare the last published values, not normalized/requantized
                // quaternions: the simulation command bus echoes these exactly.
                let previous = &entry.config.state;
                if spec.state.translation != previous.translation
                    || spec.state.rotation != previous.rotation
                {
                    if let Some(body) = entry.body {
                        scene.world.bodies[body].set_position(pose(&spec.state), true);
                    } else {
                        scene.world.colliders[entry.collider].set_position(pose(&spec.state));
                    }
                    changed = true;
                }
                if let Some(body) = entry.body {
                    let body = &mut scene.world.bodies[body];
                    if spec.state.velocity != previous.velocity {
                        body.set_linvel(vector(spec.state.velocity), true);
                    }
                    if spec.state.angular_velocity != previous.angular_velocity {
                        body.set_angvel(vector(spec.state.angular_velocity), true);
                    }
                }
                spec.state.sleeping = entry
                    .body
                    .is_some_and(|body| scene.world.bodies[body].is_sleeping());
                entry.config = spec;
            }
            scene.query_dirty |= changed;
        }
    }
    pub fn step(&mut self, dt: f64) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) {
            return Err(PhysicsError::InvalidStep);
        }
        let mut overlaps = BTreeSet::new();
        for (scene_id, scene) in &mut self.scenes {
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
                    if !state
                        .translation
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                        || !state
                            .velocity
                            .iter()
                            .chain(&state.angular_velocity)
                            .all(|v| v.is_finite() && v.abs() <= 10000.)
                    {
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
        Ok(())
    }
    pub fn state_iter(&self) -> impl Iterator<Item = (&str, &BodyState)> {
        self.scenes.values().flat_map(|scene| {
            scene
                .entries
                .iter()
                .map(|(id, entry)| (id.as_str(), &entry.config.state))
        })
    }
    pub fn states(&self) -> BTreeMap<String, BodyState> {
        self.scenes
            .values()
            .flat_map(|scene| {
                scene
                    .entries
                    .iter()
                    .map(|(id, entry)| (id.clone(), entry.config.state.clone()))
            })
            .collect()
    }
    pub fn events(&self) -> &[TriggerEvent] {
        &self.events
    }
}
impl SceneWorld {
    fn remove(&mut self, id: &str) {
        if let Some(entry) = self.entries.remove(id) {
            self.ids.remove(&entry.collider);
            if let Some(body) = entry.body {
                self.world.remove_body(body);
            } else {
                self.world.remove_collider(entry.collider);
            }
        }
    }
    fn insert(&mut self, id: String, spec: Spec) {
        let shape = match spec.collider.shape {
            ColliderShape::Box {
                half_extents: [x, y, z],
            } => ColliderBuilder::cuboid(x as f32, y as f32, z as f32),
            ColliderShape::Sphere { radius } => ColliderBuilder::ball(radius as f32),
            ColliderShape::Capsule {
                half_height,
                radius,
            } => ColliderBuilder::capsule_y(half_height as f32, radius as f32),
        }
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
