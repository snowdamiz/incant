//! Copied accepted-prefix transport. The live VM never crosses the GUI boundary.
use incant_runtime::logging::{
    LogLevel, MAX_LOG_TICK, MAX_MESSAGE_BYTES, MAX_RETAINED_BYTES, MAX_RETAINED_ENTRIES, NativeLogs,
};
use serde::Serialize;

pub const MAX_BATCH_ENTRIES: usize = 64;
pub const MAX_BATCH_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Default, Serialize)]
pub struct Summary {
    pub count: usize,
    pub dropped: u64,
}
#[derive(Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub tick: u64,
    pub level: LogLevel,
    pub message: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub generation: u32,
    pub from: usize,
    pub next: usize,
    pub dropped: u64,
    /// One stable Console event for aggregate truncation, including UI retention.
    pub warning_id: String,
    pub entries: Vec<Entry>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Play log prefix changed")]
    PrefixChanged,
    #[error("Play log limit exceeded")]
    LimitExceeded,
    #[error("Play log cursor is invalid")]
    CursorInvalid,
}
pub struct Store {
    entries: Vec<Entry>,
    bytes: usize,
    dropped: u64,
    warning_id: String,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            bytes: 0,
            dropped: 0,
            warning_id: incant_doc::new_id(),
        }
    }
}
impl Store {
    pub fn reset(&mut self) {
        self.entries.clear();
        self.bytes = 0;
        self.dropped = 0;
    }
    pub fn summary(&self) -> Summary {
        Summary {
            count: self.entries.len(),
            dropped: self.dropped,
        }
    }
    pub fn append(&mut self, logs: &NativeLogs) -> Result<(), Error> {
        let entries = logs.entries();
        let dropped = logs.dropped().min(MAX_LOG_TICK);
        if entries.len() < self.entries.len()
            || entries.len() > MAX_RETAINED_ENTRIES
            || dropped < self.dropped
        {
            return Err(Error::PrefixChanged);
        }
        // Inspect/copy only newly accepted rows. Repeated pause/redraw does no history walk.
        let tail = &entries[self.entries.len()..];
        let bytes = tail
            .iter()
            .try_fold(self.bytes, |bytes, row| {
                if row.tick > MAX_LOG_TICK || row.message.len() > MAX_MESSAGE_BYTES {
                    return None;
                }
                bytes
                    .checked_add(row.message.len())
                    .filter(|n| *n <= MAX_RETAINED_BYTES)
            })
            .ok_or(Error::LimitExceeded)?;
        self.entries.extend(tail.iter().map(|row| Entry {
            id: incant_doc::new_id(),
            tick: row.tick,
            level: row.level,
            message: row.message.clone(),
        }));
        self.bytes = bytes;
        self.dropped = dropped;
        Ok(())
    }
    pub fn read(&self, generation: u32, from: usize) -> Result<Batch, Error> {
        if from > self.entries.len() {
            return Err(Error::CursorInvalid);
        }
        let mut bytes = 0;
        let entries: Vec<_> = self.entries[from..]
            .iter()
            .take(MAX_BATCH_ENTRIES)
            .take_while(|entry| {
                bytes += entry.message.len();
                bytes <= MAX_BATCH_BYTES
            })
            .cloned()
            .collect();
        Ok(Batch {
            generation,
            from,
            next: from + entries.len(),
            dropped: self.dropped,
            warning_id: self.warning_id.clone(),
            entries,
        })
    }
}
