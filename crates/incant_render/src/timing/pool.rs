use super::{GPU_TIMING_CAPACITY, GpuFrameTiming, GpuTimingError, GpuTimingStats};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
#[expect(
    clippy::large_enum_variant,
    reason = "Three fixed inline result slots avoid a heap allocation for every completed sample"
)]
enum Slot {
    Idle,
    Retired,
    Pending(u64),
    Ready(Result<GpuFrameTiming, GpuTimingError>),
}
struct State {
    slots: [Slot; GPU_TIMING_CAPACITY],
    sequence: u64,
    skipped: u64,
    abandoned: u64,
}
#[derive(Clone)]
pub(super) struct Pool(Arc<Mutex<State>>);
pub(super) struct Lease {
    pool: Pool,
    pub index: usize,
    pub sequence: u64,
    armed: bool,
}
impl Default for Pool {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State {
            slots: [Slot::Idle; GPU_TIMING_CAPACITY],
            sequence: 0,
            skipped: 0,
            abandoned: 0,
        })))
    }
}
impl Pool {
    pub fn reserve(&self) -> Result<Option<Lease>, GpuTimingError> {
        let mut state = self.0.lock().map_err(|_| GpuTimingError::State)?;
        state.sequence = state
            .sequence
            .checked_add(1)
            .ok_or(GpuTimingError::Sequence)?;
        let Some(index) = state.slots.iter().position(|s| matches!(s, Slot::Idle)) else {
            state.skipped = state.skipped.saturating_add(1);
            return Ok(None);
        };
        let sequence = state.sequence;
        state.slots[index] = Slot::Pending(sequence);
        Ok(Some(Lease {
            pool: self.clone(),
            index,
            sequence,
            armed: true,
        }))
    }
    pub fn take(&self) -> Result<Option<Result<GpuFrameTiming, GpuTimingError>>, GpuTimingError> {
        let mut state = self.0.lock().map_err(|_| GpuTimingError::State)?;
        for slot in &mut state.slots {
            if let Slot::Ready(result) = *slot {
                *slot = if result == Err(GpuTimingError::Readback) {
                    Slot::Retired
                } else {
                    Slot::Idle
                };
                return Ok(Some(result));
            }
        }
        Ok(None)
    }
    pub fn stats(&self) -> Result<GpuTimingStats, GpuTimingError> {
        let state = self.0.lock().map_err(|_| GpuTimingError::State)?;
        Ok(GpuTimingStats {
            enabled: true,
            pending: state
                .slots
                .iter()
                .filter(|s| matches!(s, Slot::Pending(_)))
                .count(),
            ready: state
                .slots
                .iter()
                .filter(|s| matches!(s, Slot::Ready(_)))
                .count(),
            skipped: state.skipped,
            abandoned: state.abandoned,
            retired: state
                .slots
                .iter()
                .filter(|s| matches!(s, Slot::Retired))
                .count(),
        })
    }
}
impl Lease {
    pub fn complete(mut self, result: Result<GpuFrameTiming, GpuTimingError>) {
        // Disarm before publishing: a consumer can reclaim and reuse the slot
        // before this callback's captured lease is dropped.
        self.armed = false;
        if let Ok(mut state) = self.pool.0.lock()
            && matches!(state.slots[self.index], Slot::Pending(n) if n == self.sequence)
        {
            state.slots[self.index] = Slot::Ready(result);
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.armed
            && let Ok(mut state) = self.pool.0.lock()
            && matches!(state.slots[self.index], Slot::Pending(n) if n == self.sequence)
        {
            state.slots[self.index] = Slot::Idle;
            state.abandoned = state.abandoned.saturating_add(1);
        }
    }
}
