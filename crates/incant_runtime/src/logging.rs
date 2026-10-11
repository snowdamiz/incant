//! Bounded committed script diagnostics shared by native script hosts.
use serde::Serialize;

pub const MAX_MESSAGE_BYTES: usize = 1024;
pub const MAX_TICK_ENTRIES: usize = 64;
pub const MAX_TICK_BYTES: usize = 16 * 1024;
pub const MAX_RETAINED_ENTRIES: usize = 1024;
pub const MAX_RETAINED_BYTES: usize = 256 * 1024;
/// Match the native QuickJS successful-update clock and JSON integer precision.
pub const MAX_LOG_TICK: u64 = (1 << 53) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
impl NativeLogs {
    pub fn entries(&self) -> &[NativeLog] {
        &self.entries
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    /// Trusted host staging only: each VM validates messages and update quotas
    /// before calling this after acceptance. This does not grant guest authority.
    #[doc(hidden)]
    pub fn commit(&mut self, tick: u64, records: Vec<(LogLevel, String)>) {
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
