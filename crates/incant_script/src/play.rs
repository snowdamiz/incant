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
    input: incant_input::InputRuntime,
    replay: Option<incant_input::InputReplay>,
}
impl PlaySession {
    pub fn new(project: &incant_doc::Project, compiled_source: &str) -> Result<Self, ScriptError> {
        Self::with_navigation_resources(
            project,
            compiled_source,
            incant_core::NavigationResources::new(),
        )
    }
    pub fn with_navigation_resources(
        project: &incant_doc::Project,
        compiled_source: &str,
        resources: incant_core::NavigationResources,
    ) -> Result<Self, ScriptError> {
        let engine = incant_core::Engine::with_navigation_resources(project, resources)?;
        let mut host = ScriptHost::new(compiled_source)?;
        host.raycaster = Some(Arc::new(engine.raycaster()));
        host.character_mover = Some(Arc::new(engine.character_mover()));
        host.navigator = Some(Arc::new(engine.navigator()));
        host.install_queries()?;
        let mut input = incant_input::InputRuntime::default();
        input.map_actions(&project.settings.input_actions)?;
        Ok(Self {
            host,
            authored_sha256: super::saves::hash(project.canonical_text()?.as_bytes()),
            manifest_sha256: super::saves::manifest(project)?,
            ticks: 0,
            elapsed_seconds: 0.,
            failed: false,
            input,
            replay: None,
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
        let events = self
            .replay
            .as_ref()
            .map(|replay| {
                replay
                    .events_at(self.ticks + 1)
                    .map(<[incant_input::InputEvent]>::to_vec)
            })
            .transpose()?
            .unwrap_or_default();
        self.advance(&events)
    }
    /// Validate a complete clip and seed its physical input history at the
    /// current game clock. A rejected clip leaves the previous input intact.
    pub fn replay_input(&mut self, text: &str) -> Result<(), ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        let replay = incant_input::InputReplay::from_text(
            text,
            self.ticks,
            self.project().settings.tick_rate,
        )?;
        let mut input = replay.initial_state();
        input.map_actions(&self.project().settings.input_actions)?;
        self.input = input;
        self.replay = Some(replay);
        Ok(())
    }
    pub fn input_replay_end(&self) -> Option<u64> {
        self.replay
            .as_ref()
            .map(incant_input::InputReplay::end_tick)
    }
    /// Process a device packet for the next successful game tick. Invalid input
    /// is rejected before physics or script state advances and can be retried.
    pub fn tick_with_input(
        &mut self,
        events: &[incant_input::InputEvent],
    ) -> Result<usize, ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        if self.replay.is_some() {
            return Err(ScriptError::InputReplayConflict);
        }
        self.advance(events)
    }
    fn advance(&mut self, events: &[incant_input::InputEvent]) -> Result<usize, ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        if self.ticks >= super::saves::MAX_SAVE_TICK {
            self.failed = true;
            return Err(ScriptError::InputLimit);
        }
        self.input
            .advance_mapped(events, self.dt, &self.bus.project().settings.input_actions)?;
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
    pub fn input(&self) -> &incant_input::InputFrame {
        self.input.frame()
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
        let count =
            self.host
                .tick_with_events(&mut self.bus, self.dt, &events, self.input.frame())?;
        self.engine.sync(self.bus.project())?;
        Ok(count)
    }
}
