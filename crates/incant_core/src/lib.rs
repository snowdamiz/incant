//! Bevy ECS projection and fixed-step simulation. The editor document is immutable
//! during play; stopping discards the runtime projection, preserving authored state.
mod scene;
use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use incant_doc::{MeshRenderer, Project};
pub use incant_physics::{PhysicsError, RayHit, RayQuery, TriggerEvent};
use incant_physics::{PhysicsRuntime, PreparedPhysics};
use scene::{LocalFrame, ParentId, SceneOrder, WorldFrame, prepare, propagate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Component, Clone)]
pub struct StableId(pub String);
#[derive(Component, Clone)]
pub struct SceneId(pub String);
#[derive(Component, Clone)]
pub struct Position(pub [f64; 3]);
#[derive(Component, Clone)]
pub struct LinearVelocity(pub [f64; 3]);
#[derive(Component)]
struct PhysicsBody;
#[derive(Component, Clone)]
pub struct AngularVelocity(pub [f64; 3]);
#[derive(Component, Clone)]
pub struct MeshBinding(pub MeshRenderer);
#[derive(Resource)]
struct FixedStep(f64);
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEntity {
    pub id: String,
    pub scene_id: String,
    pub translation: [f64; 3],
    pub velocity: [f64; 3],
    pub angular_velocity: [f64; 3],
    pub parent: Option<String>,
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
    /// Column-major local-to-world matrix, including every ancestor. Translation
    /// and velocity above remain parent-local, preserving the script contract.
    pub world_transform: [[f64; 4]; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh: Option<MeshRenderer>,
}
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSnapshot {
    pub tick: u64,
    pub elapsed_seconds: f64,
    pub entities: BTreeMap<String, RuntimeEntity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trigger_events: Vec<TriggerEvent>,
}

pub struct Engine {
    app: App,
    physics: Arc<Mutex<PhysicsRuntime>>,
    entities: BTreeMap<String, Entity>,
    tick: u64,
    dt: f64,
    elapsed_seconds: f64,
}
impl Engine {
    pub fn new(project: &Project) -> Result<Self, incant_doc::DocumentError> {
        let mut app = App::new();
        app.add_systems(Update, (integrate, propagate).chain());
        let mut engine = Self {
            app,
            physics: Arc::new(Mutex::new(PhysicsRuntime::default())),
            entities: BTreeMap::new(),
            tick: 0,
            dt: 0.,
            elapsed_seconds: 0.,
        };
        engine.sync(project)?;
        Ok(engine)
    }
    pub fn step(&mut self) -> Result<(), PhysicsError> {
        let mut physics = self.physics.lock().unwrap_or_else(|e| e.into_inner());
        physics.step(self.dt)?;
        for (id, state) in physics.states() {
            let entity = self.entities[&id];
            let mut target = self.app.world_mut().entity_mut(entity);
            target.get_mut::<Position>().expect("physics position").0 = state.translation;
            target
                .get_mut::<LocalFrame>()
                .expect("physics frame")
                .rotation = state.rotation;
            target
                .get_mut::<LinearVelocity>()
                .expect("physics velocity")
                .0 = state.velocity;
            target
                .get_mut::<AngularVelocity>()
                .expect("physics angular velocity")
                .0 = state.angular_velocity;
        }
        drop(physics);
        self.app.update();
        self.tick += 1;
        self.elapsed_seconds += self.dt;
        Ok(())
    }
    pub fn run_ticks(&mut self, ticks: u64) -> Result<(), PhysicsError> {
        for _ in 0..ticks {
            self.step()?;
        }
        Ok(())
    }
    /// Read-only query capability. No solver handles or mutation methods escape.
    pub fn raycaster(
        &self,
    ) -> impl Fn(RayQuery) -> Result<Option<RayHit>, PhysicsError> + Send + Sync + 'static {
        let physics = self.physics.clone();
        move |query| {
            physics
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .raycast(&query)
        }
    }
    pub fn snapshot(&mut self) -> RuntimeSnapshot {
        let mut query = self.app.world_mut().query::<(
            &StableId,
            &SceneId,
            &Position,
            &LinearVelocity,
            &AngularVelocity,
            &ParentId,
            &LocalFrame,
            &WorldFrame,
            Option<&MeshBinding>,
        )>();
        let entities = query
            .iter(self.app.world())
            .map(
                |(id, scene, position, velocity, angular_velocity, parent, local, world, mesh)| {
                    (
                        id.0.clone(),
                        RuntimeEntity {
                            id: id.0.clone(),
                            scene_id: scene.0.clone(),
                            translation: position.0,
                            velocity: velocity.0,
                            angular_velocity: angular_velocity.0,
                            parent: parent.0.clone(),
                            rotation: local.rotation,
                            scale: local.scale,
                            world_transform: world.0,
                            mesh: mesh.map(|binding| binding.0.clone()),
                        },
                    )
                },
            )
            .collect();
        RuntimeSnapshot {
            tick: self.tick,
            elapsed_seconds: self.elapsed_seconds,
            entities,
            trigger_events: self
                .physics
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .events()
                .to_vec(),
        }
    }
    /// Apply script-generated authored state through the command bus, then sync the
    /// ECS projection. Validation/decoding finish before any live entity changes.
    /// Existing Bevy entities and schedules are reused; no scripts receive World.
    pub fn sync(&mut self, project: &Project) -> Result<(), incant_doc::DocumentError> {
        let staged = prepare(project)?;
        let prepared_physics = PreparedPhysics::new(project)?;
        let mut physics = self.physics.lock().unwrap_or_else(|e| e.into_inner());
        physics.sync(prepared_physics);
        let physics_states = physics.states();
        drop(physics);
        let retained: BTreeSet<_> = staged.iter().map(|entity| entity.id.as_str()).collect();
        let world = self.app.world_mut();
        self.entities.retain(|id, entity| {
            if retained.contains(id.as_str()) {
                true
            } else {
                world.despawn(*entity);
                false
            }
        });
        let mut order = SceneOrder::default();
        for entity in staged {
            let id = *self
                .entities
                .entry(entity.id.clone())
                .or_insert_with(|| world.spawn_empty().id());
            let mut target = world.entity_mut(id);
            target.insert((
                StableId(entity.id.clone()),
                SceneId(entity.scene),
                Position(entity.local.translation),
                LinearVelocity(entity.velocity),
                AngularVelocity(
                    physics_states
                        .get(&entity.id)
                        .map_or([0.; 3], |state| state.angular_velocity),
                ),
                ParentId(entity.parent_id),
                LocalFrame {
                    rotation: entity.local.rotation,
                    scale: entity.local.scale,
                },
                WorldFrame(entity.world.to_cols_array_2d()),
            ));
            if physics_states.contains_key(&entity.id) {
                target.insert(PhysicsBody);
            } else {
                target.remove::<PhysicsBody>();
            }
            if let Some(mesh) = entity.mesh {
                target.insert(MeshBinding(mesh));
            } else {
                target.remove::<MeshBinding>();
            }
            order.entities.push((id, entity.parent_index));
            order.worlds.push(entity.world);
        }
        self.dt = 1. / f64::from(project.settings.tick_rate);
        world.insert_resource(FixedStep(self.dt));
        world.insert_resource(order);
        Ok(())
    }
}
fn integrate(
    step: Res<FixedStep>,
    mut query: Query<(&mut Position, &LinearVelocity), Without<PhysicsBody>>,
) {
    for (mut position, velocity) in &mut query {
        for axis in 0..3 {
            position.0[axis] += velocity.0[axis] * step.0;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_doc::{Entity as DocEntity, Scene, Transform as DocTransform};
    use serde_json::json;
    #[test]
    fn fixed_step_is_reproducible_and_does_not_edit_project() {
        let mut project = Project::empty("sample");
        let mut scene = Scene::new("world");
        let mut entity = DocEntity::new("player");
        let id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(DocTransform::default()));
        entity
            .components
            .insert("Velocity".into(), json!({"linear":[3.,0.,0.]}));
        scene.entities.insert(id.clone(), entity);
        project.scenes.insert(scene.id.clone(), scene);
        let before = project.clone();
        let mut a = Engine::new(&project).unwrap();
        let mut b = Engine::new(&project).unwrap();
        a.run_ticks(120).unwrap();
        b.run_ticks(120).unwrap();
        let sa = a.snapshot();
        let sb = b.snapshot();
        assert_eq!(
            serde_json::to_value(&sa).unwrap(),
            serde_json::to_value(&sb).unwrap()
        );
        assert!((sa.entities[&id].translation[0] - 6.).abs() < 1e-9);
        assert_eq!(project, before);
    }
}
