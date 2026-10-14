//! A trusted host's one opened document. No serialized value grants a path.
use super::*;
use std::{path::PathBuf, sync::Arc};

pub const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
const MAX_PROJECT_MARKER_BYTES: usize = 128 * 1024 * 1024;
const MARKER: &str = "project.pending";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectEdit {
    pub project_id: String,
    pub before: String,
    pub after: String,
}
impl ProjectEdit {
    pub fn validate(&self) -> Result<(), SourceError> {
        ulid::Ulid::from_string(&self.project_id)
            .map_err(|_| SourceError::Invalid("invalid project ULID".into()))?;
        if self.before.len() > MAX_PROJECT_BYTES || self.after.len() > MAX_PROJECT_BYTES {
            return Err(SourceError::Invalid("project exceeds 32 MiB".into()));
        }
        Ok(())
    }
    pub fn reversed(&self) -> Self {
        Self {
            project_id: self.project_id.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub text: String,
    identity: (u64, u64),
    node: Arc<File>,
}
impl PartialEq for ProjectSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity && self.text == other.text
    }
}
impl Eq for ProjectSnapshot {}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectPending {
    version: u32,
    pub operation: String,
    binding: String,
    pub edit: ProjectEdit,
    original: (u64, u64),
    staged: (u64, u64),
    backup: (u64, u64),
    // Live observations pin nodes, preventing identity reuse within this process.
    // Persisted IDs remain correspondence checks, never independent authority.
    #[serde(skip)]
    held: Vec<Arc<File>>,
}
/// Not Clone, Serialize, Deserialize, or schema-addressable. Only the host can
/// construct this capability. Root and journal handles stay pinned for its life.
pub struct OpenedProject {
    root: Dir,
    leaf: String,
    journal_leaf: String,
    journal_path: PathBuf,
    journal_identity: (u64, u64),
    // The bus owns the locked writer; this duplicate also pins the actual node
    // if that writer is replaced while handling an ambiguous I/O failure.
    _journal_node: File,
    binding: Option<String>,
    expected: Option<Arc<File>>,
}
fn leaf(path: &Path) -> Result<String, SourceError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SourceError::Invalid("invalid host project filename".into()))?;
    validate_path(&format!("{name}.ts"))?;
    if name.ends_with(['.', ' ']) {
        return Err(SourceError::Invalid("unsafe filename".into()));
    }
    Ok(name.into())
}
fn read(dir: &Dir, name: &str) -> Result<Option<ProjectSnapshot>, SourceError> {
    let mut file = match dir.open(name) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    platform::regular(&file)?;
    let identity = platform::identity(&file)?;
    let before = file.metadata()?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_PROJECT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    platform::regular(&file)?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(SourceError::Conflict);
    }
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err(SourceError::Invalid("project exceeds 32 MiB".into()));
    }
    // Recheck the named node as well as the opened handle: a same-byte external
    // replacement must not inherit a review made against the retired inode.
    let named = dir.open(name)?;
    platform::regular(&named)?;
    if platform::identity(&named)? != identity {
        return Err(SourceError::Conflict);
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| SourceError::Invalid("project must be UTF-8".into()))?;
    Ok(Some(ProjectSnapshot {
        text,
        identity,
        node: Arc::new(file),
    }))
}
fn named(operation: &str, suffix: &str) -> Result<String, SourceError> {
    let id = ulid::Ulid::from_string(operation)
        .map_err(|_| SourceError::Invalid("invalid operation ULID".into()))?;
    if id.to_string() != operation {
        return Err(SourceError::Invalid("noncanonical operation ULID".into()));
    }
    Ok(format!("{operation}.project.{suffix}"))
}
fn matches(snapshot: &Option<ProjectSnapshot>, text: &str, identity: (u64, u64)) -> bool {
    snapshot
        .as_ref()
        .is_some_and(|s| s.text == text && s.identity == identity)
}
impl OpenedProject {
    /// Lock the derived journal before parsing the project (which may be absent
    /// after displacement). Neither project data nor command data supplies paths.
    pub fn lock(path: &Path) -> Result<(Self, File), SourceError> {
        if !SUPPORTED {
            return Err(SourceError::Unavailable);
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let root = Dir::root(parent)?;
        let document_leaf = leaf(path)?;
        let journal_path = path.with_extension("journal");
        let journal_leaf = leaf(&journal_path)?;
        if document_leaf.eq_ignore_ascii_case(&journal_leaf) {
            return Err(SourceError::Invalid("project aliases journal".into()));
        }
        let file = match root.create(&journal_leaf) {
            Ok(file) => {
                file.sync_all()?;
                root.sync()?;
                file
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => root.open_rw(&journal_leaf)?,
            Err(e) => return Err(e.into()),
        };
        platform::regular(&file)?;
        file.try_lock()
            .map_err(|e| SourceError::Io(io::Error::other(e)))?;
        let journal_identity = platform::identity(&file)?;
        let journal_path = std::fs::canonicalize(parent)?.join(&journal_leaf);
        Ok((
            Self {
                root,
                leaf: document_leaf,
                journal_leaf,
                journal_path,
                journal_identity,
                _journal_node: file.try_clone()?,
                binding: None,
                expected: None,
            },
            file,
        ))
    }
    pub fn source_store(&mut self, project_id: &str) -> Result<SourceStore, SourceError> {
        self.verify_journal()?;
        let store = SourceStore::open_at(self.root.duplicate()?, &self.journal_path, project_id)?;
        self.binding = Some(format!(
            "{:x}",
            Sha256::digest(format!(
                "{}\n{}\n{:?}",
                store.binding, self.leaf, self.journal_identity
            ))
        ));
        Ok(store)
    }
    fn verify_journal(&self) -> Result<(), SourceError> {
        let file = self.root.open(&self.journal_leaf)?;
        platform::regular(&file)?;
        if platform::identity(&file)? != self.journal_identity {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
    pub fn read(&self) -> Result<Option<ProjectSnapshot>, SourceError> {
        self.verify_journal()?;
        read(&self.root, &self.leaf)
    }
    pub fn admit_snapshot(&mut self, snapshot: &ProjectSnapshot) {
        self.expected = Some(Arc::clone(&snapshot.node));
    }
    pub fn admit_result(
        &mut self,
        pending: &ProjectPending,
        committed: bool,
    ) -> Result<(), SourceError> {
        let identity = if committed {
            pending.staged
        } else {
            pending.original
        };
        self.expected = Some(Arc::clone(
            pending
                .held
                .iter()
                .find(|file| platform::identity(file).ok() == Some(identity))
                .ok_or(SourceError::RecoveryRequired)?,
        ));
        Ok(())
    }
    pub fn read_admitted(&self) -> Result<ProjectSnapshot, SourceError> {
        let snapshot = self.read()?.ok_or(SourceError::Conflict)?;
        if self
            .expected
            .as_ref()
            .map(|file| platform::identity(file))
            .transpose()?
            != Some(snapshot.identity)
        {
            return Err(SourceError::Conflict);
        }
        Ok(snapshot)
    }
    pub fn pending(&self, store: &SourceStore) -> Result<Option<ProjectPending>, SourceError> {
        self.verify_journal()?;
        let Some(text) = optional_read(&store.state, MARKER, MAX_PROJECT_MARKER_BYTES)
            .map_err(|_| SourceError::RecoveryRequired)?
        else {
            return Ok(None);
        };
        let mut p: ProjectPending =
            serde_json::from_str(&text).map_err(|_| SourceError::RecoveryRequired)?;
        if p.version != 1
            || self.binding.as_ref() != Some(&p.binding)
            || p.edit.validate().is_err()
            || named(&p.operation, "backup").is_err()
            || store.pending()?.is_some()
        {
            return Err(SourceError::RecoveryRequired);
        }
        let backup = read(&store.state, &named(&p.operation, "backup")?)?;
        if !matches(&backup, &p.edit.before, p.backup) {
            return Err(SourceError::RecoveryRequired);
        }
        p.held.push(backup.expect("checked backup").node);
        for (dir, name) in [
            (&self.root, self.leaf.clone()),
            (&store.state, named(&p.operation, "previous")?),
            (&store.state, named(&p.operation, "next")?),
        ] {
            match dir.open(&name) {
                Ok(file) => {
                    platform::regular(&file)?;
                    p.held.push(Arc::new(file));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(Some(p))
    }
    fn verified(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
    ) -> Result<ProjectPending, SourceError> {
        self.pending(store)?
            .filter(|p| p.operation == op && p.edit == *edit)
            .ok_or(SourceError::RecoveryRequired)
    }
    pub fn begin(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
        reviewed: &ProjectSnapshot,
    ) -> Result<ProjectPending, SourceError> {
        store.ensure_ready()?;
        self.verify_journal()?;
        edit.validate()?;
        let backup = named(op, "backup")?;
        let next = named(op, "next")?;
        if reviewed.text != edit.before || self.read()?.as_ref() != Some(reviewed) {
            return Err(SourceError::Conflict);
        }
        write_new(&store.state, &backup, edit.before.as_bytes())?;
        project_crash("backup");
        write_new(&store.state, &next, edit.after.as_bytes())?;
        let staged = read(&store.state, &next)?.ok_or(SourceError::RecoveryRequired)?;
        let backup = read(&store.state, &backup)?.ok_or(SourceError::RecoveryRequired)?;
        let p = ProjectPending {
            version: 1,
            operation: op.into(),
            binding: self.binding.clone().ok_or(SourceError::Unavailable)?,
            edit: edit.clone(),
            original: reviewed.identity,
            staged: staged.identity,
            backup: backup.identity,
            held: vec![Arc::clone(&reviewed.node), staged.node, backup.node],
        };
        let encoded = bounded_json(&p, MAX_PROJECT_MARKER_BYTES)?;
        let marker = named(op, "marker")?;
        write_new(&store.state, &marker, &encoded)?;
        store.state.move_to(&marker, &store.state, MARKER)?;
        project_crash("marker");
        Ok(p)
    }
    pub fn displace(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
    ) -> Result<(), SourceError> {
        let p = self.verified(store, op, edit)?;
        if !matches(&self.read()?, &edit.before, p.original) {
            return Err(SourceError::Conflict);
        }
        self.root
            .move_to(&self.leaf, &store.state, &named(op, "previous")?)?;
        project_crash("displaced");
        if !matches(
            &read(&store.state, &named(op, "previous")?)?,
            &edit.before,
            p.original,
        ) {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
    pub fn publish(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
    ) -> Result<(), SourceError> {
        let p = self.verified(store, op, edit)?;
        if !matches(
            &read(&store.state, &named(op, "previous")?)?,
            &edit.before,
            p.original,
        ) {
            return Err(SourceError::RecoveryRequired);
        }
        store
            .state
            .move_to(&named(op, "next")?, &self.root, &self.leaf)?;
        project_crash("published");
        self.verify_result(store, &p, true)
    }
    pub fn verify_result(
        &self,
        _store: &SourceStore,
        p: &ProjectPending,
        committed: bool,
    ) -> Result<(), SourceError> {
        let (text, identity) = if committed {
            (&p.edit.after, p.staged)
        } else {
            (&p.edit.before, p.original)
        };
        if !matches(&self.read()?, text, identity) {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn check_rollback(
        &self,
        store: &SourceStore,
        p: &ProjectPending,
    ) -> Result<(), SourceError> {
        let current = self.read()?;
        if matches(&current, &p.edit.before, p.original) {
            return Ok(());
        }
        if !matches(
            &read(&store.state, &named(&p.operation, "previous")?)?,
            &p.edit.before,
            p.original,
        ) || (current.is_some() && !matches(&current, &p.edit.after, p.staged))
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn rollback(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
    ) -> Result<(), SourceError> {
        let p = self.verified(store, op, edit)?;
        let current = self.read()?;
        if matches(&current, &edit.before, p.original) {
            return Ok(());
        }
        let previous = named(op, "previous")?;
        if !matches(&read(&store.state, &previous)?, &edit.before, p.original)
            || (current.is_some() && !matches(&current, &edit.after, p.staged))
        {
            return Err(SourceError::RecoveryRequired);
        }
        if current.is_some() {
            let rejected = named(op, "rejected")?;
            self.root.move_to(&self.leaf, &store.state, &rejected)?;
            if !matches(&read(&store.state, &rejected)?, &edit.after, p.staged) {
                return Err(SourceError::RecoveryRequired);
            }
        }
        store.state.move_to(&previous, &self.root, &self.leaf)?;
        self.verify_result(store, &p, false)
    }
    pub fn finish(
        &self,
        store: &SourceStore,
        op: &str,
        edit: &ProjectEdit,
        committed: bool,
    ) -> Result<(), SourceError> {
        let p = self.verified(store, op, edit)?;
        self.verify_result(store, &p, committed)?;
        store.publish_receipt(op)?;
        store.state.remove(MARKER)?;
        project_crash("finish");
        Ok(())
    }
}
/// Bounded encoded output, including JSON escaping. Fails before growing beyond
/// the protocol limit; a small input of control characters cannot bypass it.
pub fn bounded_json(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, SourceError> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if b.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(io::Error::other("encoded protocol limit"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut out, value).map_err(|e| SourceError::Invalid(e.to_string()))?;
    Ok(out.bytes)
}
fn project_crash(_phase: &str) {
    #[cfg(test)]
    if std::env::var("INCANT_PROJECT_CRASH_PHASE").as_deref() == Ok(_phase) {
        std::process::exit(73);
    }
}

#[cfg(test)]
#[path = "project_tests.rs"]
mod tests;
