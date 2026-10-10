use super::{ScriptError, ScriptHost};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Origin;
use std::sync::Arc;

/// A disposable play session. ECS query results enter the same validated runtime
/// command bus before scripts read them; the author's document never changes.
pub struct PlaySession {
    pub host: ScriptHost,
    pub(super) bus: CommandBus,
    engine: incant_core::Engine,
    pub(super) dt: f64,
    pub(super) authored_sha256: String,
    pub(super) manifest_sha256: String,
    pub(super) ticks: u64,
    pub(super) elapsed_seconds: f64,
    pub(super) failed: bool,
}
impl PlaySession {
    pub fn new(project: &incant_doc::Project, compiled_source: &str) -> Result<Self, ScriptError> {
        let engine = incant_core::Engine::new(project)?;
        let mut host = ScriptHost::new(compiled_source)?;
        host.raycaster = Some(Arc::new(engine.raycaster()));
        host.character_mover = Some(Arc::new(engine.character_mover()));
        host.install_queries()?;
        Ok(Self {
            host,
            authored_sha256: super::saves::hash(project.canonical_text()?.as_bytes()),
            manifest_sha256: super::saves::manifest(project)?,
            ticks: 0,
            elapsed_seconds: 0.,
            failed: false,
            bus: CommandBus::simulation(project.clone())?,
            engine,
            dt: 1. / f64::from(project.settings.tick_rate),
        })
    }
    pub fn project(&self) -> &incant_doc::Project {
        self.bus.project()
    }
    pub fn snapshot(&mut self) -> incant_core::RuntimeSnapshot {
        let mut snapshot = self.engine.snapshot();
        snapshot.tick = self.ticks;
        snapshot.elapsed_seconds = self.elapsed_seconds;
        snapshot
    }
    pub fn tick(&mut self) -> Result<usize, ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        if self.ticks >= super::saves::MAX_SAVE_TICK {
            self.failed = true;
            return Err(ScriptError::InputLimit);
        }
        match self.tick_inner() {
            Ok(count) => {
                self.ticks += 1;
                self.elapsed_seconds += self.dt;
                Ok(count)
            }
            Err(error) => {
                // Physics may already have stepped before a script failure. Do
                // not label that partial tick as resumable committed game state.
                self.failed = true;
                Err(error)
            }
        }
    }
    fn tick_inner(&mut self) -> Result<usize, ScriptError> {
        self.engine.step()?;
        let mut commands = Vec::new();
        let snapshot = self.engine.snapshot();
        for runtime in snapshot.entities.values() {
            let entity = &self.bus.project().scenes[&runtime.scene_id].entities[&runtime.id];
            if let Some(value) = entity.components.get("Transform") {
                let mut transform: incant_doc::Transform = serde_json::from_value(value.clone())?;
                if transform.translation != runtime.translation
                    || transform.rotation != runtime.rotation
                {
                    transform.translation = runtime.translation;
                    transform.rotation = runtime.rotation;
                    commands.push(Command::SetComponent {
                        scene_id: runtime.scene_id.clone(),
                        entity_id: runtime.id.clone(),
                        component: "Transform".into(),
                        value: serde_json::to_value(transform)?,
                    });
                }
            }
            if entity.components.contains_key("RigidBody") {
                for (name, value) in [
                    ("Velocity", serde_json::json!({"linear": runtime.velocity})),
                    (
                        "AngularVelocity",
                        serde_json::json!({"angular": runtime.angular_velocity}),
                    ),
                ] {
                    if entity.components.get(name) != Some(&value) {
                        commands.push(Command::SetComponent {
                            scene_id: runtime.scene_id.clone(),
                            entity_id: runtime.id.clone(),
                            component: name.into(),
                            value,
                        });
                    }
                }
            }
        }
        if !commands.is_empty() {
            self.bus.execute(
                commands,
                Actor {
                    origin: Origin::Script,
                    actor: "bevy-simulation".into(),
                    model: None,
                    conversation_id: None,
                },
                "Simulation tick",
                None,
            )?;
        }
        let events = snapshot.trigger_events;
        let count = self
            .host
            .tick_with_events(&mut self.bus, self.dt, &events)?;
        self.engine.sync(self.bus.project())?;
        Ok(count)
    }
}
