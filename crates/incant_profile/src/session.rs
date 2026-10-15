//! Bounded owner-thread measurements, separate from process Capture. Only work
//! inside Capture::frame enters this collector. No allocator, heap or Tracy data.
use crate::{Error, MAX_SOURCES, PHASE_COUNT, Phase, Totals};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
mod report;
pub use report::*;

pub const MAX_FRAMES: usize = 256;
pub const MAX_STORAGE_BYTES: usize = 2 * 1024 * 1024;
thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}
struct Active {
    storage: Rc<RefCell<Storage>>,
    attempt: u64,
}
#[derive(Clone, Copy, Default)]
struct Metric {
    totals: Totals,
    outstanding: u32,
    invalid: bool,
}
#[derive(Clone)]
struct Row {
    attempt: u64,
    tick: u64,
    finished: bool,
    success: bool,
    phases: [Metric; PHASE_COUNT],
    sources: [Metric; MAX_SOURCES],
    dropped_sources: u64,
}
impl Default for Row {
    fn default() -> Self {
        Self {
            attempt: 0,
            tick: 0,
            finished: false,
            success: false,
            phases: [Metric::default(); PHASE_COUNT],
            sources: [Metric::default(); MAX_SOURCES],
            dropped_sources: 0,
        }
    }
}
const _: () = assert!(size_of::<Row>() * MAX_FRAMES + size_of::<Storage>() <= MAX_STORAGE_BYTES);
struct Storage {
    rows: Vec<Row>,
    sources: [Option<Option<[u8; 16]>>; MAX_SOURCES],
    attempted: u64,
}
impl Storage {
    fn row(&mut self, attempt: u64) -> Option<&mut Row> {
        let row = &mut self.rows[((attempt - 1) % MAX_FRAMES as u64) as usize];
        (row.attempt == attempt && !row.finished).then_some(row)
    }
}
/// Non-Send owner. Allocation happens before the first measured tick. A completed
/// Recording contains no TLS handle and may safely be retained by its host.
pub struct Capture(Rc<RefCell<Storage>>);
impl Capture {
    pub fn new() -> Result<Self, Error> {
        let mut rows = Vec::new();
        rows.try_reserve_exact(MAX_FRAMES)
            .map_err(|_| Error::Storage)?;
        rows.resize_with(MAX_FRAMES, Row::default);
        Ok(Self(Rc::new(RefCell::new(Storage {
            rows,
            sources: [None; MAX_SOURCES],
            attempted: 0,
        }))))
    }
    /// The outer Result is collector admission, the inner Result is the unchanged
    /// game result. Panic unwinding still retires TLS and records a failed frame.
    pub fn frame<T, E>(
        &mut self,
        tick: u64,
        run: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<T, E>, Error> {
        let attempt = ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active.is_some() {
                return Err(Error::SessionBusy);
            }
            let mut s = self.0.borrow_mut();
            let attempt = s.attempted.checked_add(1).ok_or(Error::Sequence)?;
            let index = ((attempt - 1) % MAX_FRAMES as u64) as usize;
            s.rows[index] = Row {
                attempt,
                tick,
                ..Row::default()
            };
            s.attempted = attempt;
            *active = Some(Active {
                storage: self.0.clone(),
                attempt,
            });
            Ok(attempt)
        })?;
        let mut guard = FrameGuard {
            storage: self.0.clone(),
            attempt,
            success: false,
        };
        let scope = begin(Phase::Frame, None);
        let result = run();
        drop(scope);
        guard.success = result.is_ok();
        drop(guard);
        Ok(result)
    }
    pub fn finish(self) -> Result<Recording, Error> {
        let s = Rc::try_unwrap(self.0)
            .map_err(|_| Error::SessionBusy)?
            .into_inner();
        Ok(Recording(s))
    }
}
struct FrameGuard {
    storage: Rc<RefCell<Storage>>,
    attempt: u64,
    success: bool,
}
impl Drop for FrameGuard {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_ref()
                .is_some_and(|a| a.attempt == self.attempt && Rc::ptr_eq(&a.storage, &self.storage))
            {
                *active = None;
            }
        });
        if let Some(row) = self.storage.borrow_mut().row(self.attempt) {
            row.finished = true;
            row.success = self.success && !std::thread::panicking();
            for metric in row.phases.iter_mut().chain(row.sources.iter_mut()) {
                metric.invalid |= metric.outstanding != 0;
            }
        }
    }
}
pub struct Recording(Storage);
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
fn duration(start: Instant, end: Instant) -> Option<u64> {
    u64::try_from(end.checked_duration_since(start)?.as_nanos()).ok()
}
fn sum(value: &mut u64, n: u64, invalid: &mut bool) {
    if let Some(next) = value.checked_add(n) {
        *value = next;
    } else {
        *invalid = true;
    }
}
impl Metric {
    fn start(&mut self) {
        self.invalid |= self.outstanding != 0;
        if let Some(next) = self.outstanding.checked_add(1) {
            self.outstanding = next;
        } else {
            self.invalid = true;
        }
    }
    fn end(&mut self, wall: Option<u64>, cpu: Option<u64>, unwound: bool) {
        self.outstanding = self.outstanding.saturating_sub(1);
        sum(&mut self.totals.calls, 1, &mut self.invalid);
        if let Some(wall) = wall {
            sum(&mut self.totals.wall_ns, wall, &mut self.invalid);
            self.totals.max_wall_ns = self.totals.max_wall_ns.max(wall);
        } else {
            self.invalid = true;
        }
        if let Some(cpu) = cpu {
            sum(&mut self.totals.thread_cpu_ns, cpu, &mut self.invalid);
        } else {
            sum(
                &mut self.totals.unavailable_cpu_samples,
                1,
                &mut self.invalid,
            );
        }
        if unwound {
            self.invalid = true;
            sum(&mut self.totals.unwound, 1, &mut self.invalid);
        }
    }
}
pub(crate) struct Scope(Option<Started>);
struct Started {
    storage: Weak<RefCell<Storage>>,
    attempt: u64,
    phase: Phase,
    source: Option<usize>,
    wall: Instant,
    cpu: Option<Duration>,
}
pub(crate) fn begin(phase: Phase, source: Option<Option<[u8; 16]>>) -> Scope {
    ACTIVE
        .try_with(|active| {
            let active = active.borrow();
            let Some(active) = active.as_ref() else {
                return Scope(None);
            };
            let mut s = active.storage.borrow_mut();
            let slot = source.and_then(|id| {
                if let Some(slot) = s.sources.iter().position(|v| *v == Some(id)) {
                    return Some(slot);
                }
                if let Some(slot) = s.sources.iter().position(Option::is_none) {
                    s.sources[slot] = Some(id);
                    return Some(slot);
                }
                None
            });
            let Some(row) = s.row(active.attempt) else {
                return Scope(None);
            };
            row.phases[phase as usize].start();
            if let Some(slot) = slot {
                row.sources[slot].start();
            } else if source.is_some() {
                if let Some(next) = row.dropped_sources.checked_add(1) {
                    row.dropped_sources = next;
                } else {
                    row.phases[phase as usize].invalid = true;
                }
            }
            Scope(Some(Started {
                storage: Rc::downgrade(&active.storage),
                attempt: active.attempt,
                phase,
                source: slot,
                wall: Instant::now(),
                cpu: cpu(),
            }))
        })
        .unwrap_or(Scope(None))
}
impl Drop for Scope {
    fn drop(&mut self) {
        let Some(start) = self.0.take() else {
            return;
        };
        let wall = duration(start.wall, Instant::now());
        let cpu = start
            .cpu
            .zip(cpu())
            .and_then(|(a, b)| b.checked_sub(a))
            .and_then(|v| u64::try_from(v.as_nanos()).ok());
        let Some(storage) = start.storage.upgrade() else {
            return;
        };
        let mut s = storage.borrow_mut();
        let Some(row) = s.row(start.attempt) else {
            return;
        };
        row.phases[start.phase as usize].end(wall, cpu, std::thread::panicking());
        if let Some(slot) = start.source {
            row.sources[slot].end(wall, cpu, std::thread::panicking());
        }
    }
}

#[cfg(test)]
mod tests;
