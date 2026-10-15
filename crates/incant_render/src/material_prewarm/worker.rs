//! One process-wide native compiler worker. No frame locks, waits, driver calls
//! or file I/O. Cancellation cannot interrupt an in-flight native driver call;
//! its permit stays occupied until compilation and captured-resource teardown
//! finish. Gate release precedes thread return; only a successful join proves exit.
use incant_render_data::material_keys::{MATERIAL_KEY_COUNT, MaterialKey};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::{
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FailureKind {
    Validation,
    Internal,
    OutOfMemory,
    Panic,
    Observation,
}
#[derive(Clone, Debug, thiserror::Error, serde::Serialize)]
#[error("material compilation {kind:?}: {detail}")]
pub struct CompileFailure {
    pub kind: FailureKind,
    pub detail: String,
}
impl CompileFailure {
    pub(super) fn new(kind: FailureKind, detail: impl std::fmt::Display) -> Self {
        Self {
            kind,
            detail: detail.to_string().chars().take(256).collect(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("the one material compiler worker is active or quarantined")]
    Busy,
    #[error("cannot start material compiler worker: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("material compiler is closed")]
    Closed,
    #[error("material compiler remains in flight; replacement stays refused")]
    StillBusy,
    #[error("material compiler thread panicked")]
    Panic,
    #[error("material compiler wait must be no longer than two seconds")]
    Deadline,
}
struct Shared<T> {
    slots: [OnceLock<Result<T, CompileFailure>>; MATERIAL_KEY_COUNT],
    queued: AtomicU32,
    requested: AtomicU32,
    cancelled: AtomicBool,
    #[cfg(test)]
    paused: AtomicBool,
}
struct Permit(Arc<AtomicBool>);
impl Permit {
    fn acquire(gate: Arc<AtomicBool>) -> Result<Self, WorkerError> {
        gate.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| WorkerError::Busy)?;
        Ok(Self(gate))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub(super) struct Worker<T> {
    shared: Arc<Shared<T>>,
    handle: Option<JoinHandle<()>>,
    pub(super) thread_identity: String,
}
impl<T: Send + Sync + 'static> Worker<T> {
    pub(super) fn new(
        ready: impl IntoIterator<Item = (MaterialKey, T)>,
        compile: impl FnMut(MaterialKey) -> Result<T, CompileFailure> + Send + 'static,
    ) -> Result<Self, WorkerError> {
        static GATE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
        Self::with_gate(
            Arc::clone(GATE.get_or_init(|| Arc::new(AtomicBool::new(false)))),
            ready,
            compile,
        )
    }
    fn with_gate(
        gate: Arc<AtomicBool>,
        ready: impl IntoIterator<Item = (MaterialKey, T)>,
        compile: impl FnMut(MaterialKey) -> Result<T, CompileFailure> + Send + 'static,
    ) -> Result<Self, WorkerError> {
        let permit = Permit::acquire(gate)?;
        let shared = Arc::new(Shared {
            slots: std::array::from_fn(|_| OnceLock::new()),
            queued: AtomicU32::new(0),
            requested: AtomicU32::new(0),
            cancelled: AtomicBool::new(false),
            #[cfg(test)]
            paused: AtomicBool::new(false),
        });
        for (key, value) in ready {
            assert!(
                shared.slots[key.index()].set(Ok(value)).is_ok(),
                "distinct admitted keys"
            );
        }
        let state = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("incant-material-compiler".into())
            .spawn(move || {
                let _permit = permit;
                // Move captures into later locals: reverse local destruction
                // drops the compiler/native resources and state before permit.
                // Leaving them as closure captures releases the gate too early.
                let compiler_state = state;
                let mut compiler = compile;
                loop {
                    if compiler_state.cancelled.load(Ordering::Acquire) {
                        break;
                    }
                    let queued = compiler_state.queued.swap(0, Ordering::AcqRel);
                    if queued == 0 {
                        thread::park();
                        continue;
                    }
                    for index in 0..MATERIAL_KEY_COUNT {
                        if compiler_state.cancelled.load(Ordering::Acquire) {
                            return;
                        }
                        if queued & (1 << index) == 0 || compiler_state.slots[index].get().is_some()
                        {
                            continue;
                        }
                        #[cfg(test)]
                        while compiler_state.paused.load(Ordering::Acquire) {
                            if compiler_state.cancelled.load(Ordering::Acquire) {
                                return;
                            }
                            thread::park();
                        }
                        let key = MaterialKey::try_from(index as u8).expect("finite key bitmap");
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            compiler(key)
                        }))
                        .unwrap_or_else(|_| {
                            Err(CompileFailure::new(FailureKind::Panic, "compiler panic"))
                        });
                        let panicked = result
                            .as_ref()
                            .is_err_and(|failure| failure.kind == FailureKind::Panic);
                        let _ = compiler_state.slots[index].set(result);
                        if panicked {
                            compiler_state.cancelled.store(true, Ordering::Release);
                            return;
                        }
                    }
                }
            })?;
        let thread_identity = format!("{:?}", handle.thread().id());
        Ok(Self {
            shared,
            handle: Some(handle),
            thread_identity,
        })
    }
    /// A read followed by coalesced request only. OnceLock::get never waits for
    /// initialization; the worker publishes a complete owned result atomically.
    pub(super) fn get_or_request(
        &self,
        key: MaterialKey,
    ) -> Result<Option<&Result<T, CompileFailure>>, WorkerError> {
        if self.shared.cancelled.load(Ordering::Acquire) {
            return Err(WorkerError::Closed);
        }
        if let Some(value) = self.shared.slots[key.index()].get() {
            return Ok(Some(value));
        }
        self.shared
            .requested
            .fetch_or(1 << key.index(), Ordering::Relaxed);
        self.shared
            .queued
            .fetch_or(1 << key.index(), Ordering::Release);
        if let Some(handle) = &self.handle {
            handle.thread().unpark();
        }
        Ok(None)
    }
    pub(super) fn get(&self, key: MaterialKey) -> Option<&Result<T, CompileFailure>> {
        self.shared.slots[key.index()].get()
    }
    pub(super) fn requested(&self) -> u32 {
        self.shared.requested.load(Ordering::Acquire)
    }
    #[cfg(test)]
    pub(super) fn pause(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
        if let Some(handle) = &self.handle {
            handle.thread().unpark();
        }
    }
    /// Explicit cold shutdown. A timeout preserves the process-wide permit and
    /// owned thread/resources; Drop only requests cancellation and detaches it.
    pub(super) fn close(&mut self, timeout: Duration) -> Result<(), WorkerError> {
        if timeout > Duration::from_secs(2) {
            return Err(WorkerError::Deadline);
        }
        self.cancel();
        let start = Instant::now();
        while self
            .handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            if start.elapsed() >= timeout {
                return Err(WorkerError::StillBusy);
            }
            thread::sleep(Duration::from_millis(1).min(timeout.saturating_sub(start.elapsed())));
        }
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| WorkerError::Panic)?;
        }
        Ok(())
    }
}
impl<T> Worker<T> {
    fn cancel(&self) {
        self.shared.cancelled.store(true, Ordering::Release);
        self.shared.queued.store(0, Ordering::Release);
        if let Some(handle) = &self.handle {
            handle.thread().unpark();
        }
    }
}
impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicUsize, mpsc};
    #[test]
    fn pending_selection_is_nonblocking_coalesced_and_publishes_one_owned_result() {
        let requester = thread::current().id();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&calls);
        let mut worker = Worker::with_gate(Arc::new(AtomicBool::new(false)), [], move |key| {
            assert_ne!(thread::current().id(), requester);
            count.fetch_add(1, Ordering::Relaxed);
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            Ok(key.index())
        })
        .unwrap();
        let key = MaterialKey::try_from(1).unwrap();
        assert!(worker.get_or_request(key).unwrap().is_none());
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        // The actual compiler closure is still blocked. These calls can return
        // fallback without waiting; this does not prove the driver cannot stall.
        for _ in 0..100 {
            assert!(worker.get_or_request(key).unwrap().is_none());
        }
        release_tx.send(()).unwrap();
        let start = Instant::now();
        while worker.get(key).is_none() {
            assert!(start.elapsed() < Duration::from_secs(2));
            thread::yield_now();
        }
        assert_eq!(*worker.get(key).unwrap().as_ref().unwrap(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        worker.close(Duration::from_secs(1)).unwrap();
    }
    #[test]
    fn cancelled_inflight_job_retains_one_quarantine_permit_until_owned_teardown() {
        let gate = Arc::new(AtomicBool::new(false));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut worker = Worker::with_gate(Arc::clone(&gate), [], move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            Ok(())
        })
        .unwrap();
        let key = MaterialKey::try_from(0).unwrap();
        worker.get_or_request(key).unwrap();
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            worker.close(Duration::ZERO),
            Err(WorkerError::StillBusy)
        ));
        assert!(matches!(
            Worker::with_gate(Arc::clone(&gate), [], |_| Ok(())),
            Err(WorkerError::Busy)
        ));
        drop(worker);
        assert!(matches!(
            Worker::with_gate(Arc::clone(&gate), [], |_| Ok(())),
            Err(WorkerError::Busy)
        ));
        release_tx.send(()).unwrap();
        let start = Instant::now();
        while gate.load(Ordering::Acquire) {
            assert!(start.elapsed() < Duration::from_secs(2));
            thread::yield_now();
        }
        let mut replacement = Worker::with_gate(gate, [], |_| Ok(())).unwrap();
        replacement.close(Duration::from_secs(1)).unwrap();
    }
    #[test]
    fn replacement_stays_busy_while_captured_native_owner_is_being_destroyed() {
        struct Owner {
            started: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Drop for Owner {
            fn drop(&mut self) {
                let _ = self.started.send(());
                let _ = self.release.recv_timeout(Duration::from_secs(2));
            }
        }
        let gate = Arc::new(AtomicBool::new(false));
        let (started, wait) = mpsc::channel();
        let (release, stopped) = mpsc::channel();
        let owner = Owner {
            started,
            release: stopped,
        };
        let mut worker = Worker::with_gate(Arc::clone(&gate), [], move |_| {
            let _keep_owned = &owner;
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            worker.close(Duration::ZERO),
            Err(WorkerError::StillBusy)
        ));
        wait.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            Worker::with_gate(Arc::clone(&gate), [], |_| Ok(())),
            Err(WorkerError::Busy)
        ));
        release.send(()).unwrap();
        worker.close(Duration::from_secs(1)).unwrap();
        let mut replacement = Worker::with_gate(gate, [], |_| Ok(())).unwrap();
        replacement.close(Duration::from_secs(1)).unwrap();
    }
    #[test]
    fn a_failed_key_is_published_once_without_retrying_or_blocking_other_keys() {
        let calls = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&calls);
        let mut worker = Worker::with_gate(Arc::new(AtomicBool::new(false)), [], move |key| {
            count.fetch_add(1, Ordering::Relaxed);
            if key.index() == 0 {
                Err(CompileFailure::new(
                    FailureKind::Validation,
                    "controlled refusal",
                ))
            } else {
                Ok(key.index())
            }
        })
        .unwrap();
        for index in [0, 1] {
            let key = MaterialKey::try_from(index).unwrap();
            worker.get_or_request(key).unwrap();
            let start = Instant::now();
            while worker.get(key).is_none() {
                assert!(start.elapsed() < Duration::from_secs(2));
                thread::yield_now();
            }
        }
        for _ in 0..100 {
            assert!(
                worker
                    .get_or_request(MaterialKey::try_from(0).unwrap())
                    .unwrap()
                    .unwrap()
                    .is_err()
            );
        }
        assert_eq!(
            *worker
                .get(MaterialKey::try_from(1).unwrap())
                .unwrap()
                .as_ref()
                .unwrap(),
            1
        );
        worker.close(Duration::from_secs(1)).unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(matches!(
            worker.get_or_request(MaterialKey::try_from(1).unwrap()),
            Err(WorkerError::Closed)
        ));
    }
}
