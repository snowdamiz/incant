//! QuickJS sandbox for SWC-compiled TypeScript. No OS, filesystem, module loader,
//! network, native plugins or shell are installed into the runtime.
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::Origin;
use rquickjs::{Context, Runtime};
use serde::Deserialize;
use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("script failed (syntax, exception, memory limit or execution deadline)")]
    Execution,
    #[error("script result is invalid: {0}")]
    Result(#[from] serde_json::Error),
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error("script input exceeds configured limit")]
    InputLimit,
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
}

/// A disposable play session. ECS query results enter the same validated runtime
/// command bus before scripts read them; the author's document never changes.
pub struct PlaySession {
    pub host: ScriptHost,
    bus: CommandBus,
    engine: incant_core::Engine,
    dt: f64,
}
impl PlaySession {
    pub fn new(project: &incant_doc::Project, compiled_source: &str) -> Result<Self, ScriptError> {
        Ok(Self {
            host: ScriptHost::new(compiled_source)?,
            bus: CommandBus::simulation(project.clone())?,
            engine: incant_core::Engine::new(project)?,
            dt: 1. / f64::from(project.settings.tick_rate),
        })
    }
    pub fn project(&self) -> &incant_doc::Project {
        self.bus.project()
    }
    pub fn snapshot(&mut self) -> incant_core::RuntimeSnapshot {
        self.engine.snapshot()
    }
    pub fn tick(&mut self) -> Result<usize, ScriptError> {
        self.engine.step();
        let mut commands = Vec::new();
        for runtime in self.engine.snapshot().entities.values() {
            let entity = &self.bus.project().scenes[&runtime.scene_id].entities[&runtime.id];
            if let Some(value) = entity.components.get("Transform") {
                let mut transform: incant_doc::Transform = serde_json::from_value(value.clone())?;
                if transform.translation != runtime.translation {
                    transform.translation = runtime.translation;
                    commands.push(Command::SetComponent {
                        scene_id: runtime.scene_id.clone(),
                        entity_id: runtime.id.clone(),
                        component: "Transform".into(),
                        value: serde_json::to_value(transform)?,
                    });
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
        let count = self.host.tick(&mut self.bus, self.dt)?;
        self.engine.sync(self.bus.project())?;
        Ok(count)
    }
}
#[derive(Deserialize)]
struct TickResult {
    state: Value,
    commands: Vec<Command>,
}
pub struct ScriptHost {
    context: Context,
    _runtime: Runtime,
    deadline: Arc<Mutex<Instant>>,
    budget: Duration,
    state: Value,
}
impl ScriptHost {
    pub fn new(compiled_source: &str) -> Result<Self, ScriptError> {
        Self::with_budget(compiled_source, Duration::from_millis(50))
    }
    pub fn with_budget(compiled_source: &str, budget: Duration) -> Result<Self, ScriptError> {
        if compiled_source.len() > 1_000_000 {
            return Err(ScriptError::InputLimit);
        }
        let runtime = Runtime::new().map_err(|_| ScriptError::Execution)?;
        runtime.set_memory_limit(32 * 1024 * 1024);
        runtime.set_max_stack_size(1024 * 1024);
        let deadline = Arc::new(Mutex::new(Instant::now() + budget));
        let timer = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            Instant::now() >= *timer.lock().unwrap_or_else(|e| e.into_inner())
        })));
        let context = Context::full(&runtime).map_err(|_| ScriptError::Execution)?;
        let source = format!(
            r#"
   globalThis.defineBehavior = x => x;
   const __behavior = (function(exports) {{ "use strict"; {compiled_source}
; return exports.default; }})({{}});
   if (!__behavior || typeof __behavior.update !== 'function') throw new Error('default behavior.update required');
   let __state = JSON.parse(JSON.stringify(__behavior.initialState ?? {{}}));
   globalThis.__tick = (worldJson, dt, stateJson) => {{
     const world = JSON.parse(worldJson);
     const state = JSON.parse(stateJson);
     const commands = [];
     const api = Object.freeze({{
       query: (component) => Object.values(world.scenes).flatMap(scene => Object.values(scene.entities)
         .filter(entity => !component || Object.hasOwn(entity.components, component))
         .map(entity => ({{...entity, scene_id: scene.id}}))),
       command: (command) => {{ if (commands.length >= 10000) throw new Error('command limit'); commands.push(command); }}
     }});
     __behavior.update(api, dt, state);
     return JSON.stringify({{state, commands}});
   }};
   JSON.stringify(__state);
  "#
        );
        let state = context
            .with(|ctx| ctx.eval::<String, _>(source))
            .map_err(|_| ScriptError::Execution)?;
        if state.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        Ok(Self {
            context,
            _runtime: runtime,
            deadline,
            budget,
            state: serde_json::from_str(&state)?,
        })
    }
    pub fn state(&self) -> &Value {
        &self.state
    }
    pub fn hot_reload(&mut self, source: &str) -> Result<(), ScriptError> {
        let mut next = Self::with_budget(source, self.budget)?;
        next.state = preserve_compatible(&self.state, &next.state);
        *self = next;
        Ok(())
    }
    pub fn tick(&mut self, bus: &mut CommandBus, dt: f64) -> Result<usize, ScriptError> {
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(ScriptError::InputLimit);
        }
        let world = serde_json::to_string(bus.project())?;
        let state = serde_json::to_string(&self.state)?;
        if world.len() > 16 * 1024 * 1024 || state.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now() + self.budget;
        let result = self
            .context
            .with(|ctx| {
                let function: rquickjs::Function = ctx.globals().get("__tick")?;
                function.call::<_, String>((world, dt, state))
            })
            .map_err(|_| ScriptError::Execution)?;
        if result.len() > 16 * 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        let output: TickResult = serde_json::from_str(&result)?;
        if serde_json::to_vec(&output.state)?.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        let count = output.commands.len();
        if count > 0 {
            bus.execute(
                output.commands,
                Actor {
                    origin: Origin::Script,
                    actor: "quickjs".into(),
                    model: None,
                    conversation_id: None,
                },
                "Script tick",
                None,
            )?;
        }
        self.state = output.state;
        Ok(count)
    }
}
fn preserve_compatible(old: &Value, new: &Value) -> Value {
    match (old, new) {
        (Value::Object(old), Value::Object(new)) => Value::Object(
            new.iter()
                .map(|(key, new)| {
                    (
                        key.clone(),
                        old.get(key)
                            .map_or_else(|| new.clone(), |old| preserve_compatible(old, new)),
                    )
                })
                .collect(),
        ),
        (Value::Array(old), Value::Array(new)) if old.len() == new.len() => Value::Array(
            old.iter()
                .zip(new)
                .map(|(a, b)| preserve_compatible(a, b))
                .collect(),
        ),
        (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_))
        | (Value::Bool(_), Value::Bool(_))
        | (Value::Null, Value::Null) => old.clone(),
        _ => new.clone(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_doc::Project;
    use serde_json::json;
    #[test]
    fn sandbox_has_no_host_capabilities() {
        let source = r#"exports.default=defineBehavior({initialState:{safe:false},update(api,dt,state){state.safe=[typeof require,typeof process,typeof fetch,typeof std,typeof os].every(x=>x==='undefined');}});"#;
        let mut host = ScriptHost::new(source).unwrap();
        let mut bus = CommandBus::new(Project::empty("sandbox")).unwrap();
        host.tick(&mut bus, 1. / 60.).unwrap();
        assert_eq!(host.state()["safe"], true);
    }
    #[test]
    fn deadline_interrupts_infinite_loop() {
        let mut host = ScriptHost::with_budget(
            "exports.default={update(){while(true){}}};",
            Duration::from_millis(10),
        )
        .unwrap();
        let mut bus = CommandBus::new(Project::empty("sandbox")).unwrap();
        let start = Instant::now();
        assert!(host.tick(&mut bus, 1. / 60.).is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn hot_reload_preserves_matching_types_and_old_program_on_error() {
        let mut host=ScriptHost::new("exports.default={initialState:{count:0,changed:1},update(api,dt,state){state.count++;}};").unwrap();
        let mut bus = CommandBus::new(Project::empty("sandbox")).unwrap();
        host.tick(&mut bus, 1. / 60.).unwrap();
        assert!(host.hot_reload("syntax !!!").is_err());
        host.tick(&mut bus, 1. / 60.).unwrap();
        host.hot_reload(
            "exports.default={initialState:{count:0,changed:'new',added:true},update(){}};",
        )
        .unwrap();
        assert_eq!(
            host.state(),
            &json!({"count":2,"changed":"new","added":true})
        );
    }
    #[test]
    fn invalid_script_transaction_does_not_change_state_or_document() {
        let source = "exports.default={initialState:{count:0},update(api,dt,state){state.count++;api.command({op:'set_memory',section:'ok',text:'change'});api.command({op:'delete_scene',scene_id:'missing'});}};";
        let mut host = ScriptHost::new(source).unwrap();
        let mut bus = CommandBus::new(Project::empty("sandbox")).unwrap();
        let before = bus.project().clone();
        assert!(host.tick(&mut bus, 1. / 60.).is_err());
        assert_eq!(bus.project(), &before);
        assert_eq!(host.state()["count"], 0);
    }
    #[test]
    fn scripts_query_live_ecs_and_hot_reload_without_changing_authored_state() {
        let mut project = Project::empty("play integration");
        let mut scene = incant_doc::Scene::new("world");
        let mut entity = incant_doc::Entity::new("moving");
        let id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(incant_doc::Transform::default()));
        entity
            .components
            .insert("Velocity".into(), json!({"linear":[60.,0.,0.]}));
        scene.entities.insert(id.clone(), entity);
        project.scenes.insert(scene.id.clone(), scene);
        let authored = project.canonical_text().unwrap();
        let source = "exports.default={initialState:{seen:0},update(api,dt,state){state.seen=api.query('Transform')[0].components.Transform.translation[0];}};";
        let mut play = PlaySession::new(&project, source).unwrap();
        play.tick().unwrap();
        assert_eq!(play.host.state()["seen"], 1.);
        play.host.hot_reload(source).unwrap();
        play.tick().unwrap();
        assert_eq!(play.host.state()["seen"], 2.);
        assert_eq!(play.snapshot().entities[&id].translation[0], 2.);
        drop(play);
        assert_eq!(project.canonical_text().unwrap(), authored);
    }
}
