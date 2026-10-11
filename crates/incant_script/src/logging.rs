//! Bounded primitive-only diagnostics, committed with a successful script phase.
//! A retained log function loses authority when its update exits.
use crate::ScriptError;
use rquickjs::{Ctx, Exception, Function, Value, function::Rest};
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

const MAX_MESSAGE_BYTES: usize = 1024;
const MAX_TICK_ENTRIES: usize = 64;
const MAX_TICK_BYTES: usize = 16 * 1024;
const MAX_RETAINED_ENTRIES: usize = 1024;
const MAX_RETAINED_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct NativeLog {
    /// Accepted script update, matching the native script clock.
    pub tick: u64,
    pub level: LogLevel,
    pub message: String,
}
/// Bounded prefix of committed messages. Once full, further committed messages
/// increment `dropped` instead of consuming additional host storage.
#[derive(Default, Debug, Serialize)]
pub struct NativeLogs {
    entries: Vec<NativeLog>,
    dropped: u64,
    #[serde(skip)]
    bytes: usize,
}
// Static modules currently have no logging ABI; their committed log is empty.
#[cfg(feature = "static-module")]
pub(crate) static NO_MODULE_LOGS: NativeLogs = NativeLogs {
    entries: Vec::new(),
    dropped: 0,
    bytes: 0,
};
impl NativeLogs {
    pub fn entries(&self) -> &[NativeLog] {
        &self.entries
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    pub(crate) fn commit(&mut self, tick: u64, records: Vec<(LogLevel, String)>) {
        for (level, message) in records {
            if self.dropped > 0
                || self.entries.len() == MAX_RETAINED_ENTRIES
                || message.len() > MAX_RETAINED_BYTES - self.bytes
            {
                self.dropped = self.dropped.saturating_add(1);
                continue;
            }
            self.bytes += message.len();
            self.entries.push(NativeLog {
                tick,
                level,
                message,
            });
        }
    }
}
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
