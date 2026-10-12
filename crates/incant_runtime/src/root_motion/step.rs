//! Prepare all root-owned animation and transform candidates before publication.
use super::{PreparedRootMotion, RootMotionState, policy};
use crate::{
    BodyState, CookedSpatial, NativeWorld, RuntimeError, SceneError, StableId,
    animation::Playback,
    world::{Local, Motion},
};
use glam::{DQuat, DVec3};
use incant_anim::RootMotionDelta;
use incant_types::{RootMotionApplication, Transform};

pub(crate) struct PreparedRootStep {
    pub id: StableId,
    pub prepared: PreparedRootMotion,
    pub local: Option<Transform>,
    pub body: Option<BodyState>,
    pub before: Transform,
    pub character: bool,
}
impl NativeWorld {
    pub(crate) fn prepare_root_motion(&mut self, dt: f64, tick: u64) -> Result<(), RuntimeError> {
        self.root_steps.clear();
        for id in &self.root_ids {
            let handle = self.entities[id];
            let node = self
                .world
                .get::<CookedSpatial>(handle)
                .expect("root spatial");
            let definition = node.root_motion.as_ref().expect("root catalog");
            let state = *self
                .world
                .get::<RootMotionState>(handle)
                .expect("root state");
            let playback = self.world.get::<Playback>(handle).expect("root playback");
            let before = &self.world.get::<Local>(handle).expect("root transform").0;
            let binding = self.world.get::<crate::physics::Binding>(handle);
            let velocity = self.world.get::<Motion>(handle).map(|v| &v.0);
            policy::validate(
                definition.application,
                self.parents[id].is_some(),
                before,
                velocity,
                binding.and_then(|b| b.body.as_ref()),
                binding.map(|b| &b.collider),
                self.world
                    .get::<crate::physics_2d::Binding2D>(handle)
                    .is_some(),
            )?;
            if definition.application == RootMotionApplication::Transform {
                let mut parent = self.parents[id];
                while let Some(id) = parent {
                    if !policy::uniform(
                        self.world
                            .get::<Local>(self.entities[&id])
                            .expect("ancestor")
                            .0
                            .scale,
                    ) {
                        return Err(SceneError::Invalid(
                            "root-motion ancestors require positive uniform scale",
                        )
                        .into());
                    }
                    parent = self.parents[&id];
                }
            }
            let mut prepared = PreparedRootMotion::new(definition, playback, state, dt, tick)?;
            let delta = if state.applying {
                prepared.root.requested
            } else {
                RootMotionDelta::IDENTITY
            };
            let (local, body) = match definition.application {
                RootMotionApplication::ExtractOnly => (None, None),
                RootMotionApplication::Transform => {
                    let candidate = compose(before, delta)?;
                    crate::scene::validate_components(*id, &candidate, None)?;
                    prepared.root.applied = delta;
                    accumulate(&mut prepared.root)?;
                    (Some(candidate), None)
                }
                RootMotionApplication::Kinematic3d | RootMotionApplication::Character3d => {
                    if definition.application == RootMotionApplication::Character3d
                        && !policy::yaw_only(delta.rotation)
                    {
                        return Err(SceneError::Invalid(
                            "character root motion cannot apply pitch or roll",
                        )
                        .into());
                    }
                    let candidate = compose(before, delta)?;
                    let rotation = DQuat::from_array(before.rotation).normalize();
                    let mut spin =
                        rotation * DQuat::from_array(delta.rotation).to_scaled_axis() / dt;
                    if delta == RootMotionDelta::IDENTITY {
                        spin = DVec3::ZERO;
                    }
                    let state = BodyState {
                        translation: before.translation,
                        rotation: before.rotation,
                        velocity: std::array::from_fn(|i| {
                            (candidate.translation[i] - before.translation[i]) / dt
                        }),
                        angular_velocity: spin.to_array(),
                        sleeping: false,
                    };
                    crate::physics::validate_motion(
                        *id,
                        false,
                        &candidate,
                        Some(&incant_types::Velocity {
                            linear: state.velocity,
                        }),
                        Some(&incant_types::AngularVelocity {
                            angular: state.angular_velocity,
                        }),
                        binding.and_then(|b| b.body.as_ref()),
                    )?;
                    crate::scene::validate_components(*id, &candidate, None)?;
                    (None, Some(state))
                }
            };
            self.root_steps.push(PreparedRootStep {
                id: *id,
                prepared,
                local,
                body,
                before: before.clone(),
                character: definition.application == RootMotionApplication::Character3d,
            });
        }
        if self.root_steps.iter().any(|row| row.character) {
            let scene = self.scene_id();
            let world = &self.world;
            let entities = &self.entities;
            // One scene-wide query refresh after every delta passed numeric
            // preflight, using current ECS values and retained query geometry.
            self.physics.prepare_character_queries(&scene, |id| {
                let handle = *entities.get(id)?;
                Some(crate::physics::state(
                    &world.get::<Local>(handle)?.0,
                    world.get::<Motion>(handle).map(|v| &v.0),
                    world.get::<crate::physics::Spin>(handle).map(|v| &v.0),
                ))
            })?;
            let result = (|| {
                for row in &mut self.root_steps {
                    if !row.character
                        || !row.prepared.root.applying
                        || row.prepared.root.requested == RootMotionDelta::IDENTITY
                    {
                        continue;
                    }
                    let body = row.body.as_mut().expect("character candidate");
                    let result = self.physics.compute_prepared_character_motion(
                        &incant_physics_runtime::CharacterQuery {
                            scene_id: scene,
                            entity_id: row.id,
                            translation: body.velocity.map(|v| v * dt),
                            options: Default::default(),
                        },
                        dt,
                    )?;
                    body.velocity = result.translation.map(|v| v / dt);
                }
                Ok::<(), RuntimeError>(())
            })();
            self.physics.clear_character_query_frame(&scene);
            result?;
        }
        Ok(())
    }
    /// Startup/structural cold hook; same storage survives all ordinary steps.
    pub(crate) fn prepare_root_query_storage(&mut self) {
        if self.root_ids.iter().any(|id| {
            self.world
                .get::<CookedSpatial>(self.entities[id])
                .and_then(|n| n.root_motion.as_ref())
                .is_some_and(|r| r.application == RootMotionApplication::Character3d)
        }) {
            self.physics
                .enable_character_queries(&self.scene_id())
                .expect("admitted character scene capacity");
        }
    }
    pub(crate) fn finish_root_motion(&mut self) -> Result<(), RuntimeError> {
        // Post-solver checks still precede the first ECS/root publication. The
        // caller already marks the solver-owning world terminal on refusal.
        let scene_id = self.scene_id();
        for row in &mut self.root_steps {
            if row.body.is_some() {
                let result = self.physics.state(&scene_id, &row.id).expect("root body");
                let rotation = DQuat::from_array(row.before.rotation).normalize();
                row.prepared.root.applied = RootMotionDelta {
                    translation: (rotation.conjugate()
                        * (DVec3::from_array(result.translation)
                            - DVec3::from_array(row.before.translation)))
                    .to_array(),
                    rotation: (rotation.conjugate() * DQuat::from_array(result.rotation))
                        .normalize()
                        .to_array(),
                };
                accumulate(&mut row.prepared.root)?;
            }
        }
        Ok(())
    }
    pub(crate) fn publish_root_motion(&mut self) {
        for row in &self.root_steps {
            let mut entity = self.world.entity_mut(self.entities[&row.id]);
            if let Some(local) = &row.local {
                entity.get_mut::<Local>().expect("root transform").0 = local.clone();
            }
            let mut playback = entity.get_mut::<Playback>().expect("root playback");
            playback.state = row.prepared.animation.state;
            playback.graph = row.prepared.animation.graph;
            *entity.get_mut::<RootMotionState>().expect("root state") = row.prepared.root;
        }
    }
}
fn compose(before: &Transform, delta: RootMotionDelta) -> Result<Transform, SceneError> {
    if delta == RootMotionDelta::IDENTITY {
        return Ok(before.clone());
    }
    let rotation = DQuat::from_array(before.rotation).normalize();
    let translation = DVec3::from_array(before.translation)
        + rotation * (DVec3::from_array(delta.translation) * before.scale[0]);
    let rotation = (rotation * DQuat::from_array(delta.rotation)).normalize();
    if !translation.is_finite() || !rotation.is_finite() {
        return Err(SceneError::Invalid("root-motion owner transform overflow"));
    }
    Ok(Transform {
        translation: translation.to_array(),
        rotation: rotation.to_array(),
        scale: before.scale,
    })
}
fn accumulate(state: &mut RootMotionState) -> Result<(), SceneError> {
    if state.applied != RootMotionDelta::IDENTITY {
        state.accumulated_applied = state
            .accumulated_applied
            .compose(state.applied)
            .map_err(|_| SceneError::Invalid("root-motion accumulated application overflow"))?;
    }
    Ok(())
}
