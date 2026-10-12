//! One accepted tick per acquired surface, at most four ticks of pending work.
//! Paused/unavailable time is reset; overload is discarded and counted.
use std::time::Instant;
pub struct Clock {
    last: Instant,
    phase: u128,
    pub discarded: u64,
}
impl Clock {
    pub fn new(now: Instant) -> Self {
        Self {
            last: now,
            phase: 0,
            discarded: 0,
        }
    }
    pub fn reset(&mut self, now: Instant) {
        self.last = now;
        self.phase = 0;
    }
    pub fn due(&mut self, now: Instant, rate: u32) -> bool {
        const UNIT: u128 = 1_000_000_000;
        self.phase = self.phase.saturating_add(
            now.saturating_duration_since(self.last)
                .as_nanos()
                .saturating_mul(u128::from(rate)),
        );
        self.last = now;
        let excess = (self.phase / UNIT).saturating_sub(4);
        self.discarded = self
            .discarded
            .saturating_add(u64::try_from(excess).unwrap_or(u64::MAX));
        self.phase -= excess * UNIT;
        if self.phase >= UNIT {
            self.phase -= UNIT;
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn rational_ticks_and_bounded_overload_do_not_hide_discarded_time() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let mut ticks = 0;
        for ms in 1..=10000 {
            ticks += usize::from(clock.due(start + Duration::from_millis(ms), 144));
        }
        assert_eq!(ticks, 1440);
        assert_eq!(clock.discarded, 0);
        let later = start + Duration::from_secs(20);
        assert!(clock.due(later, 60));
        assert_eq!(clock.discarded, 596);
        for _ in 0..3 {
            assert!(clock.due(later, 60));
        }
        assert!(!clock.due(later, 60));
        clock.reset(start + Duration::from_secs(30));
        assert!(!clock.due(start + Duration::from_secs(30), 60));
        assert_eq!(clock.discarded, 596);
    }
}
