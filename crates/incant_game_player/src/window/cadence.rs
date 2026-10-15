//! Optional CPU host observations, not GPU completion or display timestamps.
//! Storage is allocated once when the host has an active profile capture.
use incant_profile::{FRAME_BUCKET_COUNT, FRAME_BUCKET_WIDTH_NS, FRAME_MAX_NS};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Serialize)]
struct Histogram {
    samples: u64,
    overflow_samples: u64,
    saturated: bool,
    counts: Vec<u64>,
}
impl Histogram {
    fn new() -> Self {
        Self {
            samples: 0,
            overflow_samples: 0,
            saturated: false,
            counts: vec![0; FRAME_BUCKET_COUNT],
        }
    }
    fn increment(value: &mut u64, saturated: &mut bool) {
        match value.checked_add(1) {
            Some(next) => *value = next,
            None => *saturated = true,
        }
    }
    fn record(&mut self, elapsed: Duration) {
        Self::increment(&mut self.samples, &mut self.saturated);
        let ns = elapsed.as_nanos();
        if ns > u128::from(FRAME_MAX_NS) {
            Self::increment(&mut self.overflow_samples, &mut self.saturated);
        } else {
            let bucket = (ns as u64).div_ceil(FRAME_BUCKET_WIDTH_NS) as usize;
            Self::increment(&mut self.counts[bucket], &mut self.saturated);
        }
    }
    fn percentile(&self) -> Option<[u64; 2]> {
        if self.samples == 0 || self.saturated {
            return None;
        }
        let rank = (u128::from(self.samples) * 95).div_ceil(100);
        let mut cumulative = 0u128;
        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += u128::from(count);
            if cumulative >= rank {
                return Some(if i == 0 {
                    [0, 0]
                } else {
                    [
                        (i as u64 - 1) * FRAME_BUCKET_WIDTH_NS + 1,
                        i as u64 * FRAME_BUCKET_WIDTH_NS,
                    ]
                });
            }
        }
        // The p95 lies above the finite histogram range. Never clamp to 100ms.
        None
    }
}
struct State {
    host_wall: Histogram,
    accepted_interval: Histogram,
    previous_end: Option<Instant>,
    interrupted_segments: u64,
    invalid_samples: u64,
    saturated: bool,
}
impl State {
    fn new() -> Self {
        Self {
            host_wall: Histogram::new(),
            accepted_interval: Histogram::new(),
            previous_end: None,
            interrupted_segments: 0,
            invalid_samples: 0,
            saturated: false,
        }
    }
    fn interrupt(&mut self) {
        if self.previous_end.take().is_some() {
            Histogram::increment(&mut self.interrupted_segments, &mut self.saturated);
        }
    }
    fn finish(&mut self, started: Instant, ended: Instant) {
        let elapsed = ended.checked_duration_since(started);
        let interval = self
            .previous_end
            .map(|last| ended.checked_duration_since(last));
        if elapsed.is_none() || matches!(interval, Some(None)) {
            Histogram::increment(&mut self.invalid_samples, &mut self.saturated);
            self.interrupt();
            return;
        }
        self.host_wall.record(elapsed.unwrap());
        if let Some(Some(interval)) = interval {
            self.accepted_interval.record(interval);
        }
        self.previous_end = Some(ended);
    }
}
pub(super) struct Cadence(Option<Box<State>>);
impl Cadence {
    pub fn new() -> Self {
        Self(incant_profile::enabled().then(|| Box::new(State::new())))
    }
    pub fn begin(&self) -> Option<Instant> {
        self.0.as_ref().map(|_| Instant::now())
    }
    pub fn finish(&mut self, started: Option<Instant>) {
        if let (Some(state), Some(started)) = (&mut self.0, started) {
            state.finish(started, Instant::now());
        }
    }
    pub fn interrupt(&mut self) {
        if let Some(state) = &mut self.0 {
            state.interrupt();
        }
    }
    pub fn append(&self, report: &mut serde_json::Value) {
        if let Some(state) = &self.0 {
            report["host_cadence"] = serde_json::json!({
                "version": 1,
                "unit": "nanoseconds",
                "timestamp_domain": "cpu_monotonic_host_completion",
                "display_or_gpu_completion": false,
                "bucket_width_ns": FRAME_BUCKET_WIDTH_NS,
                "max_ns": FRAME_MAX_NS,
                "host_wall": state.host_wall,
                "host_wall_p95_interval_ns": state.host_wall.percentile(),
                "accepted_interval": state.accepted_interval,
                "accepted_interval_p95_interval_ns": state.accepted_interval.percentile(),
                "interrupted_segments": state.interrupted_segments,
                "invalid_samples": state.invalid_samples,
                "saturated": state.saturated,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_intervals_include_waits_but_never_bridge_suspension_or_retry() {
        let t = Instant::now();
        let ms = |n| t + Duration::from_millis(n);
        let mut state = State::new();
        state.finish(ms(0), ms(3));
        state.finish(ms(16), ms(20));
        assert_eq!(state.host_wall.samples, 2);
        assert_eq!(state.host_wall.percentile(), Some([3_990_001, 4_000_000]));
        assert_eq!(
            state.accepted_interval.percentile(),
            Some([16_990_001, 17_000_000])
        );
        state.interrupt();
        state.interrupt();
        state.finish(ms(1000), ms(1005));
        assert_eq!(state.host_wall.samples, 3);
        assert_eq!(state.accepted_interval.samples, 1);
        assert_eq!(state.interrupted_segments, 1);
    }

    #[test]
    fn histogram_retains_zero_upper_boundary_and_overflow() {
        let mut histogram = Histogram::new();
        for ns in [
            0,
            1,
            FRAME_BUCKET_WIDTH_NS,
            FRAME_BUCKET_WIDTH_NS + 1,
            FRAME_MAX_NS,
        ] {
            histogram.record(Duration::from_nanos(ns));
        }
        assert_eq!(histogram.counts[0..3], [1, 2, 1]);
        assert_eq!(
            histogram.percentile(),
            Some([FRAME_MAX_NS - FRAME_BUCKET_WIDTH_NS + 1, FRAME_MAX_NS])
        );
        histogram.record(Duration::from_nanos(FRAME_MAX_NS + 1));
        assert_eq!(histogram.overflow_samples, 1);
        assert_eq!(histogram.percentile(), None);
        histogram.samples = u64::MAX;
        histogram.record(Duration::ZERO);
        assert!(histogram.saturated);
        assert_eq!(histogram.percentile(), None);
    }

    #[test]
    fn invalid_clock_observations_break_the_segment_and_are_not_successes() {
        let t = Instant::now();
        let mut state = State::new();
        state.finish(t, t + Duration::from_millis(2));
        state.finish(t + Duration::from_millis(5), t + Duration::from_millis(4));
        state.finish(t, t + Duration::from_millis(3));
        assert_eq!(state.invalid_samples, 1);
        assert_eq!(state.host_wall.samples, 2);
        assert_eq!(state.accepted_interval.samples, 0);
        assert_eq!(state.interrupted_segments, 1);
    }

    #[test]
    fn absent_capture_has_no_storage_timestamp_or_report() {
        let mut cadence = Cadence(None);
        assert!(cadence.begin().is_none());
        cadence.finish(None);
        cadence.interrupt();
        let mut report = serde_json::json!({});
        cadence.append(&mut report);
        assert_eq!(report, serde_json::json!({}));
    }
}
