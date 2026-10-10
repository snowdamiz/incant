//! Native QuickJS host over borrowed ECS numeric columns. No authoring model,
//! command bus, JSON projection or filesystem/network capability is installed.
//! The prior document-backed host lives temporarily in incant_play_legacy.
mod budget;
mod compiled;
mod diagnostics;
mod loan;
mod synchronous;

pub use compiled::CompiledScript;
use incant_runtime::NativeWorld;
use rquickjs::{Context, Function, Persistent, Runtime, Value};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("script engine failed without a JavaScript diagnostic")]
    Execution,
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
    #[error("script source, component loan or metadata exceeds the configured limit")]
    InputLimit,
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
/// This first native host exposes `update({chunks, dt})`. Each chunk has stable
/// ID strings, Transform stride 10, and optional Velocity stride 3. Views are
/// detached on every exit. A failed callback restores its component writes and
/// permanently fails this host. Persistent state, structural commands, timers,
/// logs, queries and the legacy PlaySession are not migrated by this boundary.
pub struct ScriptHost {
    // Drop VM roots/context/runtime before bytecode: QuickJS's ROM-data module
    // loader may retain references to its input allocation.
    update: Persistent<Function<'static>>,
    context: Context,
    _runtime: Runtime,
    _compiled: CompiledScript,
    synchronous: synchronous::Synchronous,
    deadline: Arc<Mutex<budget::ExecutionBudget>>,
    failed: bool,
}

impl ScriptHost {
    pub fn new(compiled: CompiledScript) -> Result<Self, ScriptError> {
        Self::with_budget(compiled, Duration::from_millis(50))
    }

    pub fn with_budget(compiled: CompiledScript, limit: Duration) -> Result<Self, ScriptError> {
        let (runtime, context, deadline) = vm(limit)?;
        let synchronous = synchronous::Synchronous::install(&runtime);
        let update = context.with(|ctx| {
            ctx.eval::<(), _>(include_str!("harden.js"))
                .map_err(|error| {
                    diagnostics::javascript(&ctx, error, "initialization", &deadline)
                })?;
            let update = compiled.instantiate(&ctx).map_err(|error| {
                diagnostics::javascript(&ctx, error, "initialization", &deadline)
            })?;
            synchronous.check(&ctx, true)?;
            Ok(Persistent::save(&ctx, update))
        })?;
        diagnostics::check_deadline(&deadline)?;
        Ok(Self {
            update,
            context,
            _runtime: runtime,
            _compiled: compiled,
            synchronous,
            deadline,
            failed: false,
        })
    }

    /// Run only the script phase. Native simulation stepping remains explicitly
    /// owned by the caller. Tick is unchanged; hierarchy is refreshed on return.
    /// Undo storage is bounded to 32 MiB of numeric columns, separate from the
    /// VM's 32 MiB memory cap. No per-tick authoring validation is performed.
    pub fn tick(&mut self, world: &mut NativeWorld, dt: f64) -> Result<(), ScriptError> {
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
            let function = self
                .update
                .clone()
                .restore(&ctx)
                .map_err(diagnostics::allocation)?;
            world.with_numeric_columns(|columns| {
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
                let result = function.call::<_, Value>((chunks, dt));
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
                loan.commit();
                Ok(())
            })
        });
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

type Vm = (Runtime, Context, Arc<Mutex<budget::ExecutionBudget>>);
fn vm(limit: Duration) -> Result<Vm, ScriptError> {
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
    let context = Context::full(&runtime).map_err(diagnostics::allocation)?;
    Ok((runtime, context, deadline))
}

#[cfg(test)]
mod tests;
