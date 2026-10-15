//! Bounded primitive-only diagnostics, committed with a successful script phase.
//! A retained log function loses authority when its update exits.
use crate::ScriptError;
use rquickjs::{Ctx, Exception, Function, Value, function::Rest};
use std::{cell::RefCell, rc::Rc};

pub use incant_runtime::logging::{LogLevel, NativeLog, NativeLogs};
use incant_runtime::logging::{MAX_MESSAGE_BYTES, MAX_TICK_BYTES, MAX_TICK_ENTRIES};
#[derive(Default)]
struct State {
    active: bool,
    failure: Option<ScriptError>,
    bytes: usize,
    entries: Vec<(LogLevel, String)>,
}
#[derive(Default)]
pub(crate) struct TickLogs(Rc<RefCell<State>>);
impl TickLogs {
    pub fn healthy(&self) -> bool {
        self.0.borrow().failure.is_none()
    }
    pub fn activate(&self) {
        self.0.borrow_mut().active = true;
    }
    pub fn revoke(&self) {
        self.0.borrow_mut().active = false;
    }
    pub fn take(&self) -> Result<Vec<(LogLevel, String)>, ScriptError> {
        self.revoke();
        let mut state = self.0.borrow_mut();
        if let Some(error) = state.failure.take() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.entries))
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Function<'js>> {
        let state = Rc::clone(&self.0);
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
            let mut state = state.borrow_mut();
            if !state.active || state.failure.is_some() {
                return Err(Exception::throw_type(
                    &ctx,
                    "log belongs to its active update only",
                ));
            }
            // Strict primitive conversion never invokes guest getters/toString.
            let record = parse(&args.0).and_then(|(level, message)| {
                if state.entries.len() == MAX_TICK_ENTRIES
                    || message.len() > MAX_TICK_BYTES - state.bytes
                {
                    return Err(ScriptError::InputLimit);
                }
                Ok((level, message))
            });
            match record {
                Ok((level, message)) => {
                    state.bytes += message.len();
                    state.entries.push((level, message));
                    Ok(())
                }
                Err(error) => {
                    state.failure = Some(error);
                    Err(Exception::throw_type(
                        &ctx,
                        "expected bounded log(message, optional debug/info/warn/error level)",
                    ))
                }
            }
        })
    }
}
impl Drop for TickLogs {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = false;
        state.entries = Vec::new();
        state.bytes = 0;
    }
}
fn parse(args: &[Value<'_>]) -> Result<(LogLevel, String), ScriptError> {
    if !(1..=2).contains(&args.len()) {
        return Err(ScriptError::LogInput);
    }
    let level = if args.len() == 1 || args[1].is_undefined() {
        LogLevel::Info
    } else {
        match bounded_string(&args[1], 5)?.as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => return Err(ScriptError::LogInput),
        }
    };
    let message = bounded_string(&args[0], MAX_MESSAGE_BYTES)?;
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(ScriptError::LogInput);
    }
    Ok((level, message))
}
fn bounded_string(value: &Value<'_>, maximum: usize) -> Result<String, ScriptError> {
    let string = value.as_string().ok_or(ScriptError::LogInput)?;
    // VM-managed conversion can contain lone UTF-16 surrogates. Only inspect
    // CString's byte length, never its unchecked as_str/AsRef/Deref methods.
    let bytes = string
        .clone()
        .to_cstring()
        .map_err(crate::diagnostics::allocation)?;
    if bytes.len() > maximum {
        return Err(ScriptError::InputLimit);
    }
    drop(bytes);
    // This conversion validates UTF-8 before allocating the bounded Rust String.
    string.to_string().map_err(|_| ScriptError::LogInput)
}
