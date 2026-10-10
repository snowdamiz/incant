//! Bounded structured script output. Only committed ticks publish their logs.
use serde::{Deserialize, Serialize};

pub(crate) const MAX_PENDING_LOGS: usize = 256;
pub(crate) const MAX_PENDING_BYTES: usize = 256 * 1024;
pub(crate) const MAX_TICK_LOGS: usize = 64;
pub(crate) const MAX_MESSAGE_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptLog {
    pub level: LogLevel,
    pub message: String,
}

pub(crate) fn fits(pending: &[ScriptLog], next: &[ScriptLog]) -> bool {
    next.len() <= MAX_TICK_LOGS
        && pending.len() + next.len() <= MAX_PENDING_LOGS
        && next
            .iter()
            .all(|entry| entry.message.len() <= MAX_MESSAGE_BYTES)
        && pending
            .iter()
            .chain(next)
            .map(|entry| entry.message.len())
            .sum::<usize>()
            <= MAX_PENDING_BYTES
}
