//! Local, per-project usage accounting, separate from undoable project content.
//!
//! A lease holds an OS lock across a turn. Before any provider work, its entire
//! allowance is durably reserved. A normal completion replaces the reservation
//! with charged usage; interruption without a receipt leaves a conservative,
//! explicitly uncertain charge. Changing accounts or builds does not reset it.
#[cfg(test)]
mod tests;
use crate::Budget;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const MAX_TOKEN_CAP: u64 = 9_007_199_254_740_991;
const MAX_RECORD_BYTES: u64 = 4096;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("invalid project identity or token cap")]
    InvalidRequest,
    #[error("another Incant process owns this project's agent budget")]
    Busy,
    #[error("project agent token allowance is exhausted")]
    Exhausted,
    #[error("saved agent usage is invalid; it has not been overwritten")]
    InvalidRecord,
    #[error("agent usage storage is unavailable")]
    Storage,
    #[error("agent usage receipt is inconsistent; the reservation remains charged")]
    InvalidReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    initial_tokens: u64,
    max_tokens: u64,
    prior_uncertain: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    project_id: String,
    charged_tokens: u64,
    accounting_uncertain: bool,
    pending: Option<Pending>,
}

#[derive(Debug)]
pub struct LedgerSnapshot {
    pub budget: Budget,
    pub accounting_uncertain: bool,
    /// An unfinished reservation from an earlier owner, not a live worker.
    pub recovered_reservation: bool,
}

/// Open only on a worker: filesystem operations never belong on the GUI thread.
pub struct Ledger {
    path: PathBuf,
    record: Record,
    _lock: OwnedLock,
}
// A fork may transiently inherit the open file description before exec closes
// its descriptor. Closing only our File would leave the flock held by that
// unrelated alias. Release ownership explicitly while this descriptor is valid.
struct OwnedLock(File);
impl Drop for OwnedLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub struct Lease {
    ledger: Ledger,
    pending: Pending,
}

fn valid_id(id: &str) -> bool {
    id.len() == 26
        && id.as_bytes()[0] <= b'7'
        && id
            .bytes()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c))
}
fn cap(max_tokens: u64) -> Result<(), LedgerError> {
    if max_tokens <= MAX_TOKEN_CAP {
        Ok(())
    } else {
        Err(LedgerError::InvalidRequest)
    }
}
fn regular_or_absent(path: &Path) -> Result<(), LedgerError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(LedgerError::Storage),
    }
}
impl Ledger {
    pub fn open(project_id: &str) -> Result<Self, LedgerError> {
        let directory = dirs::config_dir()
            .ok_or(LedgerError::Storage)?
            .join("Incant")
            .join("agent-usage");
        Self::open_at(&directory, project_id)
    }
    /// Use a trusted host-selected state directory, never an agent/tool or
    /// project-supplied path. The default application entry point is `open`.
    /// This also supports isolated host integration tests without changing the
    /// process-wide configuration environment. Call only on a worker.
    pub fn open_in(directory: &Path, project_id: &str) -> Result<Self, LedgerError> {
        Self::open_at(directory, project_id)
    }

    fn open_at(directory: &Path, project_id: &str) -> Result<Self, LedgerError> {
        if !valid_id(project_id) {
            return Err(LedgerError::InvalidRequest);
        }
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(directory)
            .map_err(|_| LedgerError::Storage)?;
        let meta = fs::symlink_metadata(directory).map_err(|_| LedgerError::Storage)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(LedgerError::Storage);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| LedgerError::Storage)?;
        }
        let path = directory.join(format!("{project_id}.json"));
        let lock_path = directory.join(format!("{project_id}.lock"));
        regular_or_absent(&lock_path)?;
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(lock_path).map_err(|_| LedgerError::Storage)?;
        lock.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => LedgerError::Busy,
            _ => LedgerError::Storage,
        })?;
        // Guard every subsequent refusal too, before constructing a Ledger.
        let lock = OwnedLock(lock);
        regular_or_absent(&path)?;
        let record = match File::open(&path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(MAX_RECORD_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| LedgerError::Storage)?;
                if bytes.len() as u64 > MAX_RECORD_BYTES {
                    return Err(LedgerError::InvalidRecord);
                }
                let value: Record =
                    serde_json::from_slice(&bytes).map_err(|_| LedgerError::InvalidRecord)?;
                if value.version != 1
                    || value.project_id != project_id
                    || value.charged_tokens > MAX_TOKEN_CAP
                    || value.pending.as_ref().is_some_and(|pending| {
                        pending.initial_tokens >= pending.max_tokens
                            || pending.max_tokens != value.charged_tokens
                            || !value.accounting_uncertain
                    })
                {
                    return Err(LedgerError::InvalidRecord);
                }
                value
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Record {
                version: 1,
                project_id: project_id.to_owned(),
                charged_tokens: 0,
                accounting_uncertain: false,
                pending: None,
            },
            Err(_) => return Err(LedgerError::Storage),
        };
        Ok(Self {
            path,
            record,
            _lock: lock,
        })
    }
    pub fn snapshot(&self, max_tokens: u64) -> Result<LedgerSnapshot, LedgerError> {
        cap(max_tokens)?;
        Ok(LedgerSnapshot {
            budget: Budget {
                max_tokens,
                used_tokens: self.record.charged_tokens,
            },
            accounting_uncertain: self.record.accounting_uncertain,
            recovered_reservation: self.record.pending.is_some(),
        })
    }
    pub fn reserve(mut self, max_tokens: u64) -> Result<Lease, LedgerError> {
        let snapshot = self.snapshot(max_tokens)?;
        if snapshot.budget.remaining() == 0 {
            return Err(LedgerError::Exhausted);
        }
        let pending = Pending {
            initial_tokens: self.record.charged_tokens,
            max_tokens,
            prior_uncertain: self.record.accounting_uncertain,
        };
        self.record.charged_tokens = max_tokens;
        self.record.accounting_uncertain = true;
        self.record.pending = Some(pending.clone());
        self.save()?;
        Ok(Lease {
            ledger: self,
            pending,
        })
    }
    fn save(&self) -> Result<(), LedgerError> {
        let parent = self.path.parent().ok_or(LedgerError::Storage)?;
        let bytes = serde_json::to_vec(&self.record).map_err(|_| LedgerError::Storage)?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| LedgerError::Storage)?;
        file.write_all(&bytes)
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| LedgerError::Storage)?;
        file.persist(&self.path).map_err(|_| LedgerError::Storage)?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| LedgerError::Storage)?;
        Ok(())
    }
}
impl Lease {
    pub fn budget(&self) -> Budget {
        Budget {
            max_tokens: self.pending.max_tokens,
            used_tokens: self.pending.initial_tokens,
        }
    }
    /// Receipt is cumulative, as in `Agent::budget`. Never infer zero usage
    /// from cancellation or an error. Drop without a trustworthy receipt instead.
    pub fn complete(
        mut self,
        budget: &Budget,
        accounting_uncertain: bool,
    ) -> Result<LedgerSnapshot, LedgerError> {
        if budget.max_tokens != self.pending.max_tokens
            || budget.used_tokens < self.pending.initial_tokens
        {
            return Err(LedgerError::InvalidReceipt);
        }
        // Over-limit measured usage remains charged. Saturation exhausts every
        // representable cap rather than making an unsafe JS integer reusable.
        self.ledger.record.charged_tokens = budget.used_tokens.min(MAX_TOKEN_CAP);
        self.ledger.record.accounting_uncertain = accounting_uncertain
            || self.pending.prior_uncertain
            || budget.used_tokens > MAX_TOKEN_CAP;
        self.ledger.record.pending = None;
        self.ledger.save()?;
        self.ledger.snapshot(budget.max_tokens)
    }
}
