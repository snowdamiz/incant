//! Bounded, document-free planar physics. Each owner has one scene solver;
//! stable IDs cross the boundary, never Rapier handles. Native deterministic
//! builds use parallel Rapier; WebAssembly remains serial.
// A solver variant is a build choice, never a runtime toggle. Rapier itself also
// rejects this combination; keep the local diagnostic at our public boundary.
#[cfg(all(feature = "deterministic", feature = "fast-simd8"))]
compile_error!("deterministic and fast-simd8 physics variants are mutually exclusive");

/// Build identity; Cargo feature admission is also checked by the exporter.
pub const DETERMINISTIC: bool = cfg!(feature = "deterministic");
pub const SIMD_LANES: u8 = if cfg!(feature = "fast-simd8") { 8 } else { 4 };

mod config;
mod query;
pub use config::{PreparedBody, validate_collider, validate_state};
use incant_types::{BodyMotion, StableId};
pub use query::{RayHit, RayQuery};
use rapier2d::prelude::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

pub const MAX_BODIES: usize = 65_536;
pub const MAX_SHAPES: usize = 262_144;
pub const MAX_TRIGGER_PAIRS: usize = 65_536;
pub const MAX_TRIGGER_EVENTS: usize = MAX_TRIGGER_PAIRS * 2;

#[derive(Debug, Error, PartialEq)]
pub enum PhysicsError {
    #[error("2D step must be finite and between 1/240 and 1 seconds")]
    InvalidStep,
    #[error("2D solver failed; discard this session")]
    Failed,
    #[error("2D collection limit exceeded or duplicate structural ID")]
    Limit,
    #[error("invalid typed 2D component: {0}")]
    Component(String),
    #[error("invalid 2D query or unknown body")]
    Query,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BodyState {
    pub translation: [f64; 2],
    /// Counter-clockwise radians. Solver output is normalized to [-pi, pi].
    pub rotation: f64,
    pub velocity: [f64; 2],
    pub angular_velocity: f64,
    /// Observation only; inputs cannot directly force the sleep state.
    pub sleeping: bool,
}
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[schemars(rename = "TriggerEvent2D")]
pub struct TriggerEvent<I = StableId> {
    pub first: I,
    pub second: I,
    pub entered: bool,
}
struct Entry {
    body: Option<RigidBodyHandle>,
    collider: ColliderHandle,
    config: PreparedBody,
}
#[derive(Default)]
pub struct PhysicsWorld2D {
    world: PhysicsWorld,
    entries: BTreeMap<StableId, Entry>,
    ids: HashMap<ColliderHandle, StableId>,
    overlaps: BTreeSet<(StableId, StableId)>,
    events: Vec<TriggerEvent>,
    query_dirty: bool,
    failed: bool,
}
impl PhysicsWorld2D {
    /// Check the complete final collection before changing any live solver.
    /// Prepared declarations have already passed geometry/motion validation.
    pub fn validate_changes(
        &self,
        changes: &[(StableId, Option<PreparedBody>)],
    ) -> Result<(), PhysicsError> {
        self.check_live()?;
        if changes.len() > MAX_BODIES {
            return Err(PhysicsError::Limit);
        }
        let mut ids = BTreeSet::new();
        let mut bodies = self.entries.len();
        let mut shapes: usize = self.entries.values().map(|e| e.config.shapes()).sum();
        for (id, config) in changes {
            if !ids.insert(*id) {
                return Err(PhysicsError::Limit);
            }
            if let Some(old) = self.entries.get(id) {
                bodies -= 1;
                shapes -= old.config.shapes();
            }
            if let Some(config) = config {
                bodies += 1;
                shapes += config.shapes();
            }
        }
        if bodies > MAX_BODIES || shapes > MAX_SHAPES {
            return Err(PhysicsError::Limit);
        }
        Ok(())
    }

    /// Atomic preflight, then infallible publication. Unchanged declarations
    /// preserve handles/contacts/sleep. No whole-world copy or authoring path.
    pub fn apply(
        &mut self,
        changes: Vec<(StableId, Option<PreparedBody>)>,
    ) -> Result<(), PhysicsError> {
        self.validate_changes(&changes)?;
        for (id, config) in changes {
            if let Some(config) = config {
                if self.entries.get(&id).is_some_and(|old| {
                    old.config.body == config.body
                        && old.config.collider == config.collider
                        && old.config.geometry == config.geometry
                }) {
                    self.update_motion(id, &config.state);
                } else {
                    self.remove(id);
                    self.insert(id, config);
                }
            } else {
                self.remove(id);
            }
        }
        Ok(())
    }

    /// Validate the entire borrowed batch before any change. A slice keeps the
    /// second pass identical, including for callers with stateful iterators.
    /// Native ECS owners can preflight their complete input and then lend stack
    /// single-entry batches; no per-tick heap state vector is required.
    pub fn sync_motion(&mut self, motions: &[(StableId, BodyState)]) -> Result<(), PhysicsError> {
        self.check_live()?;
        if motions.len() > MAX_BODIES {
            return Err(PhysicsError::Limit);
        }
        for (id, state) in motions {
            let entry = self.entries.get(id).ok_or(PhysicsError::Query)?;
            validate_state(entry.config.body.as_ref(), state)?;
        }
        for (id, state) in motions {
            self.update_motion(*id, state);
        }
        Ok(())
    }

    pub fn state(&self, id: StableId) -> Option<&BodyState> {
        if self.failed {
            return None;
        }
        self.entries.get(&id).map(|e| &e.config.state)
    }
    pub fn states(&self) -> impl Iterator<Item = (StableId, &BodyState)> {
        self.entries
            .iter()
            .filter(|_| !self.failed)
            .map(|(id, e)| (*id, &e.config.state))
    }
    pub fn events(&self) -> &[TriggerEvent] {
        if self.failed { &[] } else { &self.events }
    }

    /// Invalid dt is retryable. Any failure after solver advancement is terminal;
    /// outputs are hidden and future reads/mutations refuse this solver.
    pub fn step(&mut self, dt: f64) -> Result<(), PhysicsError> {
        self.check_live()?;
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) {
            return Err(PhysicsError::InvalidStep);
        }
        self.world.integration_parameters.dt = dt as f32;
        // A caught native unwind cannot resume a partially advanced solver.
        self.failed = true;
        self.world.step();
        self.query_dirty = false;
        if !self.world.quarantine().bodies().is_empty()
            || !self.world.quarantine().colliders().is_empty()
        {
            self.failed = true;
            return Err(PhysicsError::Failed);
        }
        for entry in self.entries.values_mut() {
            if let Some(body) = entry.body {
                let body = &self.world.bodies[body];
                let state = BodyState {
                    translation: body.translation().to_array().map(f64::from),
                    rotation: f64::from(body.rotation().angle()),
                    velocity: body.linvel().to_array().map(f64::from),
                    angular_velocity: f64::from(body.angvel()),
                    sleeping: body.is_sleeping(),
                };
                if validate_state(entry.config.body.as_ref(), &state).is_err() {
                    self.failed = true;
                    return Err(PhysicsError::Failed);
                }
                entry.config.state = state;
            }
        }
        let mut overlaps = BTreeSet::new();
        for (a, _, b, _, intersecting) in self.world.intersection_pairs() {
            if !intersecting {
                continue;
            }
            let (a, b) = (self.ids[&a], self.ids[&b]);
            if a == b {
                continue;
            }
            overlaps.insert(if a < b { (a, b) } else { (b, a) });
            if overlaps.len() > MAX_TRIGGER_PAIRS {
                self.failed = true;
                return Err(PhysicsError::Failed);
            }
        }
        self.events.clear();
        self.events
            .extend(
                overlaps
                    .difference(&self.overlaps)
                    .map(|&(first, second)| TriggerEvent {
                        first,
                        second,
                        entered: true,
                    }),
            );
        self.events
            .extend(
                self.overlaps
                    .difference(&overlaps)
                    .map(|&(first, second)| TriggerEvent {
                        first,
                        second,
                        entered: false,
                    }),
            );
        self.events.sort_unstable();
        self.overlaps = overlaps;
        self.failed = false;
        Ok(())
    }
    fn check_live(&self) -> Result<(), PhysicsError> {
        if self.failed {
            Err(PhysicsError::Failed)
        } else {
            Ok(())
        }
    }
    fn update_motion(&mut self, id: StableId, state: &BodyState) {
        let entry = self.entries.get_mut(&id).expect("prevalidated body");
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
                body.set_angvel(state.angular_velocity as f32, true);
            }
        }
        entry.config.state = state.clone();
        entry.config.state.sleeping = entry
            .body
            .is_some_and(|body| self.world.bodies[body].is_sleeping());
    }
    fn remove(&mut self, id: StableId) {
        if let Some(entry) = self.entries.remove(&id) {
            self.ids.remove(&entry.collider);
            if let Some(body) = entry.body {
                self.world.remove_body(body);
            } else {
                self.world.remove_collider(entry.collider);
            }
            self.query_dirty = true;
        }
    }
    fn insert(&mut self, id: StableId, mut config: PreparedBody) {
        let shape = ColliderBuilder::new(config.shape())
            .density(config.collider.density as f32)
            .friction(config.collider.friction as f32)
            .restitution(config.collider.restitution as f32)
            .sensor(config.collider.sensor)
            .collision_groups(groups(config.collider.memberships, config.collider.filter))
            .active_collision_types(ActiveCollisionTypes::all());
        let (body, collider) = if let Some(props) = &config.body {
            let body = match props.motion {
                BodyMotion::Fixed => RigidBodyBuilder::fixed(),
                BodyMotion::Dynamic => RigidBodyBuilder::dynamic(),
                BodyMotion::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
            }
            .pose(pose(&config.state))
            .linvel(vector(config.state.velocity))
            .angvel(config.state.angular_velocity as f32)
            .gravity_scale(props.gravity_scale as f32)
            .linear_damping(props.linear_damping as f32)
            .angular_damping(props.angular_damping as f32)
            .can_sleep(props.can_sleep)
            .ccd_enabled(props.ccd);
            let (b, c) = self.world.insert(body, shape);
            (Some(b), c)
        } else {
            (
                None,
                self.world
                    .insert_collider(shape.position(pose(&config.state)), None),
            )
        };
        config.state.sleeping = body.is_some_and(|body| self.world.bodies[body].is_sleeping());
        self.ids.insert(collider, id);
        self.entries.insert(
            id,
            Entry {
                body,
                collider,
                config,
            },
        );
        self.query_dirty = true;
    }
}
fn vector(value: [f64; 2]) -> Vector {
    Vector::from_array(value.map(|v| v as f32))
}
fn pose(state: &BodyState) -> Pose {
    Pose::from_parts(
        vector(state.translation),
        Rotation::from_angle(state.rotation as f32),
    )
}
fn groups(memberships: u32, filter: u32) -> InteractionGroups {
    InteractionGroups::new(
        Group::from_bits_retain(memberships),
        Group::from_bits_retain(filter),
        InteractionTestMode::And,
    )
}
