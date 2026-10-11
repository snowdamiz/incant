//! Native QuickJS host over borrowed ECS numeric columns. No authoring model,
//! command bus, JSON projection or filesystem/network capability is installed.
//! The prior document-backed host lives temporarily in incant_play_legacy.
pub mod artifact;
mod budget;
mod commands;
mod compiled;
#[cfg(feature = "compiler")]
mod compiler;
mod diagnostics;
mod input;
mod loan;
mod queries;
mod query_values;
mod schedule;
mod session;
mod synchronous;
mod timers;

pub use artifact::{EmbeddedScript, ScriptMetadata};
pub use compiled::CompiledScript;
use incant_runtime::NativeWorld;
use rquickjs::{Context, Function, Persistent, Runtime, Value};
pub use session::{NativeSession, PendingNativeSession, SessionError};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("script engine failed without a JavaScript diagnostic")]
    Execution,
    #[error("cooked script engine/build/target or artifact format is incompatible")]
    ArtifactCompatibility,
    #[error("cooked script bytecode integrity check failed")]
    ArtifactIntegrity,
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
    #[error("invalid or throwing physics query arguments")]
    QueryInput,
    #[error("invalid timer ID, tick delay or timer command arguments")]
    TimerInput,
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
    #[error("script host failed; recreate it before continuing")]
    FailedSession,
}

/// Single-threaded native host. Compile source once, then instantiate from the
/// private, compiler-produced bytecode. Ticks borrow real numeric ECS columns;
/// they neither parse code nor serialize project/component values.
///
/// This native host exposes `update({chunks, dt, commands, input, physics})`. Each chunk has stable
/// ID strings, Transform stride 10, and optional Velocity stride 3. Views are
/// detached on every exit. A failed callback restores its component writes and
/// permanently fails this host. Tick-local structural commands apply only after
/// detachment and scope exit; rejection restores numeric writes too.
/// Physics queries read committed solver state during the loan; retained capabilities
/// expire before diagnostics. Fixed-tick timers publish bounded data events;
/// callback code, persistent saves, logs and the legacy PlaySession are not migrated.
pub struct ScriptHost {
    // Drop VM roots/context/runtime before bytecode: QuickJS's ROM-data module
    // loader may retain references to its input allocation.
    update: Persistent<Function<'static>>,
    context: Context,
    _runtime: Runtime,
    _compiled: CompiledScript,
    synchronous: synchronous::Synchronous,
    deadline: Arc<Mutex<budget::ExecutionBudget>>,
    schedule: schedule::Schedule,
    failed: bool,
}

impl ScriptHost {
    pub fn new(compiled: CompiledScript) -> Result<Self, ScriptError> {
        Self::with_budget(compiled, Duration::from_millis(50))
    }

    pub fn with_budget(compiled: CompiledScript, limit: Duration) -> Result<Self, ScriptError> {
        let (runtime, context, deadline) = vm(limit, compiled.allow_source)?;
        context.with(|ctx| {
            compiled
                .harden(&ctx)
                .map_err(|error| diagnostics::javascript(&ctx, error, "initialization", &deadline))
        })?;
        // Fixed engine hardening runs first. Its two internal module promises
        // must not change the guest module's separately measured baseline.
        let synchronous = synchronous::Synchronous::install(&runtime);
        let update = context.with(|ctx| {
            let update = compiled.instantiate(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "initialization", &deadline)
            })?;
            synchronous.check(&ctx, true)?;
            Ok::<_, ScriptError>(Persistent::save(&ctx, update))
        })?;
        diagnostics::check_deadline(&deadline)?;
        Ok(Self {
            update,
            context,
            _runtime: runtime,
            _compiled: compiled,
            synchronous,
            deadline,
            schedule: schedule::Schedule::default(),
            failed: false,
        })
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
        if self.failed {
            return Err(ScriptError::FailedSession);
        }
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(ScriptError::InputLimit);
        }
        self.deadline
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .reset()?;
        self.synchronous.reset();
        let result = self.context.with(|ctx| {
            let (mut schedule, events) = self.schedule.advance(dt)?;
            let timers = timers::TickTimers::default();
            let timer_api = timers.expose(&ctx, events).map_err(|error| {
                diagnostics::javascript(&ctx, error, "timer bindings", &self.deadline)
            })?;
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
            let commands = commands::TickCommands::default();
            let api = commands.expose(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "command bindings", &self.deadline)
            })?;
            // Input publication/command construction must fit the script budget
            // before any ECS table is borrowed or its numeric views are created.
            diagnostics::check_deadline(&self.deadline)?;
            world
                .with_numeric_physics(|columns, capability| {
                    let queries =
                        queries::TickQueries::new(capability, Arc::clone(&self.deadline), dt);
                    let physics = queries.expose(&ctx).map_err(|error| {
                        diagnostics::javascript(&ctx, error, "physics bindings", &self.deadline)
                    })?;
                    let mut loan = loan::Loan::new(columns)?;
                    let chunks = match loan.expose(&ctx) {
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
                    commands.activate();
                    queries.activate();
                    timers.activate();
                    let result = function
                        .call::<_, Value>((chunks, dt, api, input, physics, clock, timer_api));
                    commands.revoke();
                    queries.revoke();
                    timers.revoke();
                    // Revoke before diagnostics can execute a user-defined getter,
                    // and before Rust reads or restores any component bytes.
                    loan.revoke();
                    let returned = result.map_err(|error| {
                        diagnostics::javascript(&ctx, error, "tick", &self.deadline)
                    })?;
                    diagnostics::check_deadline(&self.deadline)?;
                    self.synchronous.check(&ctx, false)?;
                    if returned.is_promise() {
                        return Err(ScriptError::UnsupportedAsync);
                    }
                    if !returned.is_undefined() {
                        return Err(ScriptError::UnsupportedReturn);
                    }
                    queries.check()?;
                    loan.validate_numeric()?;
                    let commands = commands.take()?;
                    schedule.apply(timers.take()?)?;
                    Ok((loan.finish(), commands, schedule))
                })
                .map_err(ScriptError::Structural)?
        });
        // No JS value or raw column pointer crosses this sync point. Apply is
        // native host work, after the script CPU/deferred-work checks; its time
        // is separate from the JS execution budget, not a claimed phone budget.
        let result = result.and_then(|(undo, mut commands, schedule)| {
            undo.apply(world, &mut commands)?;
            self.schedule = schedule;
            Ok(())
        });
        if result.is_err() {
            self.failed = true;
        }
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
        interrupt
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .expired()
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
mod structural_tests;
#[cfg(all(test, feature = "compiler"))]
mod tests;

#[cfg(all(test, feature = "compiler"))]
mod input_tests;

#[cfg(all(test, feature = "compiler"))]
mod query_tests;

#[cfg(all(test, feature = "compiler"))]
mod character_tests;

#[cfg(all(test, feature = "compiler"))]
mod timer_tests;
