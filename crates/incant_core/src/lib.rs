//! Bevy ECS projection and fixed-step simulation. The editor document is immutable
//! during play; stopping discards the runtime projection, preserving authored state.
use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use incant_doc::{Project, Transform as DocTransform, Velocity as DocVelocity};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Component, Clone)]
pub struct StableId(pub String);
#[derive(Component, Clone)]
pub struct SceneId(pub String);
#[derive(Component, Clone)]
pub struct Position(pub [f64; 3]);
#[derive(Component, Clone)]
pub struct LinearVelocity(pub [f64; 3]);
#[derive(Resource)]
struct FixedStep(f64);
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEntity {
    pub id: String,
    pub scene_id: String,
    pub translation: [f64; 3],
    pub velocity: [f64; 3],
}
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSnapshot {
    pub tick: u64,
    pub elapsed_seconds: f64,
    pub entities: BTreeMap<String, RuntimeEntity>,
}

pub struct Engine {
    app: App,
    tick: u64,
    dt: f64,
}
impl Engine {
    pub fn new(project: &Project) -> Result<Self, incant_doc::DocumentError> {
        project.validate()?;
        let dt = 1. / f64::from(project.settings.tick_rate);
        let mut app = App::new();
        app.insert_resource(FixedStep(dt));
        app.add_systems(Update, integrate);
        for scene in project.scenes.values() {
            for entity in scene.entities.values() {
                let position = entity
                    .components
                    .get("Transform")
                    .map(|v| serde_json::from_value::<DocTransform>(v.clone()))
                    .transpose()?
                    .unwrap_or_default();
                let velocity = entity
                    .components
                    .get("Velocity")
                    .map(|v| serde_json::from_value::<DocVelocity>(v.clone()))
                    .transpose()?
                    .map_or([0.; 3], |v| v.linear);
                app.world_mut().spawn((
                    StableId(entity.id.clone()),
                    SceneId(scene.id.clone()),
                    Position(position.translation),
                    LinearVelocity(velocity),
                ));
            }
        }
        Ok(Self { app, tick: 0, dt })
    }
    pub fn step(&mut self) {
        self.app.update();
        self.tick += 1;
    }
    pub fn run_ticks(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }
    pub fn snapshot(&mut self) -> RuntimeSnapshot {
        let mut query = self
            .app
            .world_mut()
            .query::<(&StableId, &SceneId, &Position, &LinearVelocity)>();
        let entities = query
            .iter(self.app.world())
            .map(|(id, scene, position, velocity)| {
                (
                    id.0.clone(),
                    RuntimeEntity {
                        id: id.0.clone(),
                        scene_id: scene.0.clone(),
                        translation: position.0,
                        velocity: velocity.0,
                    },
                )
            })
            .collect();
        RuntimeSnapshot {
            tick: self.tick,
            elapsed_seconds: self.tick as f64 * self.dt,
            entities,
        }
    }
    /// Apply script-generated authored state through the command bus, then sync the
    /// ECS projection. No scripts receive World or unvalidated component access.
    pub fn sync(&mut self, project: &Project) -> Result<(), incant_doc::DocumentError> {
        let tick = self.tick;
        let mut next = Self::new(project)?;
        next.tick = tick;
        *self = next;
        Ok(())
    }
}
fn integrate(step: Res<FixedStep>, mut query: Query<(&mut Position, &LinearVelocity)>) {
    for (mut position, velocity) in &mut query {
        for axis in 0..3 {
            position.0[axis] += velocity.0[axis] * step.0;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_doc::{Entity as DocEntity, Scene};
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
        a.run_ticks(120);
        b.run_ticks(120);
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
