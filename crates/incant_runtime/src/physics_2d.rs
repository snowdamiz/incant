//! Retained native planar bindings. Geometry is prepared only at structural
//! boundaries. Ticks only visit typed ECS motion and solver output.
use crate::world::{Identity, Local, Motion};
use crate::{CookedEntity, NativeWorld, RuntimeError, SceneError, StableId};
use bevy_ecs::prelude::*;
use incant_physics_2d_runtime::{BodyState, PreparedBody};
use incant_types::{
    AngularVelocity2D, Collider2D, RigidBody, Transform, Velocity, physics_2d_validation,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CookedPhysics2D {
    pub entity: StableId,
    pub body: Option<RigidBody>,
    pub collider: Collider2D,
    pub angular_velocity: Option<AngularVelocity2D>,
}
#[derive(Component)]
pub(crate) struct Binding2D {
    pub body: Option<RigidBody>,
    /// None is an automatically generated solid-tilemap binding.
    pub collider: Option<Collider2D>,
    // Exact last published quaternion avoids angle round-trip waking sleepers.
    pub rotation: [f64; 4],
}
#[derive(Component)]
pub(crate) struct Spin2D(pub AngularVelocity2D);

impl CookedPhysics2D {
    pub(crate) fn validate(&self, entity: &CookedEntity) -> Result<(), SceneError> {
        let reject = |_| SceneError::Component(entity.id);
        crate::scene::validate_components(entity.id, &entity.transform, entity.velocity.as_ref())?;
        if let Some(body) = &self.body {
            incant_types::physics_validation::rigid_body(body).map_err(reject)?;
        }
        physics_2d_validation::collider(&self.collider).map_err(reject)?;
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
        PreparedBody::new(
            self.body.clone(),
            self.collider.clone(),
            state(
                &entity.transform,
                entity.velocity.as_ref(),
                self.angular_velocity.as_ref(),
                None,
            ),
        )
        .expect("validated 2D declaration")
    }
}

pub(crate) fn validate_motion(
    id: StableId,
    parent: bool,
    transform: &Transform,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity2D>,
    body: Option<&RigidBody>,
) -> Result<(), SceneError> {
    physics_2d_validation::motion(parent, transform, body, velocity, spin)
        .map_err(|_| SceneError::Component(id))
}
pub(crate) fn rotation(angle: f64) -> [f64; 4] {
    let (sin, cos) = (libm::sin(angle / 2.), libm::cos(angle / 2.));
    [0., 0., sin, cos]
}
pub(crate) fn state(
    t: &Transform,
    velocity: Option<&Velocity>,
    spin: Option<&AngularVelocity2D>,
    previous: Option<(&Binding2D, &BodyState)>,
) -> BodyState {
    BodyState {
        translation: [t.translation[0], t.translation[1]],
        rotation: previous
            .filter(|(b, _)| b.rotation == t.rotation)
            .map_or_else(
                || 2. * libm::atan2(t.rotation[2], t.rotation[3]),
                |(_, s)| s.rotation,
            ),
        velocity: velocity.map_or([0.; 2], |v| [v.linear[0], v.linear[1]]),
        angular_velocity: spin.map_or(0., |v| v.angular),
        sleeping: false,
    }
}

impl NativeWorld {
    pub fn physics_2d_configuration(&self, id: StableId) -> Option<CookedPhysics2D> {
        if self.physics_failed {
            return None;
        }
        let entity = self.world.entity(*self.entities.get(&id)?);
        let binding = entity.get::<Binding2D>()?;
        Some(CookedPhysics2D {
            entity: id,
            body: binding.body.clone(),
            collider: binding.collider.clone()?,
            angular_velocity: entity.get::<Spin2D>().map(|s| s.0.clone()),
        })
    }
    /// Current native numeric values; unchanged solver echoes retain sleep.
    /// Solid tilemaps are included, with fixed zero motion and retained IDs.
    pub fn physics_2d_state(&self, id: StableId) -> Option<BodyState> {
        if self.physics_failed {
            return None;
        }
        let entity = self.world.entity(*self.entities.get(&id)?);
        let binding = entity.get::<Binding2D>()?;
        let previous = self.physics_2d.state(id)?;
        let mut value = state(
            &entity.get::<Local>()?.0,
            entity.get::<Motion>().map(|m| &m.0),
            entity.get::<Spin2D>().map(|s| &s.0),
            Some((binding, previous)),
        );
        value.sleeping = previous.sleeping;
        if &value != previous {
            value.sleeping = false;
        }
        Some(value)
    }
    pub fn physics_2d_events(&self) -> &[crate::TriggerEvent2D] {
        if self.physics_failed {
            &[]
        } else {
            self.physics_2d.events()
        }
    }
    /// Flush validated numeric edits without ticking; all inputs are checked
    /// before the first solver write. No project or query-only collision step.
    pub fn raycast_2d(
        &mut self,
        query: &crate::RayQuery2D,
    ) -> Result<Option<crate::RayHit2D>, RuntimeError> {
        self.ensure_usable()?;
        query.validate()?;
        self.prepare_globals(None)?;
        self.sync_physics_2d()?;
        Ok(self.physics_2d.raycast(query)?)
    }
    pub(crate) fn sync_physics_2d(&mut self) -> Result<(), RuntimeError> {
        let mut query = self.world.query::<(
            &Identity,
            &Local,
            Option<&Motion>,
            Option<&Spin2D>,
            &Binding2D,
        )>();
        for (id, local, velocity, spin, binding) in query.iter(&self.world) {
            let previous = self.physics_2d.state(id.0).expect("2D binding");
            let state = state(
                &local.0,
                velocity.map(|v| &v.0),
                spin.map(|s| &s.0),
                Some((binding, previous)),
            );
            self.physics_2d.sync_motion(&[(id.0, state)])?;
        }
        Ok(())
    }
    pub(crate) fn install_physics_2d(
        &mut self,
        id: StableId,
        declaration: Option<CookedPhysics2D>,
        tilemap: bool,
    ) {
        let mut target = self.world.entity_mut(self.entities[&id]);
        let rotation = target.get::<Local>().unwrap().0.rotation;
        if let Some(declaration) = declaration {
            self.physics_2d_used = true;
            if declaration.body.is_some() {
                if !target.contains::<Motion>() {
                    target.insert(Motion(Velocity { linear: [0.; 3] }));
                }
                target.insert(Spin2D(
                    declaration
                        .angular_velocity
                        .unwrap_or(AngularVelocity2D { angular: 0. }),
                ));
            } else {
                target.remove::<Spin2D>();
            }
            target.insert(Binding2D {
                body: declaration.body,
                collider: Some(declaration.collider),
                rotation,
            });
        } else if tilemap {
            self.physics_2d_used = true;
            target.remove::<Spin2D>();
            target.insert(Binding2D {
                body: None,
                collider: None,
                rotation,
            });
        } else {
            target.remove::<(Binding2D, Spin2D)>();
        }
    }
}
