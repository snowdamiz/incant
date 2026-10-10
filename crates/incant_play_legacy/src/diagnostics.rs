//! Bounded diagnostics from untrusted JavaScript; never stringify arbitrary objects.
use crate::{ScriptError, budget::ExecutionBudget};
use rquickjs::{Ctx, Error, Object, Value};
use std::sync::Mutex;

pub(super) fn allocation(error: Error) -> ScriptError {
    if matches!(error, Error::Allocation) {
        ScriptError::MemoryAllocation
    } else {
        ScriptError::Execution
    }
}
fn expired(deadline: &Mutex<ExecutionBudget>) -> Option<ScriptError> {
    deadline
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .check()
        .err()
}
pub(super) fn check_deadline(deadline: &Mutex<ExecutionBudget>) -> Result<(), ScriptError> {
    deadline.lock().unwrap_or_else(|e| e.into_inner()).check()
}
fn primitive_text(value: Value<'_>) -> Option<String> {
    // No Coerced<String>, toString(), stack getter or source dump. String values
    // are copied directly; other thrown values remain explicitly unidentified.
    value.as_string().and_then(|s| s.to_string().ok())
}
fn error_property<'js>(ctx: &Ctx<'js>, object: &Object<'js>, key: &str) -> Option<String> {
    match object.get::<_, Value>(key) {
        Ok(value) => primitive_text(value),
        Err(_) => {
            // Discard a secondary getter exception; don't retain it in the VM
            // or recursively try to describe it.
            drop(ctx.catch());
            None
        }
    }
}
fn bounded(text: &str) -> String {
    let mut output = String::new();
    for c in text
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
    {
        if output.len() + c.len_utf8() > 1024 {
            output.push('…');
            break;
        }
        output.push(c);
    }
    output
}
pub(super) fn javascript(
    ctx: &Ctx<'_>,
    error: Error,
    phase: &'static str,
    deadline: &Mutex<ExecutionBudget>,
) -> ScriptError {
    if let Some(error) = expired(deadline) {
        return error;
    }
    if !matches!(error, Error::Exception) {
        return allocation(error);
    }
    let thrown = ctx.catch();
    let message = if let Some(text) = primitive_text(thrown.clone()) {
        text
    } else if let Some(object) = thrown.as_object().filter(|o| o.is_error()) {
        // An Error may have hostile property getters. The VM interrupt
        // deadline remains active during reads, and expiry wins below.
        let name = error_property(ctx, object, "name").unwrap_or_else(|| "Error".into());
        let message = error_property(ctx, object, "message");
        match message {
            Some(message) => format!("{}: {}", bounded(&name), bounded(&message)),
            None => bounded(&name),
        }
    } else {
        "non-string JavaScript exception".into()
    };
    if let Some(error) = expired(deadline) {
        return error;
    }
    ScriptError::Javascript {
        phase,
        message: bounded(&message),
    }
}
