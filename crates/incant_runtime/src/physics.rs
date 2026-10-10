//! Typed physics ownership. Configuration is prepared only at load/structural
//! boundaries; ticks visit committed numeric components and solver outputs.
use crate::scene::validate_components;
use crate::world::{Local, Motion};
use crate::{BodyState, CookedEntity, NativeWorld, SceneError, StableId, TriggerEvent};
use bevy_ecs::prelude::*;
use incant_physics_runtime::{PhysicsSpec, PreparedBody};
use incant_types::{AngularVelocity, Collider, RigidBody, Transform, Velocity, physics_validation};

#[derive(Debug, Clone, PartialEq)]
pub struct CookedPhysics {
    pub entity: StableId,
    pub body: Option<RigidBody>,
    pub collider: Collider,
    pub angular_velocity: Option<AngularVelocity>,
}

#[derive(Component, Clone)]
pub(crate) struct Binding {
    pub body: Option<RigidBody>,
    pub collider: Collider,
}
#[derive(Component)]
pub(crate) struct Spin(pub AngularVelocity);

impl CookedPhysics {
    pub(crate) fn validate(&self, entity: &CookedEntity) -> Result<(), SceneError> {
        let reject = |_| SceneError::Component(entity.id);
        validate_components(entity.id, &entity.transform, entity.velocity.as_ref())?;
        if let Some(body) = &self.body {
            physics_validation::rigid_body(body).map_err(reject)?;
        }
        physics_validation::collider(&self.collider).map_err(reject)?;
        validate_motion(
            entity.id,
            entity.parent.is_some(),
            &entity.transform,
            entity.velocity.as_ref(),
            self.angular_velocity.as_ref(),
            self.body.as_ref(),
        )
    }

    pub(crate) fn prepare(&self, entity: &CookedEntity) -> PreparedBody {
        PreparedBody::new(PhysicsSpec {
            body: self.body.clone(),
            collider: self.collider.clone(),
            state: state(
                &entity.transform,
                entity.velocity.as_ref(),
                self.angular_velocity.as_ref(),
            ),
        })
        .expect("validated declaration")
    }
}

pub(crate) fn validate_motion(
    id: StableId,
    parent: bool,
    transform: &Transform,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity>,
    body: Option<&RigidBody>,
) -> Result<(), SceneError> {
    physics_validation::motion(parent, transform, body, velocity, spin)
        .map_err(|_| SceneError::Component(id))
}

pub(crate) fn state(
    transform: &Transform,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity>,
) -> BodyState {
    BodyState {
        translation: transform.translation,
        rotation: transform.rotation,
        velocity: velocity.map_or([0.; 3], |v| v.linear),
        angular_velocity: spin.map_or([0.; 3], |v| v.angular),
        sleeping: false,
    }
}

impl NativeWorld {
    /// Explicit configuration inspection; cloning compound data is confined to
    /// this caller-requested operation, never the tick path.
    pub fn physics_configuration(&self, id: StableId) -> Option<CookedPhysics> {
        if self.physics_failed {
            return None;
        }
        let entity = self.world.entity(*self.entities.get(&id)?);
        let binding = entity.get::<Binding>()?;
        Some(CookedPhysics {
            entity: id,
            body: binding.body.clone(),
            collider: binding.collider.clone(),
            angular_velocity: entity.get::<Spin>().map(|s| s.0.clone()),
        })
    }

    /// Current typed ECS pose/motion plus committed solver sleep state. A changed
    /// pending numeric input is awake until the next successful solver step.
    /// No solver handle, pointer or project representation is exposed.
    pub fn physics_state(&self, id: StableId) -> Option<BodyState> {
        if self.physics_failed {
            return None;
        }
        let entity = self.world.entity(*self.entities.get(&id)?);
        entity.get::<Binding>()?;
        let mut result = state(
            &entity.get::<Local>()?.0,
            entity.get::<Motion>().map(|m| &m.0),
            entity.get::<Spin>().map(|s| &s.0),
        );
        if let Some(previous) = self.physics.state(&self.scene_id(), &id) {
            result.sleeping = previous.sleeping;
            if &result != previous {
                result.sleeping = false;
            }
        }
        Some(result)
    }

    /// Stable-ID ordered sensor transitions from the last completed tick.
    /// At most MAX_TRIGGER_EVENTS records; overflow poisons the physics world.
    pub fn physics_events(&self) -> &[TriggerEvent<StableId>] {
        if self.physics_failed {
            &[]
        } else {
            self.physics.events()
        }
    }

    pub(crate) fn install_physics(&mut self, declaration: CookedPhysics) {
        self.physics_used = true;
        let handle = self.entities[&declaration.entity];
        let mut target = self.world.entity_mut(handle);
        if declaration.body.is_some() {
            if !target.contains::<Motion>() {
                target.insert(Motion(Velocity { linear: [0.; 3] }));
            }
            target.insert(Spin(
                declaration
                    .angular_velocity
                    .unwrap_or(AngularVelocity { angular: [0.; 3] }),
            ));
        } else {
            target.remove::<Spin>();
        }
        target.insert(Binding {
            body: declaration.body,
            collider: declaration.collider,
        });
    }
}
