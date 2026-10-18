//! Authoring-only build jobs. Private candidates never become project assets.
mod types;
pub use types::*;
pub mod backend;
#[cfg(target_os = "macos")]
mod process;
#[cfg(any(target_os = "macos", test))]
mod recipe;
#[cfg(test)]
mod tests;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("module builds unavailable: {0}")]
    Unavailable(String),
    #[error("invalid module request: {0}")]
    Invalid(&'static str),
    #[error("module build busy")]
    Busy,
    #[error("module job cancelled")]
    Cancelled,
    #[error("module job aggregate deadline")]
    Deadline,
    #[error("module source/project changed; inspect again")]
    Stale,
    #[error("module worker failed (untrusted diagnostics): {0}")]
    Worker(String),
    #[error("module worker cleanup unproven: {0}")]
    Cleanup(String),
    #[error("module job IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("module job protocol: {0}")]
    Protocol(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
/// Callback must check project generation/revision and source recovery. No bus
/// guard may be retained while a backend runs.
pub type Validate<'a> = dyn FnMut() -> Result<()> + 'a;
pub trait Backend: Send + Sync {
    fn inspect(
        &self,
        root: &Path,
        package: &str,
        entry: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Result<SourcePackage>;
    fn build(&self, source: &SourcePackage, cancel: &dyn Fn() -> bool) -> Result<backend::Built>;
}
struct Snapshot {
    id: String,
    revision: u64,
    package: String,
    source: SourcePackage,
    digest: String,
    epoch: incant_source::SourceEpoch,
}
struct Binding {
    project: String,
    generation: String,
    root: PathBuf,
    node: std::fs::File,
}
#[derive(Default)]
struct State {
    binding: Option<Binding>,
    snapshot: Option<Snapshot>,
    candidate: Option<backend::Built>,
    status: Option<JobStatus>,
    poisoned: bool,
    active: Option<Arc<AtomicBool>>,
}
/// One instance is shared by editor IPC and its agent host, not recreated per
/// request. bind() invalidates same-ID reopened projects and cancels old jobs.
pub struct Service {
    backend: Option<Arc<dyn Backend>>,
    reason: String,
    state: Mutex<State>,
}
impl Service {
    pub fn new(backend: Arc<dyn Backend>) -> Self {
        Self {
            backend: Some(backend),
            reason: String::new(),
            state: Mutex::new(State::default()),
        }
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            backend: None,
            reason: reason.into(),
            state: Mutex::new(State::default()),
        }
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>> {
        self.state
            .lock()
            .map_err(|_| Error::Cleanup("job state poisoned".into()))
    }
    pub fn available(&self) -> bool {
        self.backend.is_some()
            && self
                .state
                .lock()
                .is_ok_and(|s| s.binding.is_some() && !s.poisoned)
    }
    pub fn bound_to(&self, project: &str, root: &Path) -> bool {
        self.state.lock().is_ok_and(|s| {
            s.binding.as_ref().is_some_and(|b| {
                b.project == project
                    && b.root == root
                    && b.node
                        .metadata()
                        .ok()
                        .zip(std::fs::metadata(root).ok())
                        .is_some_and(|(a, b)| same_node(&a, &b))
            })
        })
    }
    pub fn bind(&self, project: &str, root: &Path) -> Result<()> {
        ulid::Ulid::from_string(project).map_err(|_| Error::Invalid("project ULID"))?;
        let root = root.canonicalize()?;
        let node = std::fs::File::open(&root)?;
        if !node.metadata()?.is_dir() {
            return Err(Error::Invalid("project root"));
        }
        let mut state = self.lock()?;
        if let Some(cancel) = &state.active {
            cancel.store(true, Ordering::Release);
        }
        state.binding = Some(Binding {
            project: project.into(),
            generation: ulid::Ulid::new().to_string(),
            root,
            node,
        });
        state.snapshot = None;
        state.candidate = None;
        state.status = None;
        Ok(())
    }
    pub fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            if let Some(c) = &s.active {
                c.store(true, Ordering::Release);
            }
            s.binding = None;
            s.snapshot = None;
            s.candidate = None;
            s.status = None;
        }
    }
    pub fn status(&self, project: &str, query: &JobQuery) -> Result<JobStatus> {
        let state = self.lock()?;
        if state.binding.as_ref().is_none_or(|b| b.project != project) {
            return Err(Error::Stale);
        }
        state
            .status
            .as_ref()
            .filter(|s| s.snapshot_id == query.snapshot_id)
            .cloned()
            .ok_or(Error::Stale)
    }
    pub fn cancel_build(&self, project: &str, snapshot: &str) -> Result<()> {
        let state = self.lock()?;
        if state.binding.as_ref().is_none_or(|b| b.project != project)
            || state.snapshot.as_ref().is_none_or(|s| s.id != snapshot)
        {
            return Err(Error::Stale);
        }
        if let Some(cancel) = &state.active {
            cancel.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn begin(&self, project: &str) -> Result<(Lease<'_>, PathBuf)> {
        let backend = self
            .backend
            .clone()
            .ok_or_else(|| Error::Unavailable(self.reason.clone()))?;
        let mut s = self.lock()?;
        if s.poisoned {
            return Err(Error::Cleanup("previous job closure unproven".into()));
        }
        if s.active.is_some() {
            return Err(Error::Busy);
        }
        let b = s
            .binding
            .as_ref()
            .ok_or_else(|| Error::Unavailable("no saved project bound".into()))?;
        if b.project != project {
            return Err(Error::Stale);
        }
        // A held root node pins identity; a path replacement is never adopted.
        if !same_node(&b.node.metadata()?, &std::fs::metadata(&b.root)?) {
            return Err(Error::Stale);
        }
        let root = b.root.clone();
        let generation = b.generation.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        s.active = Some(cancel.clone());
        Ok((
            Lease {
                service: self,
                cancel,
                backend,
                generation,
            },
            root,
        ))
    }
    pub fn inspect(
        &self,
        project: &str,
        query: SourceQuery,
        cancel: &AtomicBool,
        validate: &mut Validate<'_>,
    ) -> Result<SourceView> {
        types::relative(&query.package, false)?;
        types::relative(&query.entry, true)?;
        validate()?;
        let (lease, root) = self.begin(project)?;
        let start = Instant::now();
        let stopped = || {
            cancel.load(Ordering::Acquire)
                || lease.cancel.load(Ordering::Acquire)
                || start.elapsed() >= Duration::from_secs(180)
        };
        if stopped() {
            return Err(Error::Cancelled);
        }
        let epoch = incant_source::check_recovery(&root).map_err(|_| Error::Stale)?;
        let source = lease.bounded(
            lease
                .backend
                .inspect(&root, &query.package, &query.entry, &stopped),
            start,
            cancel,
        )?;
        let digest = source.digest()?;
        lease.bounded(validate(), start, cancel)?;
        lease.bounded(Ok(()), start, cancel)?;
        if incant_source::check_recovery(&root).map_err(|_| Error::Stale)? != epoch {
            return Err(Error::Stale);
        }
        let id = ulid::Ulid::new().to_string();
        let mut state = lease.publish()?;
        state.snapshot = Some(Snapshot {
            id: id.clone(),
            revision: query.expected_revision,
            package: query.package,
            source: source.clone(),
            digest: digest.clone(),
            epoch,
        });
        state.candidate = None;
        state.status = Some(JobStatus {
            snapshot_id: id.clone(),
            build_id: None,
            state: "ready".into(),
            candidate_id: None,
        });
        Ok(SourceView {
            snapshot_id: id,
            revision: query.expected_revision,
            source_digest: digest,
            untrusted_source_data: source,
        })
    }
    pub fn build(
        &self,
        project: &str,
        request: BuildRequest,
        actor: Actor,
        cancel: &AtomicBool,
        validate: &mut Validate<'_>,
    ) -> Result<BuildResult> {
        if request.description.trim().is_empty() || request.description.len() > 1024 {
            return Err(Error::Invalid("build description"));
        }
        if !matches!(
            actor.origin.as_str(),
            "user" | "agent" | "import" | "script"
        ) || actor.actor.is_empty()
            || actor.actor.len() > 256
            || actor.model.as_ref().is_some_and(|s| s.len() > 256)
            || actor
                .conversation_id
                .as_ref()
                .is_some_and(|s| s.len() > 256)
        {
            return Err(Error::Invalid("engine actor bounds"));
        }
        validate()?;
        let (lease, root) = self.begin(project)?;
        let (package, source, digest, epoch) = {
            let s = self.lock()?;
            let old = s.snapshot.as_ref().ok_or(Error::Stale)?;
            if old.id != request.snapshot_id || old.revision != request.expected_revision {
                return Err(Error::Stale);
            }
            (
                old.package.clone(),
                old.source.clone(),
                old.digest.clone(),
                old.epoch.clone(),
            )
        };
        let start = Instant::now();
        let stopped = || {
            cancel.load(Ordering::Acquire)
                || lease.cancel.load(Ordering::Acquire)
                || start.elapsed() >= Duration::from_secs(180)
        };
        if stopped() {
            return Err(Error::Cancelled);
        }
        let build_id = ulid::Ulid::new().to_string();
        {
            let mut state = lease.publish()?;
            state.candidate = None;
            state.status = Some(JobStatus {
                snapshot_id: request.snapshot_id.clone(),
                build_id: Some(build_id.clone()),
                state: "building".into(),
                candidate_id: None,
            });
        }
        let built = lease.bounded(lease.backend.build(&source, &stopped), start, cancel)?;
        lease.bounded(validate(), start, cancel)?;
        let current = lease.bounded(
            lease
                .backend
                .inspect(&root, &package, &source.entry, &stopped),
            start,
            cancel,
        )?;
        lease.bounded(validate(), start, cancel)?;
        lease.bounded(Ok(()), start, cancel)?;
        if current.digest()? != digest
            || incant_source::check_recovery(&root).map_err(|_| Error::Stale)? != epoch
        {
            return Err(Error::Stale);
        }
        if built.diagnostics.len() > 65536 {
            return Err(Error::Invalid("diagnostics bound"));
        }
        let id = ulid::Ulid::new().to_string();
        let provenance = serde_json::json!({"actor":actor,"description":request.description,"worker":built.receipt,"project_id":project,"revision":request.expected_revision,"source_digest":digest,"not_installed":true});
        let diagnostics = built.diagnostics.clone();
        let mut state = lease.publish()?;
        state.candidate = Some(built);
        state.status = Some(JobStatus {
            snapshot_id: request.snapshot_id,
            build_id: Some(build_id.clone()),
            state: "complete".into(),
            candidate_id: Some(id.clone()),
        });
        Ok(BuildResult {
            build_id,
            candidate_id: id,
            source_digest: digest,
            untrusted_diagnostics: diagnostics,
            provenance,
        })
    }
}
struct Lease<'a> {
    service: &'a Service,
    cancel: Arc<AtomicBool>,
    backend: Arc<dyn Backend>,
    generation: String,
}
impl Lease<'_> {
    fn bounded<T>(&self, result: Result<T>, start: Instant, cancel: &AtomicBool) -> Result<T> {
        // Cleanup failure takes priority; cancellation/deadline never hide an owner.
        if matches!(&result, Err(Error::Cleanup(_))) {
            return self.check(result);
        }
        if start.elapsed() >= Duration::from_secs(180) {
            return Err(Error::Deadline);
        }
        if cancel.load(Ordering::Acquire) || self.cancel.load(Ordering::Acquire) {
            self.cancel.store(true, Ordering::Release);
            return Err(Error::Cancelled);
        }
        self.check(result)
    }
    fn check<T>(&self, result: Result<T>) -> Result<T> {
        if matches!(&result, Err(Error::Cleanup(_))) {
            self.service.lock()?.poisoned = true;
        }
        if matches!(&result, Err(Error::Cancelled)) {
            self.cancel.store(true, Ordering::Release);
        }
        result
    }
    fn publish(&self) -> Result<std::sync::MutexGuard<'_, State>> {
        let state = self.service.lock()?;
        let binding = state.binding.as_ref().ok_or(Error::Stale)?;
        if self.cancel.load(Ordering::Acquire)
            || binding.generation != self.generation
            || !same_node(
                &binding.node.metadata()?,
                &std::fs::metadata(&binding.root)?,
            )
        {
            return Err(Error::Stale);
        }
        Ok(state)
    }
}
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if let Ok(mut s) = self.service.state.lock() {
            if std::thread::panicking() {
                s.poisoned = true;
            }
            let poisoned = s.poisoned;
            if let Some(status) = &mut s.status
                && status.state == "building"
            {
                status.state = if poisoned {
                    "cleanup_unproven"
                } else if self.cancel.load(Ordering::Acquire) {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
            }
            s.active = None;
        }
    }
}
#[cfg(unix)]
fn same_node(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (a.dev(), a.ino()) == (b.dev(), b.ino())
}
#[cfg(not(unix))]
fn same_node(_: &std::fs::Metadata, _: &std::fs::Metadata) -> bool {
    false
}
