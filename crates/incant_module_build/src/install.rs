//! A completed private candidate stays service-bound until durable publication.
//! Backend/package work never runs with the command-bus or service-state lock.
use crate::*;
use sha2::{Digest, Sha256};
use std::fs::File;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallRequest {
    pub snapshot_id: String,
    pub candidate_id: String,
    pub expected_revision: u64,
}
/// Output provenance only, never accepted from project/tool input.
#[derive(Clone, Debug, serde::Serialize)]
pub struct InstallationReceipt {
    pub snapshot_id: String,
    pub candidate_id: String,
    pub build_id: String,
    pub source_digest: String,
    pub raw_sha256: String,
    pub raw_bytes: usize,
    pub prepared_sha256: String,
    pub worker: serde_json::Value,
}
struct Candidate {
    path: PathBuf,
    parent: File,
    node: File,
    bytes: Vec<u8>,
}
impl Candidate {
    fn capture(path: PathBuf, root: &Path, worker: &serde_json::Value) -> Result<Self> {
        if !path.is_absolute() || path.canonicalize()? != path || path.starts_with(root) {
            return Err(Error::Invalid("private canonical candidate required"));
        }
        let parent = node::directory(path.parent().ok_or(Error::Invalid("candidate parent"))?)?;
        let node = node::read(&path)?;
        node::regular(&node)?;
        let bytes = backend::bounded(&path, incant_source::MAX_WASM_BYTES)?;
        if bytes.get(..8) != Some(b"\0asm\x01\0\0\0")
            || worker["raw_bytes"].as_u64() != Some(bytes.len() as u64)
            || worker["raw_sha256"].as_str()
                != Some(format!("{:x}", Sha256::digest(&bytes)).as_str())
            || !node::matches(&node, &path)?
        {
            return Err(Error::Invalid(
                "candidate differs from completed worker receipt",
            ));
        }
        let candidate = Self {
            path,
            parent,
            node,
            bytes,
        };
        candidate.validate()?;
        Ok(candidate)
    }
    fn validate(&self) -> Result<()> {
        node::regular(&self.node)?;
        if !node::matches(&self.parent, self.path.parent().ok_or(Error::Stale)?)?
            || !node::matches(&self.node, &self.path)?
            || backend::bounded(&self.path, incant_source::MAX_WASM_BYTES)? != self.bytes
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
fn hash_field(worker: &serde_json::Value, key: &str) -> Result<String> {
    let value = worker[key].as_str().ok_or(Error::Invalid("worker hash"))?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Invalid("worker hash"));
    }
    Ok(value.into())
}
fn check_worker(worker: &serde_json::Value) -> Result<()> {
    incant_source::bounded_json(worker, 65536)
        .map_err(|_| Error::Invalid("worker receipt bound"))?;
    for key in [
        "raw_sha256",
        "prepared_sha256",
        "policy_sha256",
        "host_tool_manifest_sha256",
        "host_closeout_sha256",
    ] {
        hash_field(worker, key)?;
    }
    if worker["raw_bytes"]
        .as_u64()
        .is_none_or(|n| n > incant_source::MAX_WASM_BYTES as u64)
        || worker["compiler_version"]
            .as_str()
            .is_none_or(|s| s.is_empty() || s.len() > 4096)
        || !worker["sdk_sources"].is_object()
        || !worker["service_sources"].is_object()
    {
        return Err(Error::Invalid("completed worker provenance required"));
    }
    Ok(())
}
struct Authority {
    request: InstallRequest,
    registered: RegisteredSource,
    project: String,
    root: PathBuf,
    package: String,
    entry: String,
    epoch: incant_source::SourceEpoch,
    receipt: InstallationReceipt,
}
impl Authority {
    fn validate(&self, state: &State) -> Result<()> {
        let snapshot = state.snapshot.as_ref().ok_or(Error::Stale)?;
        let status = state.status.as_ref().ok_or(Error::Stale)?;
        let candidate = state.candidate.as_ref().ok_or(Error::Stale)?;
        if state.poisoned
            || state
                .binding
                .as_ref()
                .is_none_or(|b| b.project != self.project || b.root != self.root)
            || snapshot.id != self.request.snapshot_id
            || snapshot.revision != self.request.expected_revision
            || snapshot.registered.as_ref() != Some(&self.registered)
            || snapshot.digest != self.receipt.source_digest
            || status.state != "complete"
            || status.snapshot_id != self.request.snapshot_id
            || status.candidate_id.as_ref() != Some(&self.request.candidate_id)
            || status.build_id.as_ref() != Some(&self.receipt.build_id)
            || candidate.receipt != self.receipt.worker
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
/// No payload or raw-byte accessor: the ordinary import preparation is owned by
/// this generation-bound lease and can leave only through locked publication.
pub struct Installation<'a, T> {
    lease: Lease<'a>,
    authority: Authority,
    candidate: Candidate,
    package: incant_source::RustPackageSnapshot,
    payload: T,
}
impl Service {
    pub fn prepare_install<T>(
        &self,
        project: &str,
        request: InstallRequest,
        registered: RegisteredSource,
        cancel: &AtomicBool,
        prepare: impl FnOnce(&Path, &[u8], &InstallationReceipt) -> Result<T>,
    ) -> Result<Installation<'_, T>> {
        registered.query(request.expected_revision)?;
        for id in [&request.snapshot_id, &request.candidate_id] {
            if ulid::Ulid::from_string(id)
                .map(|id| id.to_string())
                .ok()
                .as_ref()
                != Some(id)
            {
                return Err(Error::Invalid("installation identity"));
            }
        }
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
        let (authority, path) = {
            let state = lease.publish()?;
            let snapshot = state.snapshot.as_ref().ok_or(Error::Stale)?;
            let status = state.status.as_ref().ok_or(Error::Stale)?;
            let built = state.candidate.as_ref().ok_or(Error::Stale)?;
            check_worker(&built.receipt)?;
            let receipt = InstallationReceipt {
                snapshot_id: request.snapshot_id.clone(),
                candidate_id: request.candidate_id.clone(),
                build_id: status.build_id.clone().ok_or(Error::Stale)?,
                source_digest: snapshot.digest.clone(),
                raw_sha256: hash_field(&built.receipt, "raw_sha256")?,
                raw_bytes: built.receipt["raw_bytes"]
                    .as_u64()
                    .ok_or(Error::Invalid("worker size"))? as usize,
                prepared_sha256: hash_field(&built.receipt, "prepared_sha256")?,
                worker: built.receipt.clone(),
            };
            let authority = Authority {
                request,
                registered,
                project: project.into(),
                root: root.clone(),
                package: snapshot.package.clone(),
                entry: snapshot.source.entry.clone(),
                epoch: snapshot.epoch.clone(),
                receipt,
            };
            authority.validate(&state)?;
            (authority, built.candidate.clone())
        };
        let recapture = || -> Result<()> {
            if incant_source::check_recovery(&root).map_err(|_| Error::Stale)? != authority.epoch {
                return Err(Error::Stale);
            }
            let source = lease.bounded(
                lease.backend.inspect_registered(
                    &root,
                    &authority.package,
                    &authority.entry,
                    &stopped,
                ),
                start,
                cancel,
            )?;
            if source.digest()? != authority.receipt.source_digest
                || !authority.registered.matches_entry(&source)
                || incant_source::check_recovery(&root).map_err(|_| Error::Stale)?
                    != authority.epoch
            {
                return Err(Error::Stale);
            }
            Ok(())
        };
        recapture()?;
        let candidate = Candidate::capture(path, &root, &authority.receipt.worker)?;
        let payload = lease.bounded(
            prepare(&root, &candidate.bytes, &authority.receipt),
            start,
            cancel,
        )?;
        // Import validation/caching is allowed to take time. Reacquire the whole
        // bounded package without a bus guard before the final publication fence.
        recapture()?;
        let package =
            incant_source::RustPackageSnapshot::capture(&root, &authority.registered.path)
                .map_err(|_| Error::Stale)?;
        let source = SourcePackage {
            entry: authority.entry.clone(),
            files: package
                .files()
                .iter()
                .map(|(path, text)| SourceFile {
                    path: path.clone(),
                    text: text.clone(),
                })
                .collect(),
        };
        if source.digest()? != authority.receipt.source_digest {
            return Err(Error::Stale);
        }
        candidate.validate()?;
        lease.bounded(Ok(()), start, cancel)?;
        authority.validate(&lease.publish()?)?;
        Ok(Installation {
            lease,
            authority,
            candidate,
            package,
            payload,
        })
    }
}
impl<T> Installation<'_, T> {
    /// Caller holds the bus first. This method acquires only the service lock;
    /// the callback must not reenter the service or reacquire the bus. No backend
    /// runs here. Cancellation after this boundary cannot hide a durable commit.
    pub fn publish<R, E>(
        self,
        cancel: &AtomicBool,
        operation: impl FnOnce(T, InstallationReceipt) -> std::result::Result<R, E>,
    ) -> Result<std::result::Result<R, E>> {
        let Self {
            lease,
            authority,
            candidate,
            package,
            payload,
        } = self;
        let state = lease.publish()?;
        authority.validate(&state)?;
        if incant_source::check_recovery(&authority.root).map_err(|_| Error::Stale)?
            != authority.epoch
        {
            return Err(Error::Stale);
        }
        candidate.validate()?;
        package.validate().map_err(|_| Error::Stale)?;
        if cancel.load(Ordering::Acquire) || lease.cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let result = operation(payload, authority.receipt);
        drop(state);
        drop(lease);
        Ok(result)
    }
}
