//! QuickJS sandbox for SWC-compiled TypeScript. No OS, filesystem, module loader,
//! network, native plugins or shell are installed into the runtime.
mod logs;
mod play;
mod saves;
pub use play::PlaySession;
pub use saves::{GameSave, MAX_SAVE_BYTES, SaveError};
mod queries;
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::Origin;
pub use logs::{LogLevel, ScriptLog};
use queries::{CharacterMover, Raycaster};
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
    #[error(transparent)]
    Physics(#[from] incant_core::PhysicsError),
    #[error("play session failed; restart or restore a saved game before continuing")]
    FailedSession,
    #[error("script input exceeds configured limit")]
    InputLimit,
    #[error("script logs exceed the message, tick or pending-output limit")]
    LogLimit,
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
}

#[derive(Deserialize)]
struct TickResult {
    state: Value,
    commands: Vec<Command>,
    logs: Vec<ScriptLog>,
}
pub struct ScriptHost {
    source_sha256: String,
    raycaster: Option<Raycaster>,
    character_mover: Option<CharacterMover>,
    query_count: Arc<std::sync::atomic::AtomicUsize>,
    context: Context,
    _runtime: Runtime,
    deadline: Arc<Mutex<Instant>>,
    budget: Duration,
    state: Value,
    logs: Vec<ScriptLog>,
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
   globalThis.__tick = (worldJson, dt, stateJson, eventsJson) => {{
     const world = JSON.parse(worldJson);
     const state = JSON.parse(stateJson);
     const commands = [];
     const logs = [];
     const api = Object.freeze({{
       raycast: (query) => {{
         const result = JSON.parse(globalThis.__incantRaycast(JSON.stringify(query)));
         if (result.error) throw new Error(result.error);
         return result.hit;
       }},
       computeCharacterMotion: (query) => {{
         const result = JSON.parse(globalThis.__incantCharacterMotion(JSON.stringify(query)));
         if (result.error) throw new Error(result.error);
         return result.movement;
       }},
       triggerEvents: () => JSON.parse(eventsJson),
       query: (component) => Object.values(world.scenes).flatMap(scene => Object.values(scene.entities)
         .filter(entity => !component || Object.hasOwn(entity.components, component))
         .map(entity => ({{...entity, scene_id: scene.id}}))),
       command: (command) => {{ if (commands.length >= 10000) throw new Error('command limit'); commands.push(command); }},
       log: (message, level = 'info') => {{
         if (typeof message !== 'string' || message.length > 4096 || logs.length >= 64 ||
             !['debug', 'info', 'warn', 'error'].includes(level)) throw new Error('invalid script log');
         logs.push({{level, message}});
       }}
     }});
     __behavior.update(api, dt, state);
     return JSON.stringify({{state, commands, logs}});
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
        let mut host = Self {
            source_sha256: saves::hash(compiled_source.as_bytes()),
            raycaster: None,
            character_mover: None,
            query_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            context,
            _runtime: runtime,
            deadline,
            budget,
            state: serde_json::from_str(&state)?,
            logs: Vec::new(),
        };
        host.install_queries()?;
        Ok(host)
    }
    pub fn state(&self) -> &Value {
        &self.state
    }
    /// Consume committed output. Call regularly to keep the pending queue bounded.
    pub fn take_logs(&mut self) -> Vec<ScriptLog> {
        std::mem::take(&mut self.logs)
    }
    pub fn hot_reload(&mut self, source: &str) -> Result<(), ScriptError> {
        let mut next = Self::with_budget(source, self.budget)?;
        next.raycaster = self.raycaster.clone();
        next.character_mover = self.character_mover.clone();
        next.install_queries()?;
        next.state = preserve_compatible(&self.state, &next.state);
        next.logs = std::mem::take(&mut self.logs);
        *self = next;
        Ok(())
    }
    pub fn tick(&mut self, bus: &mut CommandBus, dt: f64) -> Result<usize, ScriptError> {
        self.tick_with_events(bus, dt, &[])
    }
    fn tick_with_events(
        &mut self,
        bus: &mut CommandBus,
        dt: f64,
        events: &[incant_core::TriggerEvent],
    ) -> Result<usize, ScriptError> {
        self.query_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
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
                function.call::<_, String>((
                    world,
                    dt,
                    state,
                    serde_json::to_string(events).expect("serializable trigger events"),
                ))
            })
            .map_err(|_| ScriptError::Execution)?;
        // Native queries cannot be interrupted in the middle of a solver call.
        // Even if script code catches a query error, an expired tick must not
        // commit commands, logs or state after control returns to the host.
        if Instant::now() >= *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) {
            return Err(ScriptError::Execution);
        }
        if result.len() > 16 * 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        let output: TickResult = serde_json::from_str(&result)?;
        if serde_json::to_vec(&output.state)?.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        if !logs::fits(&self.logs, &output.logs) {
            return Err(ScriptError::LogLimit);
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
        self.logs.extend(output.logs);
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
    fn native_query_expiry_prevents_commit_even_when_the_script_catches_errors() {
        let source = r#"exports.default={initialState:{ok:false},update(api,dt,state){
          try { api.raycast({scene_id:'x',origin:[0,0,0],direction:[0,-1,0],max_distance:1,include_sensors:false,exclude_entity:null,memberships:1,filter:1}); } catch(e) {}
          state.ok=true; api.log('must not commit');
        }};"#;
        let mut host = ScriptHost::new(source).unwrap();
        let deadline = host.deadline.clone();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mark = called.clone();
        host.raycaster = Some(Arc::new(move |_| {
            mark.store(true, std::sync::atomic::Ordering::Relaxed);
            *deadline.lock().unwrap() = Instant::now() - Duration::from_secs(1);
            Ok(None)
        }));
        host.install_queries().unwrap();
        let mut bus = CommandBus::new(Project::empty("query deadline")).unwrap();
        assert!(host.tick(&mut bus, 1. / 60.).is_err());
        assert!(called.load(std::sync::atomic::Ordering::Relaxed));
        assert_eq!(host.state()["ok"], false);
        assert!(host.take_logs().is_empty());
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
