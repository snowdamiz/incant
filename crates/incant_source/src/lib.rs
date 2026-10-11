//! Authoring-only confined source storage. Project consumers use `check_recovery`
//! before and after acquiring a source graph and compare the returned epoch.
//! Mutating protocol primitives are for incant_cmd, the sole authoring gateway.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
#[path = "unsupported.rs"]
mod platform;
use platform::Dir;
const SUPPORTED: bool = cfg!(any(target_os = "linux", target_os = "macos", windows));
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
use thiserror::Error;

pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
const STATE: &str = ".incant-source-transactions";
const MAX_MARKER_BYTES: usize = 12 * MAX_SOURCE_BYTES + 4096;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error(
        "script source capability is unavailable (a trusted project root and persistent journal are required)"
    )]
    Unavailable,
    #[error("invalid script source: {0}")]
    Invalid(String),
    #[error("script source changed; read it again before editing")]
    Conflict,
    #[error(
        "script source recovery required; preserved transaction files must be reconciled before continuing"
    )]
    RecoveryRequired,
    #[error("script source I/O: {0}")]
    Io(#[from] io::Error),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceVersion {
    Missing,
    Present { sha256: String },
}
impl SourceVersion {
    pub fn of(source: Option<&str>) -> Self {
        source.map_or(Self::Missing, |source| Self::Present {
            sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        })
    }
}
/// Internal, exact preimage and postimage, populated by the bus after validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceEdit {
    pub script_id: String,
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
impl SourceEdit {
    pub fn reversed(&self) -> Self {
        Self {
            script_id: self.script_id.clone(),
            path: self.path.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }
    pub fn validate(&self) -> Result<(), SourceError> {
        ulid::Ulid::from_string(&self.script_id)
            .map_err(|_| SourceError::Invalid("invalid script ULID".into()))?;
        validate_path(&self.path)?;
        for source in [&self.before, &self.after].into_iter().flatten() {
            validate_source(source)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub operation: String,
    pub binding: String,
    pub edit: SourceEdit,
}
/// A read-only receipt. It detects completed engine writes across graph capture;
/// it does not lock, control, or claim a CAS against external editors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEpoch(String);

pub fn validate_source(source: &str) -> Result<(), SourceError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(SourceError::Invalid("source exceeds 256 KiB".into()));
    }
    Ok(())
}
pub fn validate_path(path: &str) -> Result<(), SourceError> {
    if path.len() > 1024
        || !path.ends_with(".ts")
        || path.ends_with(".d.ts")
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
    {
        return Err(SourceError::Invalid(
            "expected a portable project-relative .ts path".into(),
        ));
    }
    for part in path.split('/') {
        // Portable spelling prevents NTFS alternate streams, device aliases,
        // case-folded engine state names, and Win32 trailing-dot/space aliases.
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.starts_with('.')
            || part.ends_with(['.', ' '])
            || part.contains(['<', '>', '"', '|', '?', '*'])
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err(SourceError::Invalid(
                "source path has a reserved or unsafe component".into(),
            ));
        }
    }
    Ok(())
}
fn optional_read(dir: &Dir, name: &str, limit: usize) -> Result<Option<String>, SourceError> {
    let mut file = match dir.open(name) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    platform::regular(&file)?;
    let before = file.metadata()?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(SourceError::Conflict);
    }
    platform::regular(&file)?;
    if bytes.len() > limit {
        return Err(SourceError::Invalid(
            "source or transaction record exceeds its byte limit".into(),
        ));
    }
    Ok(Some(String::from_utf8(bytes).map_err(|_| {
        SourceError::Invalid("source must be UTF-8".into())
    })?))
}
fn write_new(dir: &Dir, name: &str, bytes: &[u8]) -> Result<(), SourceError> {
    let mut file = dir.create(name)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    dir.sync()?;
    Ok(())
}
fn state(root: &Dir) -> Result<Option<Dir>, SourceError> {
    match root.child(STATE) {
        Ok(dir) => Ok(Some(dir)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(SourceError::RecoveryRequired),
    }
}
/// No writes, no journal replay, no file recovery. A pending or malformed marker
/// fails closed. Compare the result before/after source graph acquisition.
pub fn check_recovery(root: &Path) -> Result<SourceEpoch, SourceError> {
    if !SUPPORTED {
        return Err(SourceError::Unavailable);
    }
    let root = Dir::root(root)?;
    let Some(state) = state(&root)? else {
        return Ok(SourceEpoch(String::new()));
    };
    let check = || match optional_read(&state, "pending", MAX_MARKER_BYTES) {
        Ok(None) => Ok(()),
        _ => Err(SourceError::RecoveryRequired),
    };
    check()?;
    let receipt = optional_read(&state, "receipt", 128)
        .map_err(|_| SourceError::RecoveryRequired)?
        .unwrap_or_default();
    if !receipt.is_empty() && ulid::Ulid::from_string(&receipt).is_err() {
        return Err(SourceError::RecoveryRequired);
    }
    check()?;
    Ok(SourceEpoch(receipt))
}

pub struct SourceStore {
    root: Dir,
    state: Dir,
    _lock: File,
    binding: String,
}
impl SourceStore {
    /// Trusted host construction only. Tool arguments never supply either path.
    /// One lock per root prevents distinct journals from concurrently writing it.
    pub fn open(root: &Path, journal: &Path, project_id: &str) -> Result<Self, SourceError> {
        if !SUPPORTED {
            return Err(SourceError::Unavailable);
        }
        let root = Dir::root(root)?;
        let state = match state(&root)? {
            Some(dir) => dir,
            None => {
                root.mkdir(STATE)?;
                root.child(STATE)?
            }
        };
        let lock = match state.create("lock") {
            Ok(lock) => lock,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => state.open("lock")?,
            Err(e) => return Err(e.into()),
        };
        platform::regular(&lock)?;
        lock.try_lock()
            .map_err(|e| SourceError::Io(io::Error::other(e)))?;
        let journal = std::fs::canonicalize(journal)?;
        let binding = format!(
            "{:x}",
            Sha256::digest(format!("{project_id}\n{}", journal.to_string_lossy()))
        );
        Ok(Self {
            root,
            state,
            _lock: lock,
            binding,
        })
    }
    fn parent(&self, path: &str) -> Result<(Dir, String), SourceError> {
        validate_path(path)?;
        let mut parts = path.split('/').peekable();
        let mut parent = self.root.duplicate()?;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                return Ok((parent, part.into()));
            }
            parent = parent.child(part)?;
        }
        Err(SourceError::Invalid("empty source path".into()))
    }
    pub fn read(&self, path: &str) -> Result<Option<String>, SourceError> {
        let (parent, leaf) = self.parent(path)?;
        optional_read(&parent, &leaf, MAX_SOURCE_BYTES)
    }
    pub fn same_file(&self, left: &str, right: &str) -> Result<bool, SourceError> {
        let open = |path| -> Result<Option<File>, SourceError> {
            let (parent, leaf) = self.parent(path)?;
            match parent.open(&leaf) {
                Ok(file) => {
                    platform::regular(&file)?;
                    Ok(Some(file))
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        };
        match (open(left)?, open(right)?) {
            (Some(left), Some(right)) => {
                Ok(platform::identity(&left)? == platform::identity(&right)?)
            }
            _ => Ok(false),
        }
    }
    pub fn pending(&self) -> Result<Option<Pending>, SourceError> {
        let Some(bytes) = optional_read(&self.state, "pending", MAX_MARKER_BYTES)
            .map_err(|_| SourceError::RecoveryRequired)?
        else {
            return Ok(None);
        };
        let pending: Pending =
            serde_json::from_str(&bytes).map_err(|_| SourceError::RecoveryRequired)?;
        if pending.binding != self.binding
            || ulid::Ulid::from_string(&pending.operation).is_err()
            || pending.edit.validate().is_err()
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(Some(pending))
    }
    pub fn ensure_ready(&self) -> Result<(), SourceError> {
        if self.pending()?.is_some() {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    /// Stage + durable marker, before journal Prepare. A crash before the marker
    /// leaves only an unreferenced uniquely named stage, never a source mutation.
    pub fn begin(&self, operation: &str, edit: &SourceEdit) -> Result<(), SourceError> {
        self.ensure_ready()?;
        edit.validate()?;
        ulid::Ulid::from_string(operation)
            .map_err(|_| SourceError::Invalid("invalid operation ULID".into()))?;
        if self.read(&edit.path)? != edit.before {
            return Err(SourceError::Conflict);
        }
        if let Some(after) = &edit.after {
            write_new(&self.state, &format!("{operation}.next"), after.as_bytes())?;
        }
        #[cfg(test)]
        crash_point("staged");
        let pending = Pending {
            operation: operation.into(),
            binding: self.binding.clone(),
            edit: edit.clone(),
        };
        let encoded =
            serde_json::to_vec(&pending).map_err(|e| SourceError::Invalid(e.to_string()))?;
        write_new(&self.state, &format!("{operation}.marker"), &encoded)?;
        self.state
            .move_to(&format!("{operation}.marker"), &self.state, "pending")?;
        Ok(())
    }
    fn verify_pending(&self, operation: &str, edit: &SourceEdit) -> Result<(), SourceError> {
        if self
            .pending()?
            .is_none_or(|p| p.operation != operation || p.edit != *edit)
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    /// No clobbering rename. Preserve the actual displaced node before checking
    /// it, since arbitrary external editors do not participate in our root lock.
    pub fn install(&self, operation: &str, edit: &SourceEdit) -> Result<(), SourceError> {
        self.verify_pending(operation, edit)?;
        let (parent, leaf) = self.parent(&edit.path)?;
        if edit.before.is_some() {
            parent.move_to(&leaf, &self.state, &format!("{operation}.previous"))?;
            #[cfg(test)]
            crash_point("displaced");
            if optional_read(
                &self.state,
                &format!("{operation}.previous"),
                MAX_SOURCE_BYTES,
            )? != edit.before
            {
                return Err(SourceError::Conflict);
            }
        }
        if edit.after.is_some() {
            self.state
                .move_to(&format!("{operation}.next"), &parent, &leaf)?;
        }
        if self.read(&edit.path)? != edit.after {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
    /// Reconcile an uncommitted operation. Foreign bytes/nodes are preserved and
    /// leave the durable pending marker in place for explicit human reconciliation.
    pub fn rollback(&self, operation: &str, edit: &SourceEdit) -> Result<(), SourceError> {
        self.verify_pending(operation, edit)?;
        let current = self
            .read(&edit.path)
            .map_err(|_| SourceError::RecoveryRequired)?;
        let previous = optional_read(
            &self.state,
            &format!("{operation}.previous"),
            MAX_SOURCE_BYTES,
        )
        .map_err(|_| SourceError::RecoveryRequired)?;
        if previous.is_some() && previous != edit.before {
            return Err(SourceError::RecoveryRequired);
        }
        if current == edit.before {
            return Ok(());
        }
        if previous != edit.before || (current.is_some() && current != edit.after) {
            return Err(SourceError::RecoveryRequired);
        }
        let (parent, leaf) = self.parent(&edit.path)?;
        if current.is_some() {
            parent
                .move_to(&leaf, &self.state, &format!("{operation}.rejected"))
                .map_err(|_| SourceError::RecoveryRequired)?;
            if optional_read(
                &self.state,
                &format!("{operation}.rejected"),
                MAX_SOURCE_BYTES,
            )
            .map_err(|_| SourceError::RecoveryRequired)?
                != edit.after
            {
                return Err(SourceError::RecoveryRequired);
            }
        }
        if edit.before.is_some() {
            self.state
                .move_to(&format!("{operation}.previous"), &parent, &leaf)
                .map_err(|_| SourceError::RecoveryRequired)?;
        }
        if self
            .read(&edit.path)
            .map_err(|_| SourceError::RecoveryRequired)?
            != edit.before
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    /// Call only after a durable journal Commit/Abort, including recovery. The
    /// new receipt makes completed operations observable to standalone readers.
    pub fn finish(&self, operation: &str) -> Result<(), SourceError> {
        if self.pending()?.is_none_or(|p| p.operation != operation) {
            return Err(SourceError::RecoveryRequired);
        }
        let next = format!("{operation}.receipt");
        // Idempotent after crashes anywhere during receipt publication.
        if optional_read(&self.state, "receipt", 128)? != Some(operation.into()) {
            if optional_read(&self.state, &next, 128)?.is_none() {
                write_new(&self.state, &next, operation.as_bytes())?;
            }
            if optional_read(&self.state, "receipt", 128)?.is_some() {
                self.state
                    .move_to("receipt", &self.state, &format!("{operation}.old-receipt"))?;
            }
            #[cfg(test)]
            crash_point("receipt_retired");
            self.state.move_to(&next, &self.state, "receipt")?;
            #[cfg(test)]
            crash_point("receipt_published");
        }
        self.state.remove("pending")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
fn crash_point(phase: &str) {
    if std::env::var("INCANT_SOURCE_CRASH_TEST_PHASE").as_deref() == Ok(phase) {
        std::process::exit(73);
    }
}
