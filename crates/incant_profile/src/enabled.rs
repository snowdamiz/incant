use crate::*;
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
static ACTIVE: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<State> = Mutex::new(State::new());
#[derive(Clone, Copy)]
struct Source {
    id: Option<[u8; 16]>,
    totals: Totals,
}
struct State {
    generation: u64,
    phases: [Totals; PHASE_COUNT],
    sources: [Option<Source>; MAX_SOURCES],
    dropped: u64,
    frames: [u64; 2],
    heap: QuickJsHeap,
    tracy: bool,
    distribution: crate::frame::Frames,
}
impl State {
    const fn new() -> Self {
        const ZERO: Totals = Totals {
            calls: 0,
            wall_ns: 0,
            thread_cpu_ns: 0,
            max_wall_ns: 0,
            unavailable_cpu_samples: 0,
            unwound: 0,
        };
        Self {
            generation: 0,
            phases: [ZERO; PHASE_COUNT],
            sources: [None; MAX_SOURCES],
            dropped: 0,
            frames: [0; 2],
            heap: QuickJsHeap {
                samples: 0,
                latest: HeapSample {
                    used_bytes: 0,
                    allocated_bytes: 0,
                    allocation_count: 0,
                },
                peak_used_bytes: 0,
                peak_allocated_bytes: 0,
            },
            tracy: false,
            distribution: crate::frame::Frames::empty(),
        }
    }
}
fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}
fn cpu() -> Option<Duration> {
    #[cfg(any(unix, windows))]
    {
        cpu_time::ThreadTime::try_now()
            .ok()
            .map(|v| v.as_duration())
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}
fn nanos(v: Duration) -> u64 {
    v.as_nanos().min(u128::from(u64::MAX)) as u64
}
/// Exclusive process capture. It includes instrumented work on all threads;
/// unrelated simultaneous games in that process are not falsely assigned to one world.
/// Dropping retires summary collection. It does not stop an explicit process Tracy transport.
pub struct Capture {
    generation: u64,
    allocation_start: Allocations,
    _thread: PhantomData<Rc<()>>,
}
impl Capture {
    pub fn start() -> Result<Self, Error> {
        Self::begin(false)
    }
    /// Standalone CLI only: transport is loopback/on-demand, lives until process exit.
    /// No unsafe Tracy shutdown or restart is attempted. Embedded hosts use start().
    pub fn start_tracy_process() -> Result<Self, Error> {
        Self::begin(true)
    }
    fn begin(tracy: bool) -> Result<Self, Error> {
        if !cfg!(any(unix, windows)) || cpu().is_none() {
            return Err(Error::ClockUnavailable);
        }
        #[cfg(not(all(feature = "tracy", any(unix, windows))))]
        if tracy {
            return Err(Error::TracyUnavailable);
        }
        let mut s = state();
        if ACTIVE.load(Ordering::Acquire) != 0 {
            return Err(Error::Busy);
        }
        let generation = s.generation.checked_add(1).ok_or(Error::Sequence)?;
        *s = State {
            generation,
            tracy,
            distribution: crate::frame::Frames::allocated()?,
            ..State::new()
        };
        #[cfg(all(feature = "tracy", any(unix, windows)))]
        if tracy {
            crate::tracy::start();
        }
        let allocation_start = crate::allocation::snapshot();
        ACTIVE.store(generation, Ordering::Release);
        Ok(Self {
            generation,
            allocation_start,
            _thread: PhantomData,
        })
    }
    /// Non-transactional atomic counter observation; no allocation or heap walk.
    pub fn allocations(&self) -> Allocations {
        crate::allocation::since(self.allocation_start)
    }
    /// Allocates bounded report rows only on explicit final observation.
    pub fn report(&self) -> Report {
        let s = state();
        let allocations = crate::allocation::since(self.allocation_start);
        Report {
            version: 2,
            frame_wall: s.distribution.report(),
            scope: "process; inclusive wall and current-thread CPU zones; allocation counters are requested Rust sizes, not RSS",
            generation: self.generation,
            zones: Phase::ALL
                .into_iter()
                .filter(|p| s.phases[*p as usize].calls > 0)
                .map(|p| PhaseReport {
                    name: p.name(),
                    totals: s.phases[p as usize],
                })
                .collect(),
            scripts: s
                .sources
                .iter()
                .enumerate()
                .filter_map(|(slot, r)| {
                    r.map(|r| SourceReport {
                        slot,
                        source: r.id,
                        totals: r.totals,
                    })
                })
                .collect(),
            dropped_source_samples: s.dropped,
            completed_frames: s.frames[0],
            failed_frames: s.frames[1],
            quickjs_heap: s.heap,
            rust_allocations: allocations,
            tracy_process_transport: s.tracy,
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        let _lock = state();
        let _ = ACTIVE.compare_exchange(self.generation, 0, Ordering::AcqRel, Ordering::Relaxed);
    }
}
#[inline]
pub fn enabled() -> bool {
    ACTIVE.load(Ordering::Acquire) != 0
}
struct Started {
    generation: u64,
    phase: Phase,
    source: Option<usize>,
    frame: Option<u64>,
    wall: Instant,
    cpu: Option<Duration>,
    _thread: PhantomData<Rc<()>>,
    #[cfg(all(feature = "tracy", any(unix, windows)))]
    _trace: Option<tracy_client::Span>,
}
pub struct Scope(Option<Started>);
#[inline]
pub fn scope(phase: Phase) -> Scope {
    begin(phase, None, None)
}
#[inline]
pub fn script_scope(source: Option<[u8; 16]>) -> Scope {
    begin(Phase::ScriptSource, Some(source), None)
}
fn begin(phase: Phase, source: Option<Option<[u8; 16]>>, expected: Option<u64>) -> Scope {
    let generation = ACTIVE.load(Ordering::Acquire);
    if generation == 0 || expected.is_some_and(|expected| expected != generation) {
        return Scope(None);
    }
    let mut s = state();
    if ACTIVE.load(Ordering::Acquire) != generation {
        return Scope(None);
    }
    let slot = source.and_then(|id| {
        if let Some(i) = s.sources.iter().position(|r| r.is_some_and(|r| r.id == id)) {
            return Some(i);
        }
        if let Some(i) = s.sources.iter().position(Option::is_none) {
            s.sources[i] = Some(Source {
                id,
                totals: Totals::default(),
            });
            return Some(i);
        }
        s.dropped = s.dropped.saturating_add(1);
        None
    });
    #[cfg(all(feature = "tracy", any(unix, windows)))]
    let trace = if s.tracy {
        crate::tracy::begin(phase, slot)
    } else {
        None
    };
    let frame = s.distribution.begin_scope(phase);
    drop(s);
    Scope(Some(Started {
        generation,
        phase,
        source: slot,
        frame,
        wall: Instant::now(),
        cpu: cpu(),
        _thread: PhantomData,
        #[cfg(all(feature = "tracy", any(unix, windows)))]
        _trace: trace,
    }))
}
impl Drop for Scope {
    fn drop(&mut self) {
        let Some(start) = self.0.take() else {
            return;
        };
        let wall = nanos(start.wall.elapsed());
        let cpu = start
            .cpu
            .zip(cpu())
            .and_then(|(a, b)| b.checked_sub(a))
            .map(nanos);
        let mut s = state();
        if ACTIVE.load(Ordering::Acquire) != start.generation {
            return;
        }
        s.distribution
            .end_scope(start.frame, start.phase, wall, std::thread::panicking());
        let update = |t: &mut Totals| {
            t.calls = t.calls.saturating_add(1);
            t.wall_ns = t.wall_ns.saturating_add(wall);
            t.max_wall_ns = t.max_wall_ns.max(wall);
            if let Some(cpu) = cpu {
                t.thread_cpu_ns = t.thread_cpu_ns.saturating_add(cpu);
            } else {
                t.unavailable_cpu_samples = t.unavailable_cpu_samples.saturating_add(1);
            }
            if std::thread::panicking() {
                t.unwound = t.unwound.saturating_add(1);
            }
        };
        update(&mut s.phases[start.phase as usize]);
        if let Some(i) = start.source {
            update(
                &mut s.sources[i]
                    .as_mut()
                    .expect("source reserved in same generation")
                    .totals,
            );
        }
    }
}
pub fn quickjs_heap(sample: HeapSample) {
    let generation = ACTIVE.load(Ordering::Acquire);
    if generation == 0 {
        return;
    }
    let mut s = state();
    if ACTIVE.load(Ordering::Acquire) != generation {
        return;
    }
    s.heap.samples = s.heap.samples.saturating_add(1);
    s.heap.latest = sample;
    s.heap.peak_used_bytes = s.heap.peak_used_bytes.max(sample.used_bytes);
    s.heap.peak_allocated_bytes = s.heap.peak_allocated_bytes.max(sample.allocated_bytes);
    #[cfg(all(feature = "tracy", any(unix, windows)))]
    if s.tracy {
        crate::tracy::heap(sample);
    }
}
pub fn frame_outcome(success: bool) {
    let generation = ACTIVE.load(Ordering::Acquire);
    if generation == 0 {
        return;
    }
    let mut s = state();
    if ACTIVE.load(Ordering::Acquire) != generation {
        return;
    }
    let i = usize::from(!success);
    s.frames[i] = s.frames[i].saturating_add(1);
    #[cfg(all(feature = "tracy", any(unix, windows)))]
    if s.tracy {
        crate::tracy::frame(crate::allocation::snapshot());
    }
}

/// Measure one synchronous simulation attempt. Failure and unwind invalidate
/// quantiles; event-loop waits and presentation-only refreshes stay outside.
#[inline]
pub fn frame<T, E>(run: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let mut guard = FrameGuard::begin();
    let result = run();
    guard.success = result.is_ok();
    result
}
struct FrameGuard {
    generation: u64,
    id: Option<u64>,
    scope: Option<Scope>,
    success: bool,
}
impl FrameGuard {
    fn begin() -> Self {
        let generation = ACTIVE.load(Ordering::Acquire);
        let mut guard = Self {
            generation,
            id: None,
            scope: None,
            success: false,
        };
        if generation == 0 {
            return guard;
        }
        let mut s = state();
        if ACTIVE.load(Ordering::Acquire) != generation {
            guard.generation = 0;
            return guard;
        }
        guard.id = s.distribution.begin();
        drop(s);
        if guard.id.is_some() {
            guard.scope = Some(begin(Phase::Frame, None, Some(generation)));
        }
        guard
    }
}
impl Drop for FrameGuard {
    fn drop(&mut self) {
        drop(self.scope.take());
        if self.generation == 0 {
            return;
        }
        let mut s = state();
        if ACTIVE.load(Ordering::Acquire) != self.generation {
            return;
        }
        s.distribution.finish(self.id, self.success);
        let i = usize::from(!self.success);
        s.frames[i] = s.frames[i].saturating_add(1);
        #[cfg(all(feature = "tracy", any(unix, windows)))]
        if s.tracy {
            crate::tracy::frame(crate::allocation::snapshot());
        }
    }
}
