//! Default-off observation of native creation calls, not proof of driver work.
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    fmt,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
use parking_lot::Mutex;
use std::{
    thread::{self, ThreadId},
    time::Instant,
};

pub const MAX_SAMPLES: usize = 4096;
const _: () = assert!(size_of::<Row>() <= 96);
static NEXT_DEVICE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    Allocation,
    AlreadyStarted,
    NotStarted,
    Context,
    ScopeBusy,
    Identity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    NagaLibrary,
    MslLibrary,
    BinaryLibrary,
    RenderPipeline,
    MeshPipeline,
    ComputePipeline,
}
pub const KIND_COUNT: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Unspecified,
    Vertex,
    Fragment,
    Compute,
    Task,
    Mesh,
    RayGeneration,
    Miss,
    AnyHit,
    ClosestHit,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Unscoped,
    Startup,
    Catalog,
    Background,
    Gameplay,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Context {
    pub session_generation: u64,
    pub surface_generation: u64,
    pub segment: u64,
    pub host_step: u64,
    pub phase: Phase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Success,
    Error,
    Unwound,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorDomain {
    None,
    Library,
    RenderPipeline,
    ComputePipeline,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorIdentity {
    pub domain: ErrorDomain,
    pub code: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub sequence: u64,
    pub context: Context,
    /// None when the entry clock precedes the device epoch.
    pub begin_ns: Option<u64>,
    /// None while pending or when the end clock precedes entry; outcome
    /// distinguishes those states. A genuine zero duration remains Some(0).
    pub elapsed_ns: Option<u64>,
    pub error_code: i64,
    pub kind: Kind,
    pub stage: Stage,
    pub outcome: Outcome,
    pub error_domain: ErrorDomain,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub attempts: u64,
    pub successes: u64,
    pub errors: u64,
    pub unwound: u64,
}
/// Cumulative invalid clock comparisons, including calls without retained rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockAnomalies {
    pub entry: u64,
    pub completion: u64,
    pub snapshot: u64,
}
struct ActiveScope {
    thread: ThreadId,
    context: Context,
}
struct State {
    rows: Option<Box<[Row]>>,
    used: usize,
    sequence: u64,
    counts: [Counts; KIND_COUNT],
    unbound: [Counts; KIND_COUNT],
    overflow: u64,
    clock_anomalies: ClockAnomalies,
    saturated: bool,
    scope: Option<ActiveScope>,
}
struct Shared {
    device: u64,
    epoch: Instant,
    state: Mutex<State>,
}
/// A diagnostic handle only: owns no native device, pipeline, window or callback.
#[derive(Clone)]
pub struct Observer(Arc<Shared>);
impl fmt::Debug for Observer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompilationObserver")
            .field("device", &self.0.device)
            .finish_non_exhaustive()
    }
}
impl Default for Observer {
    fn default() -> Self {
        let device = take_identity(&NEXT_DEVICE);
        Self(Arc::new(Shared {
            device,
            epoch: Instant::now(),
            state: Mutex::new(State {
                rows: None,
                used: 0,
                sequence: 0,
                counts: [Counts::default(); KIND_COUNT],
                unbound: [Counts::default(); KIND_COUNT],
                overflow: 0,
                clock_anomalies: ClockAnomalies::default(),
                saturated: false,
                scope: None,
            }),
        }))
    }
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub device: u64,
    pub rows: Vec<Row>,
    pub counts: [Counts; KIND_COUNT],
    /// Calls entered before capture was started, including later completions.
    pub unbound: [Counts; KIND_COUNT],
    pub overflow: u64,
    pub clock_anomalies: ClockAnomalies,
    pub saturated: bool,
    pub capture_started: bool,
    pub scope_active: bool,
    /// None when the snapshot clock precedes the device epoch.
    pub snapshot_ns: Option<u64>,
}
impl Observer {
    /// Allocate once on a cold host path, before catalog preparation. Never reset
    /// counts or discard a prefix when capture starts. Not a GPU operation.
    pub fn start(&self, capacity: usize) -> Result<(), Error> {
        if capacity == 0 || capacity > MAX_SAMPLES {
            return Err(Error::Capacity);
        }
        if self.0.device == 0 {
            return Err(Error::Identity);
        }
        let mut state = self.0.state.lock();
        if state.rows.is_some() {
            return Err(Error::AlreadyStarted);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation)?;
        rows.resize(
            capacity,
            Row {
                sequence: 0,
                context: Context::default(),
                begin_ns: None,
                elapsed_ns: None,
                error_code: 0,
                kind: Kind::NagaLibrary,
                stage: Stage::Unspecified,
                outcome: Outcome::Pending,
                error_domain: ErrorDomain::None,
            },
        );
        state.rows = Some(rows.into_boxed_slice());
        Ok(())
    }
    /// Scope only the calling thread. Other threads remain explicitly unscoped.
    /// Reject nesting instead of attributing work to a newer ambient context.
    pub fn scope(&self, context: Context) -> Result<Scope, Error> {
        if context.phase == Phase::Unscoped
            || context.session_generation == 0
            || context.phase == Phase::Gameplay
                && (context.surface_generation == 0
                    || context.segment == 0
                    || context.host_step == 0)
        {
            return Err(Error::Context);
        }
        let mut state = self.0.state.lock();
        if state.rows.is_none() {
            return Err(Error::NotStarted);
        }
        if state.scope.is_some() {
            return Err(Error::ScopeBusy);
        }
        state.scope = Some(ActiveScope {
            thread: thread::current().id(),
            context,
        });
        Ok(Scope(self.clone()))
    }
    pub(crate) fn begin(&self, kind: Kind, stage: Stage) -> Call {
        let mut state = self.0.state.lock();
        let index = kind as usize;
        let mut saturated = state.saturated;
        bump(&mut state.sequence, &mut saturated);
        bump(&mut state.counts[index].attempts, &mut saturated);
        let unbound = state.rows.is_none();
        if unbound {
            bump(&mut state.unbound[index].attempts, &mut saturated);
        }
        let context = state
            .scope
            .as_ref()
            .filter(|s| s.thread == thread::current().id())
            .map_or_else(Context::default, |s| s.context);
        let sequence = state.sequence;
        let at = state.used;
        let slot = match state.rows.as_mut() {
            Some(rows) if at < rows.len() => {
                rows[at] = Row {
                    sequence,
                    context,
                    begin_ns: None,
                    elapsed_ns: None,
                    error_code: 0,
                    kind,
                    stage,
                    outcome: Outcome::Pending,
                    error_domain: ErrorDomain::None,
                };
                state.used += 1;
                Some(at)
            }
            Some(_) => {
                bump(&mut state.overflow, &mut saturated);
                None
            }
            None => None,
        };
        // Timestamp after admission bookkeeping. The native call follows begin;
        // return/dispatch overhead remains in the observed window.
        let begin = Instant::now();
        let begin_ns = checked_nanos(
            begin,
            self.0.epoch,
            &mut state.clock_anomalies.entry,
            &mut saturated,
        );
        if let Some(slot) = slot {
            state.rows.as_mut().unwrap()[slot].begin_ns = begin_ns;
        }
        state.saturated = saturated;
        Call {
            observer: self.clone(),
            kind,
            unbound,
            slot,
            begin,
            completed: false,
        }
    }
    /// Final/cold-path allocation only. Pending rows remain pending; no wait,
    /// GPU polling or invented completion. Snapshot/JSON memory is additional.
    pub fn snapshot(&self) -> Snapshot {
        let mut state = self.0.state.lock();
        let mut saturated = state.saturated;
        let snapshot_ns = checked_nanos(
            Instant::now(),
            self.0.epoch,
            &mut state.clock_anomalies.snapshot,
            &mut saturated,
        );
        state.saturated = saturated;
        Snapshot {
            device: self.0.device,
            rows: state
                .rows
                .as_ref()
                .map_or_else(Vec::new, |rows| rows[..state.used].to_vec()),
            counts: state.counts,
            unbound: state.unbound,
            overflow: state.overflow,
            clock_anomalies: state.clock_anomalies,
            saturated,
            capture_started: state.rows.is_some(),
            scope_active: state.scope.is_some(),
            snapshot_ns,
        }
    }
}
/// Drop clears only diagnostic context; no native operation or wait.
pub struct Scope(Observer);
impl Drop for Scope {
    fn drop(&mut self) {
        self.0 .0.state.lock().scope = None;
    }
}
pub(crate) struct Call {
    observer: Observer,
    kind: Kind,
    unbound: bool,
    slot: Option<usize>,
    begin: Instant,
    completed: bool,
}
impl Call {
    pub(super) fn complete(mut self, ended: Instant, error: Option<ErrorIdentity>) {
        self.record(
            ended,
            error.map_or(Outcome::Success, |_| Outcome::Error),
            error,
        );
    }
    fn record(&mut self, ended: Instant, outcome: Outcome, error: Option<ErrorIdentity>) {
        let mut state = self.observer.0.state.lock();
        let mut saturated = state.saturated;
        let elapsed_ns = checked_nanos(
            ended,
            self.begin,
            &mut state.clock_anomalies.completion,
            &mut saturated,
        );
        let index = self.kind as usize;
        finish(&mut state.counts[index], outcome, &mut saturated);
        if self.unbound {
            finish(&mut state.unbound[index], outcome, &mut saturated);
        }
        if let Some(slot) = self.slot {
            let row = &mut state.rows.as_mut().unwrap()[slot];
            row.elapsed_ns = elapsed_ns;
            row.outcome = outcome;
            if let Some(error) = error {
                row.error_domain = error.domain;
                row.error_code = error.code;
            }
        }
        state.saturated = saturated;
        self.completed = true;
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        if !self.completed {
            self.record(Instant::now(), Outcome::Unwound, None);
        }
    }
}
fn take_identity(counter: &AtomicU64) -> u64 {
    let mut value = counter.load(Ordering::Relaxed);
    loop {
        let Some(next) = value.checked_add(1) else {
            return 0;
        };
        match counter.compare_exchange_weak(value, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return value,
            Err(current) => value = current,
        }
    }
}
fn bump(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *saturated = true;
    }
}
fn nanos(value: u128, saturated: &mut bool) -> u64 {
    u64::try_from(value).unwrap_or_else(|_| {
        *saturated = true;
        u64::MAX
    })
}
fn checked_nanos(
    end: Instant,
    start: Instant,
    anomalies: &mut u64,
    saturated: &mut bool,
) -> Option<u64> {
    match end.checked_duration_since(start) {
        Some(duration) => Some(nanos(duration.as_nanos(), saturated)),
        None => {
            bump(anomalies, saturated);
            None
        }
    }
}
fn finish(counts: &mut Counts, outcome: Outcome, saturated: &mut bool) {
    match outcome {
        Outcome::Success => bump(&mut counts.successes, saturated),
        Outcome::Error => bump(&mut counts.errors, saturated),
        Outcome::Unwound => bump(&mut counts.unwound, saturated),
        Outcome::Pending => unreachable!("A pending call is not completed"),
    }
}
#[cfg(test)]
#[path = "compilation_tests.rs"]
mod tests;
