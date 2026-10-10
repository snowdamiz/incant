//! Reject deferred work, including promises a callback creates but never returns.
use crate::ScriptError;
use rquickjs::{Ctx, Runtime, promise::PromiseHookType, qjs};
use std::{cell::Cell, rc::Rc};

pub(crate) struct Synchronous(Rc<Cell<u8>>);

impl Synchronous {
    pub fn install(runtime: &Runtime) -> Self {
        let count = Rc::new(Cell::new(0u8));
        let observed = Rc::clone(&count);
        runtime.set_promise_hook(Some(Box::new(move |_, event, _, _| {
            // No JS calls, allocation, roots or runtime locking in this hook.
            if event == PromiseHookType::Init {
                observed.set(observed.get().saturating_add(1).min(3));
            }
        })));
        Self(count)
    }

    pub fn reset(&self) {
        self.0.set(0);
    }

    pub fn check(&self, ctx: &Ctx<'_>, module_initialization: bool) -> Result<(), ScriptError> {
        // Pinned QuickJS creates two promises before executing this single
        // synchronous JS module: js_evaluate_module's completion promise, then
        // js_execute_sync_module -> js_async_function_call's execution promise.
        // Any additional Init belongs to project code. Ticks create none. This
        // baseline is tested and must be re-audited on engine/loader changes.
        let expected = if module_initialization { 2 } else { 0 };
        // SAFETY: ctx is live on this runtime's owner thread, inside the context
        // lock. These functions only inspect its pending-job queue. Calling the
        // safe Runtime wrapper here would attempt to acquire the same lock.
        let pending = unsafe { qjs::JS_IsJobPending(qjs::JS_GetRuntime(ctx.as_raw().as_ptr())) };
        if self.0.get() != expected || pending {
            return Err(ScriptError::UnsupportedAsync);
        }
        Ok(())
    }
}
