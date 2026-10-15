//! Native QuickJS host over borrowed ECS numeric columns. No authoring model,
//! command bus, JSON projection or filesystem/network capability is installed.
//! The prior document-backed host lives temporarily in incant_play_legacy.
mod animation_commands;
mod animation_graphs;
mod animation_ik;
mod animations;
pub mod artifact;
mod audio_commands;
mod bindings;
#[cfg(feature = "compiler")]
mod bindings_embed;
mod bindings_static;
mod budget;
mod checkpoint;
mod commands;
mod compiled;
#[cfg(feature = "compiler")]
mod compiler;
mod coroutines;
mod diagnostics;
mod flipbooks;
#[cfg(feature = "profile")]
mod heap_profile;
mod hud;
mod input;
mod light_commands;
mod loan;
mod localization_values;
mod logging;
mod navigation_values;
mod persistence;
mod program;
mod queries;
mod query_values;
mod query_values_2d;
mod root_motion;
mod schedule;
mod session;
mod steering_values;
mod synchronous;
#[cfg(all(test, feature = "diagnostic-core-stress"))]
mod tick_probe;
mod timers;

pub use artifact::{EmbeddedScript, ScriptMetadata};
pub use bindings::{
    MAX_SCRIPT_INSTANCES, MAX_SCRIPT_SOURCES, ScriptBindings, ScriptInstance, ScriptValue,
};
pub use bindings_static::{EmbeddedScriptInstance, EmbeddedScriptValue};
pub use checkpoint::{CheckpointError, MAX_CHECKPOINT_BYTES, PendingSessionCheckpoint};
#[cfg(feature = "native-checkpoints")]
pub use checkpoint::{MAX_PERSISTED_SESSION_BYTES, NativeSessionCheckpoint};
pub use compiled::CompiledScript;
#[cfg(feature = "compiler")]
pub use compiler::ScriptBuild;
use incant_runtime::NativeWorld;
pub use logging::{LogLevel, NativeLog, NativeLogs};
pub use program::SessionProgram;
use rquickjs::{Context, Function, Persistent, Runtime, Value};
pub use session::{NativeSession, PendingNativeSession, SessionError};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thiserror::Error;

/// Accepted metadata for the trusted composed owner. This does not mutate a
/// world and contains no persisted command or guest-supplied startup payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRequest {
    Restart,
}

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error(transparent)]
    Haptics(#[from] incant_input::haptics::HapticError),
    #[error("this host does not accept session restart requests")]
    SessionRequestsUnsupported,
    #[error("script engine failed without a JavaScript diagnostic")]
    Execution,
    #[error("cooked script engine/build/target or artifact format is incompatible")]
    ArtifactCompatibility,
    #[error("cooked script bytecode integrity check failed")]
    ArtifactIntegrity,
    #[error("native script source/attachment catalog or cooked scene pairing is invalid")]
    ScriptBindings,
    #[error("script execution exceeded its {budget_ms} ms thread CPU budget")]
    ExecutionDeadline { budget_ms: u128 },
    #[error("script CPU budget clock {reason}")]
    CpuClock { reason: &'static str },
    #[error("script engine memory allocation failed (configured limit: 32 MiB)")]
    MemoryAllocation,
    #[error("script {phase} failed: {message}")]
    Javascript {
        phase: &'static str,
        message: String,
    },
    #[error(
        "script source, component loan, metadata or command buffer exceeds the configured limit"
    )]
    InputLimit,
    #[error("invalid or throwing native structural command arguments")]
    CommandInput,
    #[error("invalid or throwing native query arguments")]
    QueryInput,
    #[error("invalid timer ID, tick delay or timer command arguments")]
    TimerInput,
    #[error("invalid or excessive native coroutine operation")]
    CoroutineInput,
    #[error("invalid native log arguments or control characters")]
    LogInput,
    #[error("native script clock or timer deadline exceeds its numeric range")]
    TimerRange,
    #[error(transparent)]
    PhysicsQuery(#[from] incant_runtime::QueryError),
    #[error("native structural batch failed: {0}")]
    Structural(#[from] incant_runtime::RuntimeError),
    #[error("promises and deferred work are unsupported in synchronous native behaviors")]
    UnsupportedAsync,
    #[error("native bulk behavior callbacks must return undefined")]
    UnsupportedReturn,
    #[error("invalid, excessive or ungranted persistent-state access")]
    PersistenceInput,
    #[error("script host failed; recreate it before continuing")]
    FailedSession,
}

/// Single-threaded native host. Compile source once, then instantiate from the
/// private, compiler-produced bytecode. Ticks borrow real numeric ECS columns;
/// they neither parse code nor serialize project/component values.
///
/// This native host exposes `update({chunks, dt, commands, input, physics, navigation, localization, clock, timers, log, flipbooks, animations})`. Each chunk has stable
/// ID strings, Transform stride 10, and optional Velocity stride 3. Views are
/// detached on every exit. A failed callback restores its component writes and
/// permanently fails this host. Tick-local structural commands apply only after
/// detachment and scope exit; rejection restores numeric writes too.
/// Physics queries read committed solver state during the loan; retained capabilities
/// expire before diagnostics. Fixed-tick timers publish bounded data events;
/// callback code is never serialized. Opted-in native sessions can checkpoint
/// typed persistent data; this is not an arbitrary VM heap snapshot.
/// Bounded primitive logs publish only after a successful script transaction.
pub struct ScriptHost {
    // Drop VM roots/context/runtime before bytecode: QuickJS's ROM-data module
    // loader may retain references to its input allocation.
    update: Persistent<Function<'static>>,
    freeze: Persistent<Function<'static>>,
    persistent_wrap: Persistent<Function<'static>>,
    coroutine_shape: Option<Persistent<Function<'static>>>,
    persistent_opt_in: Vec<bool>,
    coroutine_catalog: coroutines::Catalog,
    persistent: std::rc::Rc<std::cell::RefCell<persistence::Store>>,
    project: Option<bindings::BoundProject>,
    chunk_ids: loan::IdCache,
    loan_scratch: loan::Scratch,
    context: Context,
    _runtime: Runtime,
    _compiled: CompiledScript,
    synchronous: synchronous::Synchronous,
    deadline: Arc<Mutex<budget::ExecutionBudget>>,
    schedule: schedule::Schedule,
    logs: NativeLogs,
    flipbooks: flipbooks::Scratch,
    animations: animations::Scratch,
    restart_requests: bool,
    session_request: Option<SessionRequest>,
    haptic_capabilities: incant_input::haptics::HapticCapabilities,
    haptic_requests: incant_input::haptics::HapticRequests,
    failed: bool,
}

impl ScriptHost {
    pub fn new(compiled: CompiledScript) -> Result<Self, ScriptError> {
        Self::with_budget(compiled, Duration::from_millis(50))
    }

    pub fn with_budget(compiled: CompiledScript, limit: Duration) -> Result<Self, ScriptError> {
        Self::initialize(compiled, limit, None, None)
    }

    /// Attach the compiler-produced catalog to this exact fresh native world.
    /// Plain Spawn never inherits an attachment, including reuse of a stable ID.
    pub fn for_world(compiled: CompiledScript, world: &NativeWorld) -> Result<Self, ScriptError> {
        Self::initialize(compiled, Duration::from_millis(50), Some(world), None)
    }

    pub(crate) fn for_checkpoint(
        compiled: CompiledScript,
        world: &NativeWorld,
        active: &[incant_runtime::StableId],
    ) -> Result<Self, ScriptError> {
        Self::initialize(
            compiled,
            Duration::from_millis(50),
            Some(world),
            Some(active),
        )
    }
    fn initialize(
        compiled: CompiledScript,
        limit: Duration,
        world: Option<&NativeWorld>,
        active: Option<&[incant_runtime::StableId]>,
    ) -> Result<Self, ScriptError> {
        if compiled.bindings.is_some() && world.is_none() {
            return Err(ScriptError::ScriptBindings);
        }
        let (runtime, context, deadline) = vm(limit, compiled.allow_source)?;
        let (freeze, persistent_wrap, persistent_shape, project) = context.with(|ctx| {
            let (freeze, persistent_wrap, persistent_shape) =
                compiled.harden(&ctx).map_err(|error| {
                    diagnostics::javascript(&ctx, error, "initialization", &deadline)
                })?;
            let project = compiled
                .bindings
                .as_ref()
                .map(|bindings| {
                    let world = world.expect("checked native world");
                    match active {
                        Some(ids) => {
                            bindings::BoundProject::restore(&ctx, &freeze, bindings, world, ids)
                        }
                        None => bindings::BoundProject::new(&ctx, &freeze, bindings, world),
                    }
                })
                .transpose()?;
            Ok::<_, ScriptError>((
                Persistent::save(&ctx, freeze),
                Persistent::save(&ctx, persistent_wrap),
                Persistent::save(&ctx, persistent_shape),
                project,
            ))
        })?;
        // Fixed engine hardening runs first. Its two internal module promises
        // must not change the guest module's separately measured baseline.
        let synchronous = synchronous::Synchronous::install(&runtime);
        let (update, persistent_opt_in, coroutine_catalog) = context.with(|ctx| {
            let (update, persistent_opt_in, coroutine_catalog) =
                compiled.instantiate(&ctx).map_err(|error| {
                    diagnostics::javascript(&ctx, error, "initialization", &deadline)
                })?;
            synchronous.check(&ctx, true)?;
            Ok::<_, ScriptError>((
                Persistent::save(&ctx, update),
                persistent_opt_in,
                coroutine_catalog,
            ))
        })?;
        diagnostics::check_deadline(&deadline)?;
        let coroutine_shape = coroutine_catalog.enabled().then_some(persistent_shape);
        Ok(Self {
            update,
            freeze,
            persistent_wrap,
            coroutine_shape,
            persistent_opt_in,
            coroutine_catalog,
            persistent: Default::default(),
            project,
            chunk_ids: Default::default(),
            loan_scratch: Default::default(),
            context,
            _runtime: runtime,
            _compiled: compiled,
            synchronous,
            deadline,
            schedule: schedule::Schedule::default(),
            logs: NativeLogs::default(),
            flipbooks: flipbooks::Scratch::default(),
            animations: animations::Scratch::default(),
            restart_requests: false,
            session_request: None,
            haptic_capabilities: Default::default(),
            haptic_requests: Default::default(),
            failed: false,
        })
    }

    /// Borrow the bounded committed log prefix. Failed hosts expose no live state.
    pub fn logs(&self) -> Result<&NativeLogs, ScriptError> {
        if self.failed {
            Err(ScriptError::FailedSession)
        } else {
            Ok(&self.logs)
        }
    }

    /// Trusted owner opt-in. The caller must consume requests after its complete
    /// composed tick succeeds. Ordinary hosts refuse the primitive by default.
    pub fn enable_restart_requests(&mut self) -> Result<(), ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        self.restart_requests = true;
        Ok(())
    }

    /// Script-phase acceptance only. A composed host must wait for its audio
    /// and mandatory consumer acceptance before exposing this request.
    pub fn take_session_request(&mut self) -> Result<Option<SessionRequest>, ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        Ok(self.session_request.take())
    }

    /// Current trusted host authority; never read from a checkpoint or recording.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), ScriptError> {
        self.haptics_ready()?;
        self.haptic_capabilities = value;
        Ok(())
    }
    pub(crate) fn haptics_ready(&self) -> Result<(), ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        if !self.haptic_requests.is_empty() {
            return Err(incant_input::haptics::HapticError::Pending.into());
        }
        Ok(())
    }
    /// Consume only after the complete composed tick has accepted. Delivery is
    /// asynchronous host work, not proof of physical actuator success.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, ScriptError> {
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        Ok(std::mem::take(&mut self.haptic_requests))
    }
    /// Host suspension/replacement discards transient effects and authority.
    pub fn clear_haptics(&mut self) {
        self.haptic_requests = Default::default();
        self.haptic_capabilities = Default::default();
    }

    /// Run only the script phase. Native simulation stepping remains explicitly
    /// owned by the caller. Tick is unchanged; hierarchy is refreshed on return.
    /// Undo storage is bounded to 32 MiB of numeric columns, separate from the
    /// VM's 32 MiB memory cap. No per-tick authoring validation is performed.
    pub fn tick(&mut self, world: &mut NativeWorld, dt: f64) -> Result<(), ScriptError> {
        self.tick_with_input(world, dt, &incant_input::InputRuntime::default())
    }

    /// Typed input comes only from an engine-validated, bounded runtime. This
    /// method owns the script phase only; NativeSession owns fixed-step ordering.
    pub fn tick_with_input(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &incant_input::InputRuntime,
    ) -> Result<(), ScriptError> {
        #[cfg(feature = "diagnostic-script-phases")]
        let mut attribution = incant_profile::diagnostic::Frame::start();
        let _profile = incant_profile::scope(incant_profile::Phase::ScriptTick);
        #[cfg(all(test, feature = "diagnostic-core-stress"))]
        tick_probe::mark(tick_probe::Region::Enter);
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        self.haptics_ready()?;
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(ScriptError::InputLimit);
        }
        self.deadline
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .reset()?;
        self.synchronous.reset();
        #[cfg(all(test, feature = "diagnostic-core-stress"))]
        tick_probe::mark(tick_probe::Region::InitialChecks);
        #[cfg(feature = "diagnostic-script-phases")]
        incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::Snapshots);
        let result = self.context.with(|ctx| {
            self.flipbooks.capture(world)?;
            let flipbooks = self.flipbooks.expose(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "flipbook snapshot", &self.deadline)
            })?;
            self.animations.capture(world)?;
            let animations = self.animations.expose(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "animation snapshot", &self.deadline)
            })?;
            #[cfg(all(test, feature = "diagnostic-core-stress"))]
            tick_probe::mark(tick_probe::Region::Snapshots);
            #[cfg(feature = "diagnostic-script-phases")]
            incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::GroupSetup);
            let freeze = self
                .freeze
                .clone()
                .restore(&ctx)
                .map_err(diagnostics::allocation)?;
            let mut schedule = self.schedule.clone();
            let inspect = self
                .persistent_wrap
                .clone()
                .restore(&ctx)
                .map_err(diagnostics::allocation)?;
            let mut groups = Vec::new();
            let mut retired = Vec::new();
            if let Some(project) = &mut self.project {
                if project.incarnation != world.incarnation() {
                    return Err(ScriptError::ScriptBindings);
                }
                for (index, group) in project.groups.iter_mut().enumerate() {
                    let (instances, ids) = group
                        .snapshot(&ctx, &freeze, world, &mut retired)
                        .map_err(diagnostics::allocation)?;
                    let scope = schedule::Scope::Source(group.source);
                    if instances.is_empty() {
                        schedule.cancel_scope(scope);
                    } else {
                        if let Some(tasks) = &mut schedule.coroutines {
                            tasks.retain(|key, task| {
                                key.scope != scope
                                    || task.entity.is_none_or(|id| ids.binary_search(&id).is_ok())
                            });
                        }
                        groups.push((index, scope, instances, ids));
                    }
                }
            } else {
                groups.push((
                    0,
                    schedule::Scope::Global,
                    rquickjs::Array::new(ctx.clone()).map_err(diagnostics::allocation)?,
                    Vec::new().into(),
                ));
            }
            let persistent =
                persistence::TickState::new(std::rc::Rc::clone(&self.persistent), retired);
            let (mut schedule, events) = schedule.advance(dt)?;
            let logs = logging::TickLogs::default();
            let log_api = logs.expose(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "log bindings", &self.deadline)
            })?;
            let timers = timers::TickTimers::default();
            // No per-tick queue/API allocation for programs without a catalog.
            let coroutines = self.coroutine_catalog.enabled().then(|| {
                coroutines::Tick::new(
                    schedule.coroutines.take().unwrap_or_default(),
                    schedule.tick,
                )
            });
            let coroutine_shape = self
                .coroutine_shape
                .as_ref()
                .map(|shape| shape.clone().restore(&ctx))
                .transpose()
                .map_err(diagnostics::allocation)?;
            let groups = groups
                .into_iter()
                .map(|(index, scope, instances, ids)| {
                    let api = timers.expose(&ctx, scope, &events).map_err(|error| {
                        diagnostics::javascript(&ctx, error, "timer bindings", &self.deadline)
                    })?;
                    let coroutine_api = coroutines
                        .as_ref()
                        .map(|tasks| {
                            tasks.expose(
                                &ctx,
                                scope,
                                Arc::clone(&self.coroutine_catalog.0[index]),
                                Arc::clone(&ids),
                                &inspect,
                            )
                        })
                        .transpose()
                        .map_err(diagnostics::allocation)?;
                    let state_api = persistent
                        .expose(
                            &ctx,
                            scope,
                            ids,
                            self.persistent_opt_in[index],
                            inspect.clone(),
                        )
                        .map_err(diagnostics::allocation)?;
                    Ok::<_, ScriptError>((index, scope, instances, api, state_api, coroutine_api))
                })
                .collect::<Result<Vec<_>, _>>()?;
            #[cfg(all(test, feature = "diagnostic-core-stress"))]
            tick_probe::mark(tick_probe::Region::GroupSetup);
            #[cfg(feature = "diagnostic-script-phases")]
            incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::ApiSetup);
            let clock = timers::clock(&ctx, &schedule).map_err(|error| {
                diagnostics::javascript(&ctx, error, "script clock", &self.deadline)
            })?;
            let input = input::expose(&ctx, input.frame()).map_err(|error| {
                diagnostics::javascript(&ctx, error, "input snapshot", &self.deadline)
            })?;
            diagnostics::check_deadline(&self.deadline)?;
            let function = self
                .update
                .clone()
                .restore(&ctx)
                .map_err(diagnostics::allocation)?;
            let commands =
                commands::TickCommands::new(self.restart_requests, self.haptic_capabilities);
            let hud = hud::expose(&ctx, world)?;
            let api = commands.expose(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "command bindings", &self.deadline)
            })?;
            // Input publication/command construction must fit the script budget
            // before any ECS table is borrowed or its numeric views are created.
            diagnostics::check_deadline(&self.deadline)?;
            #[cfg(feature = "diagnostic-script-phases")]
            incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::Capabilities);
            world
                .with_numeric_physics(|columns, capability| {
                    let queries =
                        queries::TickQueries::new(capability, Arc::clone(&self.deadline), dt);
                    let physics = queries.expose(&ctx).map_err(|error| {
                        diagnostics::javascript(&ctx, error, "physics bindings", &self.deadline)
                    })?;
                    let navigation = queries.expose_navigation(&ctx).map_err(|error| {
                        diagnostics::javascript(&ctx, error, "navigation bindings", &self.deadline)
                    })?;
                    let localization = queries.expose_localization(&ctx).map_err(|error| {
                        diagnostics::javascript(
                            &ctx,
                            error,
                            "localization bindings",
                            &self.deadline,
                        )
                    })?;
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::Capabilities);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::UndoCapture,
                    );
                    let mut loan = loan::Loan::new(columns, &mut self.loan_scratch)?;
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::UndoCapture);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::ViewExposure,
                    );
                    let chunks = match loan.expose(&ctx, &freeze, &mut self.chunk_ids) {
                        Ok(chunks) => chunks,
                        Err(error) => {
                            // Prototype setters can run while metadata is installed.
                            // Revoke partial loans before inspecting their exception.
                            loan.revoke();
                            return Err(diagnostics::javascript(
                                &ctx,
                                error,
                                "component views",
                                &self.deadline,
                            ));
                        }
                    };
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::ViewExposure);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::Callbacks);
                    commands.activate();
                    queries.activate();
                    logs.activate();
                    let mut result = Ok(());
                    for (index, scope, instances, timer_api, state_api, coroutine_api) in groups {
                        timers.activate(scope);
                        persistent.activate(scope);
                        if let Some(tasks) = &coroutines {
                            tasks.activate(scope);
                        }
                        let source_profile = incant_profile::script_scope(match scope {
                            schedule::Scope::Global => None,
                            schedule::Scope::Source(id) => Some(id.0),
                        });
                        #[cfg(feature = "diagnostic-script-phases")]
                        let callback_attribution =
                            incant_profile::diagnostic::Span::callback(match scope {
                                schedule::Scope::Global => None,
                                schedule::Scope::Source(id) => Some(id.0),
                            });
                        let called = if coroutines.is_none() {
                            // Keep the original no-task call shape and moved
                            // scoped objects; no extra argument/clone per callback.
                            function.call::<_, Value>((
                                rquickjs::function::Flat((
                                    chunks.clone(),
                                    dt,
                                    api.clone(),
                                    input.clone(),
                                    physics.clone(),
                                    navigation.clone(),
                                    localization.clone(),
                                )),
                                rquickjs::function::Flat((clock.clone(), timer_api)),
                                rquickjs::function::Flat((
                                    log_api.clone(),
                                    flipbooks.clone(),
                                    animations.clone(),
                                )),
                                index as u32,
                                instances,
                                state_api,
                                hud.clone(),
                            ))
                        } else {
                            let call = |task: Value<'_>, routine: &str| {
                                function.call::<_, Value>((
                                    rquickjs::function::Flat((
                                        chunks.clone(),
                                        dt,
                                        api.clone(),
                                        input.clone(),
                                        physics.clone(),
                                        navigation.clone(),
                                        localization.clone(),
                                    )),
                                    rquickjs::function::Flat((clock.clone(), timer_api.clone())),
                                    rquickjs::function::Flat((
                                        log_api.clone(),
                                        flipbooks.clone(),
                                        animations.clone(),
                                    )),
                                    index as u32,
                                    instances.clone(),
                                    state_api.clone(),
                                    hud.clone(),
                                    coroutine_api.clone(),
                                    task,
                                    routine,
                                ))
                            };
                            call(Value::new_undefined(ctx.clone()), "").and_then(|value| {
                                if !value.is_undefined() {
                                    return Ok(value);
                                }
                                if let Some(tasks) = &coroutines {
                                    if tasks.check().is_err() {
                                        return Err(rquickjs::Exception::throw_type(
                                            &ctx,
                                            "coroutine transaction failed",
                                        ));
                                    }
                                    for (key, task) in tasks.due(scope) {
                                        // Never run another callback after deferred work or deadline failure.
                                        if self.synchronous.check(&ctx, false).is_err()
                                            || diagnostics::check_deadline(&self.deadline).is_err()
                                        {
                                            break;
                                        }
                                        if !commands.healthy()
                                            || !queries.healthy()
                                            || !timers.healthy()
                                            || !persistent.healthy()
                                            || !logs.healthy()
                                            || tasks.check().is_err()
                                        {
                                            return Err(rquickjs::Exception::throw_type(
                                                &ctx,
                                                "script transaction failed before coroutine resume",
                                            ));
                                        }
                                        if !tasks.current(&key, &task) {
                                            continue;
                                        }
                                        let argument = tasks.task(&ctx, &freeze, &key, &task)?;
                                        let result = call(argument.into_value(), &task.routine)?;
                                        if result.is_promise() {
                                            return Ok(result);
                                        }
                                        if self.synchronous.check(&ctx, false).is_err()
                                            || diagnostics::check_deadline(&self.deadline).is_err()
                                        {
                                            break;
                                        }
                                        // The inspector itself may run traps. Normal guards are
                                        // checked again before another continuation or commit.
                                        tasks.finish(
                                            &ctx,
                                            coroutine_shape
                                                .as_ref()
                                                .expect("enabled coroutine inspector"),
                                            &key,
                                            &task,
                                            result,
                                        )?;
                                    }
                                }
                                Ok(value)
                            })
                        };
                        #[cfg(feature = "diagnostic-script-phases")]
                        drop(callback_attribution);
                        drop(source_profile);
                        timers.revoke();
                        persistent.revoke();
                        if let Some(tasks) = &coroutines {
                            tasks.revoke();
                        }
                        match called {
                            Err(error) => {
                                result = Err(error);
                                break;
                            }
                            Ok(value) => {
                                // Don't run another source after a disallowed return.
                                // Native capabilities/loans revoke before diagnostics.
                                if !value.is_undefined() {
                                    commands.revoke();
                                    queries.revoke();
                                    logs.revoke();
                                    loan.revoke();
                                    return Err(if value.is_promise() {
                                        ScriptError::UnsupportedAsync
                                    } else {
                                        ScriptError::UnsupportedReturn
                                    });
                                }
                                if let Err(error) = self
                                    .synchronous
                                    .check(&ctx, false)
                                    .and_then(|()| diagnostics::check_deadline(&self.deadline))
                                {
                                    commands.revoke();
                                    queries.revoke();
                                    logs.revoke();
                                    loan.revoke();
                                    return Err(error);
                                }
                            }
                        }
                    }
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::Callbacks);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::RevokeChecks,
                    );
                    commands.revoke();
                    queries.revoke();
                    timers.revoke();
                    logs.revoke();
                    // Revoke before diagnostics can execute a user-defined getter,
                    // and before Rust reads or restores any component bytes.
                    loan.revoke();
                    result.map_err(|error| {
                        diagnostics::javascript(&ctx, error, "tick", &self.deadline)
                    })?;
                    diagnostics::check_deadline(&self.deadline)?;
                    self.synchronous.check(&ctx, false)?;
                    queries.check()?;
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::RevokeChecks);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::Validation,
                    );
                    loan.validate_numeric()?;
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::NumericValidation);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::TransactionStage,
                    );
                    let (commands, request, haptics) = commands.take()?;
                    schedule.apply(timers.take()?)?;
                    if let Some(tasks) = &coroutines {
                        let tasks = tasks.take()?;
                        schedule.coroutines = (!tasks.entries.is_empty()).then_some(tasks);
                    }
                    let logs = logs.take()?;
                    let changes = persistent.take()?;
                    #[cfg(all(test, feature = "diagnostic-core-stress"))]
                    tick_probe::mark(tick_probe::Region::TransactionStage);
                    #[cfg(feature = "diagnostic-script-phases")]
                    incant_profile::diagnostic::mark(
                        incant_profile::diagnostic::Region::ScopeCleanup,
                    );
                    Ok((
                        loan.finish(),
                        commands,
                        request,
                        haptics,
                        schedule,
                        logs,
                        changes,
                    ))
                })
                .map_err(ScriptError::Structural)?
        });

        #[cfg(all(test, feature = "diagnostic-core-stress"))]
        tick_probe::mark(tick_probe::Region::ScopeCleanup);
        // No JS value or raw column pointer crosses this sync point. Apply is
        // native host work, after the script CPU/deferred-work checks; its time
        // is separate from the JS execution budget, not a claimed phone budget.
        #[cfg(feature = "diagnostic-script-phases")]
        incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::Publication);
        let result = result.and_then(
            |(undo, mut commands, request, haptics, schedule, logs, changes)| {
                let _profile = incant_profile::scope(incant_profile::Phase::ScriptApply);
                undo.apply(world, &mut commands)?;
                self.logs.commit(schedule.tick, logs);
                self.schedule = schedule;
                self.persistent.borrow_mut().apply(changes);
                self.session_request = self.session_request.or(request);
                self.haptic_requests = haptics;
                Ok(())
            },
        );
        if result.is_err() {
            self.failed = true;
        }
        #[cfg(all(test, feature = "diagnostic-core-stress"))]
        tick_probe::mark(tick_probe::Region::Publication);
        #[cfg(feature = "diagnostic-script-phases")]
        incant_profile::diagnostic::mark(incant_profile::diagnostic::Region::HeapObservation);
        // After all JS/context loans close; outside the guest CPU budget.
        #[cfg(feature = "profile")]
        heap_profile::observe(&self._runtime);

        #[cfg(all(test, feature = "diagnostic-core-stress"))]
        tick_probe::mark(tick_probe::Region::HeapTelemetry);
        #[cfg(feature = "diagnostic-script-phases")]
        attribution.finish(result.is_ok());
        result
    }
}

type Vm = (Runtime, Context, Arc<Mutex<budget::ExecutionBudget>>);
fn vm(limit: Duration, allow_source: bool) -> Result<Vm, ScriptError> {
    let runtime = Runtime::new().map_err(diagnostics::allocation)?;
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(1024 * 1024);
    let deadline = Arc::new(Mutex::new(budget::ExecutionBudget::new(limit)?));
    let interrupt = Arc::clone(&deadline);
    runtime.set_interrupt_handler(Some(Box::new(move || {
        #[cfg(feature = "diagnostic-script-budget")]
        let observed = incant_profile::diagnostic::budget::Span::start(
            incant_profile::diagnostic::budget::Origin::Interrupt,
        );
        let expired = interrupt
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .expired();
        #[cfg(feature = "diagnostic-script-budget")]
        observed.finish(!expired);
        expired
    })));
    let context = bytecode_context(&runtime, allow_source)?;
    Ok((runtime, context, deadline))
}

fn bytecode_context(runtime: &Runtime, allow_source: bool) -> Result<Context, ScriptError> {
    if allow_source {
        #[cfg(feature = "compiler")]
        return Context::full(runtime).map_err(diagnostics::allocation);
    }
    {
        use rquickjs::context::intrinsic as i;
        // Deliberately omit Eval. All source, including engine hardening, was
        // compiled by the cook; dynamic Function/eval routes fail explicitly.
        Context::custom::<(
            i::Date,
            i::RegExpCompiler,
            i::RegExp,
            i::Json,
            i::Proxy,
            i::MapSet,
            i::TypedArrays,
            i::Promise,
            i::Performance,
            i::WeakRef,
            i::DOMException,
        )>(runtime)
        .map_err(diagnostics::allocation)
    }
}

#[cfg(all(test, feature = "compiler"))]
mod light_tests;
#[cfg(all(test, feature = "compiler"))]
mod structural_tests;
#[cfg(all(test, feature = "compiler"))]
mod tests;

#[cfg(all(test, feature = "compiler"))]
mod loan_id_tests;

#[cfg(all(test, feature = "compiler"))]
mod input_tests;

#[cfg(all(test, feature = "compiler"))]
mod navigation_tests;
#[cfg(all(test, feature = "compiler"))]
mod query_2d_tests;
#[cfg(all(test, feature = "compiler"))]
mod query_tests;
#[cfg(all(test, feature = "compiler"))]
mod steering_tests;

#[cfg(all(test, feature = "compiler"))]
mod character_tests;

#[cfg(all(test, feature = "compiler"))]
mod log_tests;
#[cfg(all(test, feature = "compiler"))]
mod timer_tests;

#[cfg(all(test, feature = "compiler"))]
mod localization_tests;

#[cfg(test)]
mod hud_tests;

#[cfg(all(test, feature = "compiler"))]
mod restart_request_tests;

#[cfg(all(test, feature = "compiler"))]
mod haptic_tests;

#[cfg(all(test, feature = "compiler", feature = "diagnostic-script-phases"))]
mod attribution_tests;
