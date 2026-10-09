//! Structured simulation output with bounded memory and an optional new JSONL file.
use crate::play::PlayError;
use incant_script::{LogLevel, ScriptLog};
use serde::Serialize;
use std::{fs, io::Write, path::Path};

const MAX_LOG_ENTRIES: usize = 10_000;
const MAX_LOG_BYTES: usize = 8 * 1024 * 1024;

#[derive(Serialize)]
pub struct Entry {
    tick: u64,
    elapsed_seconds: f64,
    level: LogLevel,
    message: String,
}

pub struct Capture {
    file: Option<fs::File>,
    entries: Vec<Entry>,
    bytes: usize,
}
impl Capture {
    pub fn new(path: Option<&Path>, frame_output: Option<&Path>) -> Result<Self, PlayError> {
        let file = path
            .map(|path| -> Result<fs::File, PlayError> {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    fs::create_dir_all(parent)?;
                }
                if let Some(output) = frame_output {
                    let parent = path
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new("."));
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    if parent.canonicalize()? == output.canonicalize()?
                        && (name == "report.json"
                            || (name.starts_with("frame-") && name.ends_with(".png")))
                    {
                        return Err(PlayError::LogOutputConflict);
                    }
                }
                Ok(fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?)
            })
            .transpose()?;
        Ok(Self {
            file,
            entries: Vec::new(),
            bytes: 0,
        })
    }
    pub fn append(
        &mut self,
        tick: u64,
        elapsed_seconds: f64,
        logs: Vec<ScriptLog>,
    ) -> Result<(), PlayError> {
        if self.entries.len() + logs.len() > MAX_LOG_ENTRIES {
            return Err(PlayError::LogBudget);
        }
        let entries: Vec<_> = logs
            .into_iter()
            .map(|log| Entry {
                tick,
                elapsed_seconds,
                level: log.level,
                message: log.message,
            })
            .collect();
        // Validate a whole tick before publishing any of its records.
        let mut bytes = Vec::new();
        for entry in &entries {
            serde_json::to_writer(&mut bytes, entry)?;
            bytes.push(b'\n');
        }
        if self.bytes + bytes.len() > MAX_LOG_BYTES {
            return Err(PlayError::LogBudget);
        }
        if let Some(file) = &mut self.file {
            file.write_all(&bytes)?;
        }
        self.bytes += bytes.len();
        self.entries.extend(entries);
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<Entry>, PlayError> {
        if let Some(file) = self.file {
            file.sync_all()?;
        }
        Ok(self.entries)
    }
}
