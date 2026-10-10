//! QuickJS sandbox for SWC-compiled TypeScript. No OS, filesystem, module loader,
//! network, native plugins or shell are installed into the runtime.
mod diagnostics;
mod localization;
mod logs;
mod play;
mod saves;
mod timers;
pub use play::PlaySession;
pub use saves::{GameSave, MAX_SAVE_BYTES, SaveError};
pub use timers::{MAX_TIMERS, ScriptClock, TimerError, TimerEvent, TimerRequest};
mod queries;
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::Origin;
pub use logs::{LogLevel, ScriptLog};
use queries::{CharacterMover, Navigator, Raycaster, Steerer};
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
    #[error("script engine failed without a JavaScript diagnostic")]
    Execution,
    #[error("script execution exceeded its {budget_ms} ms wall-clock budget")]
    ExecutionDeadline { budget_ms: u128 },
    #[error("script engine memory allocation failed (configured limit: 32 MiB)")]
    MemoryAllocation,
    #[error("script {phase} failed: {message}")]
    Javascript {
        phase: &'static str,
        message: String,
    },
    #[error("script result is invalid: {0}")]
    Result(#[from] serde_json::Error),
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error(transparent)]
    Physics(#[from] incant_core::PhysicsError),
    #[error(transparent)]
    Input(#[from] incant_input::InputError),
    #[error(transparent)]
    InputRecording(#[from] incant_input::RecordingError),
    #[error("live input cannot be mixed with an installed input replay")]
    InputReplayConflict,
    #[error("play session failed; restart or restore a saved game before continuing")]
    FailedSession,
    #[error("script input exceeds configured limit")]
    InputLimit,
    #[error("script logs exceed the message, tick or pending-output limit")]
    LogLimit,
    #[error(
        "async/Promise and generator behavior callbacks are unsupported; use fixed-tick timers"
    )]
    UnsupportedAsync,
    #[error("a behavior with pending timers requires an onTimer handler")]
    TimerHandler,
    #[error(transparent)]
    Timer(#[from] TimerError),
    #[error(transparent)]
    Localization(#[from] incant_localization::LocalizationError),
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
}

#[derive(Deserialize)]
struct TickResult {
    state: Value,
    commands: Vec<Command>,
    logs: Vec<ScriptLog>,
    timers: Vec<timers::Action>,
    asynchronous: bool,
}
#[derive(Deserialize)]
struct InitialState {
    state: Value,
    has_timer_handler: bool,
}
pub struct ScriptHost {
    source_sha256: String,
    localization: Arc<Mutex<localization::State>>,
    raycaster: Option<Raycaster>,
    character_mover: Option<CharacterMover>,
    navigator: Option<Navigator>,
    steerer: Option<Steerer>,
    query_count: Arc<std::sync::atomic::AtomicUsize>,
    context: Context,
    _runtime: Runtime,
    deadline: Arc<Mutex<Instant>>,
    budget: Duration,
    state: Value,
    logs: Vec<ScriptLog>,
    has_timer_handler: bool,
    schedule: timers::Schedule,
}
impl ScriptHost {
    pub fn new(compiled_source: &str) -> Result<Self, ScriptError> {
        Self::with_budget(compiled_source, Duration::from_millis(50))
    }
    pub fn with_budget(compiled_source: &str, budget: Duration) -> Result<Self, ScriptError> {
        if compiled_source.len() > 1_000_000 {
            return Err(ScriptError::InputLimit);
        }
        let runtime = Runtime::new().map_err(diagnostics::allocation)?;
        runtime.set_memory_limit(32 * 1024 * 1024);
        runtime.set_max_stack_size(1024 * 1024);
        let deadline = Arc::new(Mutex::new(Instant::now() + budget));
        let timer = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            Instant::now() >= *timer.lock().unwrap_or_else(|e| e.into_inner())
        })));
        let context = Context::full(&runtime).map_err(diagnostics::allocation)?;
        let source = format!(
            r#"
   globalThis.defineBehavior = x => x;
   const __behavior = (function(exports) {{ "use strict"; {compiled_source}
; return exports.default; }})({{}});
   if (!__behavior || typeof __behavior.update !== 'function') throw new Error('default behavior.update required');
   let __state = JSON.parse(JSON.stringify(__behavior.initialState ?? {{}}));
   {}
   JSON.stringify({{state: __state, has_timer_handler: typeof __behavior.onTimer === 'function'}});
  "#,
            include_str!("runtime.js")
        );
        let initial = context.with(|ctx| {
            ctx.eval::<String, _>(source).map_err(|error| {
                diagnostics::javascript(&ctx, error, "initialization", &deadline, budget)
            })
        })?;
        let initial: InitialState = serde_json::from_str(&initial)?;
        if serde_json::to_vec(&initial.state)?.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        let mut host = Self {
            source_sha256: saves::hash(compiled_source.as_bytes()),
            localization: Arc::new(Mutex::new(localization::State::default())),
            raycaster: None,
            character_mover: None,
            navigator: None,
            steerer: None,
            query_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            context,
            _runtime: runtime,
            deadline,
            budget,
            state: initial.state,
            has_timer_handler: initial.has_timer_handler,
            schedule: timers::Schedule::default(),
            logs: Vec::new(),
        };
        host.install_queries()?;
        host.install_localization()?;
        Ok(host)
    }
    pub fn state(&self) -> &Value {
        &self.state
    }
    pub fn clock(&self) -> &ScriptClock {
        &self.schedule.clock
    }
    /// Consume committed output. Call regularly to keep the pending queue bounded.
    pub fn take_logs(&mut self) -> Vec<ScriptLog> {
        std::mem::take(&mut self.logs)
    }
    pub fn hot_reload(&mut self, source: &str) -> Result<(), ScriptError> {
        let mut next = Self::with_budget(source, self.budget)?;
        next.raycaster = self.raycaster.clone();
        next.character_mover = self.character_mover.clone();
        next.navigator = self.navigator.clone();
        next.steerer = self.steerer.clone();
        next.install_queries()?;
        if self.schedule.has_timers() && !next.has_timer_handler {
            return Err(ScriptError::TimerHandler);
        }
        next.schedule = self.schedule.clone();
        // Recreating the VM for identical source has no schema migration: keep
        // runtime-grown arrays and dynamic object keys as well as scalar fields.
        next.state = if next.source_sha256 == self.source_sha256 {
            self.state.clone()
        } else {
            preserve_compatible(&self.state, &next.state)
        };
        next.logs = std::mem::take(&mut self.logs);
        *self = next;
        Ok(())
    }
    pub fn tick(&mut self, bus: &mut CommandBus, dt: f64) -> Result<usize, ScriptError> {
        let mut input = incant_input::InputRuntime::default();
        input.map_actions(&bus.project().settings.input_actions)?;
        self.tick_with_events(bus, dt, &[], input.frame(), |_| Ok(()))
    }
    fn tick_with_events(
        &mut self,
        bus: &mut CommandBus,
        dt: f64,
        events: &[incant_core::TriggerEvent],
        input: &incant_input::InputFrame,
        validate_runtime: impl FnOnce(&incant_doc::Project) -> Result<(), ScriptError>,
    ) -> Result<usize, ScriptError> {
        self.query_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(ScriptError::InputLimit);
        }
        self.localization
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .prepare(bus.project())?;
        let (mut schedule, timer_events) = self.schedule.advance(dt)?;
        let world = serde_json::to_string(bus.project())?;
        let state = serde_json::to_string(&self.state)?;
        if world.len() > 16 * 1024 * 1024 || state.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now() + self.budget;
        let result = self.context.with(|ctx| {
            let function: rquickjs::Function = ctx
                .globals()
                .get("__tick")
                .map_err(diagnostics::allocation)?;
            function
                .call::<_, String>((
                    world,
                    dt,
                    state,
                    serde_json::to_string(events).expect("serializable trigger events"),
                    serde_json::to_string(input).expect("validated input frame"),
                    serde_json::to_string(&schedule.clock).expect("validated script clock"),
                    serde_json::to_string(&timer_events).expect("validated timer events"),
                ))
                .map_err(|error| {
                    diagnostics::javascript(&ctx, error, "tick", &self.deadline, self.budget)
                })
        })?;
        // Native queries cannot be interrupted in the middle of a solver call.
        // Even if script code catches a query error, an expired tick must not
        // commit commands, logs or state after control returns to the host.
        diagnostics::check_deadline(&self.deadline, self.budget)?;
        if result.len() > 16 * 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        let output: TickResult = serde_json::from_str(&result)?;
        if output.asynchronous {
            return Err(ScriptError::UnsupportedAsync);
        }
        schedule.apply(output.timers)?;
        if serde_json::to_vec(&output.state)?.len() > 1024 * 1024 {
            return Err(ScriptError::InputLimit);
        }
        if !logs::fits(&self.logs, &output.logs) {
            return Err(ScriptError::LogLimit);
        }
        let count = output.commands.len();
        if count > 10_000 {
            return Err(ScriptError::InputLimit);
        }
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
        validate_runtime(bus.project())?;
        self.state = output.state;
        self.schedule = schedule;
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
        assert!(matches!(
            host.tick(&mut bus, 1. / 60.),
            Err(ScriptError::ExecutionDeadline { budget_ms: 10 })
        ));
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
        assert!(matches!(
            host.tick(&mut bus, 1. / 60.),
            Err(ScriptError::ExecutionDeadline { budget_ms: 50 })
        ));
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
    fn command_limit_is_enforced_even_if_script_replaces_the_javascript_wrapper() {
        let source = r#"exports.default={initialState:{count:0},update(){
          globalThis.__tick=()=>JSON.stringify({state:{count:1},commands:Array(10001).fill(
            {op:'set_memory',section:'prefix',text:'must not commit'}),
            logs:[],timers:[],asynchronous:false});}};"#;
        let mut host = ScriptHost::new(source).unwrap();
        let mut bus = CommandBus::new(Project::empty("Untrusted wrapper")).unwrap();
        host.tick(&mut bus, 1. / 60.).unwrap();
        let before = bus.project().clone();
        assert!(matches!(
            host.tick(&mut bus, 1. / 60.),
            Err(ScriptError::InputLimit)
        ));
        assert_eq!(bus.project(), &before);
        assert_eq!(host.state()["count"], 0);
        assert_eq!(host.clock().tick, 1);
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
