//! Copied primitive records belonging to one active Wasm update.
use crate::BoundaryError as Error;
use incant_runtime::logging::{
    LogLevel, MAX_MESSAGE_BYTES, MAX_RETAINED_BYTES, MAX_RETAINED_ENTRIES, MAX_TICK_BYTES,
    MAX_TICK_ENTRIES,
};

// Registry bounds must match the unchanged native QuickJS diagnostic contract.
const _: () = {
    use incant_wasm_abi as abi;
    assert!(MAX_MESSAGE_BYTES == abi::LOG_MAX_MESSAGE_BYTES);
    assert!(MAX_TICK_ENTRIES == abi::LOG_MAX_TICK_ENTRIES);
    assert!(MAX_TICK_BYTES == abi::LOG_MAX_TICK_BYTES);
    assert!(MAX_RETAINED_ENTRIES == abi::LOG_MAX_RETAINED_ENTRIES);
    assert!(MAX_RETAINED_BYTES == abi::LOG_MAX_RETAINED_BYTES);
};

#[derive(Default)]
pub(crate) struct TickLogs {
    records: Vec<(LogLevel, String)>,
    bytes: usize,
    pub rejected: bool,
}
impl TickLogs {
    pub fn clear(&mut self) {
        self.records.clear();
        self.bytes = 0;
        self.rejected = false;
    }
    pub fn take(&mut self) -> Vec<(LogLevel, String)> {
        self.bytes = 0;
        std::mem::take(&mut self.records)
    }
    pub fn push(&mut self, level: i32, bytes: &[u8]) -> Result<(), Error> {
        if self.rejected {
            return Err(Error::Abi("rejected log transaction"));
        }
        let level = match level {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            _ => return Err(Error::Abi("log level")),
        };
        let message = std::str::from_utf8(bytes).map_err(|_| Error::Abi("log UTF-8"))?;
        if message
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err(Error::Abi("log control character"));
        }
        if self.records.len() == MAX_TICK_ENTRIES || bytes.len() > MAX_TICK_BYTES - self.bytes {
            return Err(Error::Limit("log update buffer"));
        }
        self.bytes += bytes.len();
        self.records.push((level, message.to_owned()));
        Ok(())
    }
}
