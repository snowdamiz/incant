//! Fixed capture-owned histogram storage; collection never allocates.
use crate::*;
use std::thread::{self, ThreadId};

#[derive(Clone, Copy, Default)]
struct Row {
    samples: u64,
    missing: u64,
    invalid: u64,
    overflow: u64,
}
struct Active {
    id: u64,
    thread: ThreadId,
    sums: [u64; PHASE_COUNT],
    calls: [u64; PHASE_COUNT],
    outstanding: [u32; PHASE_COUNT],
    invalid: [bool; PHASE_COUNT],
    collision: bool,
}
pub(super) struct Frames {
    bins: Vec<u64>,
    rows: [Row; PHASE_COUNT],
    attempted: u64,
    successful: u64,
    failed: u64,
    invalid: u64,
    saturated: bool,
    active: Option<Active>,
    live: [u64; PHASE_COUNT],
}
fn increment(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *saturated = true;
    }
}
impl Frames {
    #[cfg(feature = "diagnostic-script-phases")]
    pub(crate) fn diagnostic_frame(&self) -> Option<u64> {
        self.active.as_ref().and_then(|active| {
            (active.thread == thread::current().id() && !active.collision).then_some(active.id)
        })
    }
    pub const fn empty() -> Self {
        const ZERO: Row = Row {
            samples: 0,
            missing: 0,
            invalid: 0,
            overflow: 0,
        };
        Self {
            bins: Vec::new(),
            rows: [ZERO; PHASE_COUNT],
            attempted: 0,
            successful: 0,
            failed: 0,
            invalid: 0,
            saturated: false,
            active: None,
            live: [0; PHASE_COUNT],
        }
    }
    pub fn allocated() -> Result<Self, Error> {
        let mut result = Self::empty();
        // Allocate directly on the heap, never a multi-megabyte stack temporary.
        result
            .bins
            .try_reserve_exact(PHASE_COUNT * FRAME_BUCKET_COUNT)
            .map_err(|_| Error::Storage)?;
        result.bins.resize(PHASE_COUNT * FRAME_BUCKET_COUNT, 0);
        Ok(result)
    }
    pub fn begin(&mut self) -> Option<u64> {
        increment(&mut self.attempted, &mut self.saturated);
        if let Some(active) = &mut self.active {
            active.collision = true;
            return None;
        }
        if self.saturated {
            return None;
        }
        let id = self.attempted;
        self.active = Some(Active {
            id,
            thread: thread::current().id(),
            sums: [0; PHASE_COUNT],
            calls: [0; PHASE_COUNT],
            outstanding: [0; PHASE_COUNT],
            invalid: self.live.map(|count| count != 0),
            collision: false,
        });
        Some(id)
    }
    pub fn begin_scope(&mut self, phase: Phase) -> Option<u64> {
        let i = phase as usize;
        let overlapping = self.live[i] != 0;
        increment(&mut self.live[i], &mut self.saturated);
        let active = self.active.as_mut()?;
        active.invalid[i] |= overlapping;
        if active.thread != thread::current().id() {
            active.invalid[i] = true;
            return None;
        }
        if active.outstanding[i] != 0 {
            active.invalid[i] = true;
        }
        let Some(next) = active.outstanding[i].checked_add(1) else {
            active.invalid[i] = true;
            return None;
        };
        active.outstanding[i] = next;
        increment(&mut active.calls[i], &mut self.saturated);
        Some(active.id)
    }
    pub fn end_scope(&mut self, id: Option<u64>, phase: Phase, ns: u64, unwound: bool) {
        self.live[phase as usize] = self.live[phase as usize].saturating_sub(1);
        let Some(id) = id else {
            return;
        };
        let Some(active) = &mut self.active else {
            return;
        };
        if active.id != id {
            return;
        }
        let i = phase as usize;
        active.outstanding[i] = active.outstanding[i].saturating_sub(1);
        if let Some(sum) = active.sums[i].checked_add(ns) {
            active.sums[i] = sum;
        } else {
            active.invalid[i] = true;
            self.saturated = true;
        }
        active.invalid[i] |= unwound;
    }
    pub fn finish(&mut self, id: Option<u64>, success: bool) {
        if success {
            increment(&mut self.successful, &mut self.saturated);
        } else {
            increment(&mut self.failed, &mut self.saturated);
        }
        let Some(id) = id else {
            increment(&mut self.invalid, &mut self.saturated);
            return;
        };
        let Some(active) = self.active.take() else {
            increment(&mut self.invalid, &mut self.saturated);
            return;
        };
        assert_eq!(
            id, active.id,
            "private frame guards cannot replace an active frame"
        );
        if active.collision {
            increment(&mut self.invalid, &mut self.saturated);
        }
        if !success {
            return;
        }
        for i in 0..PHASE_COUNT {
            let row = &mut self.rows[i];
            if active.collision || active.invalid[i] || active.outstanding[i] != 0 {
                increment(&mut row.invalid, &mut self.saturated);
            } else if active.calls[i] == 0 {
                increment(&mut row.missing, &mut self.saturated);
            } else if active.sums[i] > FRAME_MAX_NS {
                increment(&mut row.overflow, &mut self.saturated);
            } else {
                let bin = active.sums[i].div_ceil(FRAME_BUCKET_WIDTH_NS) as usize;
                increment(
                    &mut self.bins[i * FRAME_BUCKET_COUNT + bin],
                    &mut self.saturated,
                );
                increment(&mut row.samples, &mut self.saturated);
            }
        }
    }
    pub fn report(&self) -> FrameWallReport {
        let complete = self.attempted > 0
            && self.attempted == self.successful
            && self.failed == 0
            && self.invalid == 0
            && !self.saturated
            && self.active.is_none();
        FrameWallReport {
            version: 1,
            unit: "ns",
            bucket_width_ns: FRAME_BUCKET_WIDTH_NS,
            max_ns: FRAME_MAX_NS,
            attempted_frames: self.attempted,
            successful_frames: self.successful,
            failed_frames: self.failed,
            invalid_frames: self.invalid,
            active_frames: u64::from(self.active.is_some()),
            saturated: self.saturated,
            phases: Phase::ALL
                .into_iter()
                .enumerate()
                .map(|(i, phase)| {
                    let row = self.rows[i];
                    let counts =
                        self.bins[i * FRAME_BUCKET_COUNT..(i + 1) * FRAME_BUCKET_COUNT].to_vec();
                    let bounds = (complete
                        && row.samples == self.successful
                        && row.missing == 0
                        && row.invalid == 0
                        && row.overflow == 0)
                        .then(|| percentile(&counts, row.samples))
                        .flatten();
                    FramePhaseReport {
                        name: phase.name(),
                        samples: row.samples,
                        missing_frames: row.missing,
                        invalid_frames: row.invalid,
                        overflow_samples: row.overflow,
                        p95_lower_ns: bounds.map(|b| b.0),
                        p95_upper_ns: bounds.map(|b| b.1),
                        counts,
                    }
                })
                .collect(),
        }
    }
}
fn percentile(counts: &[u64], samples: u64) -> Option<(u64, u64)> {
    if samples == 0 {
        return None;
    }
    let rank = (u128::from(samples) * 95).div_ceil(100);
    let mut total = 0_u128;
    for (i, count) in counts.iter().enumerate() {
        total += u128::from(*count);
        if total >= rank {
            let upper = i as u64 * FRAME_BUCKET_WIDTH_NS;
            let lower = if i == 0 {
                0
            } else {
                upper - FRAME_BUCKET_WIDTH_NS + 1
            };
            return Some((lower, upper));
        }
    }
    None
}
#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
