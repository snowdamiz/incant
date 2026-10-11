//! Rational wall-clock accumulator. No variable simulation dt and no unbounded
//! catch-up: excess elapsed ticks are counted and discarded, never simulated.
use std::time::{Duration, Instant};
pub const MAX_CATCH_UP: u128 = 4;
/// Claim a bounded retry opportunity. Incoming OS events must not move a future
/// deadline: a busy background window still needs periodic committed redraws.
pub fn claim_retry(deadline: &mut Instant, now: Instant) -> bool {
    if now < *deadline {
        return false;
    }
    *deadline = now + Duration::from_millis(50);
    true
}
pub struct Clock {
    rate: u32,
    last: Instant,
    phase: u128,
    pub dropped: u128,
}
impl Clock {
    pub fn new(rate: u32, now: Instant) -> Self {
        Self {
            rate,
            last: now,
            phase: 0,
            dropped: 0,
        }
    }
    pub fn reset(&mut self, now: Instant) {
        self.last = now;
        self.phase = 0;
    }
    pub fn due(&mut self, now: Instant) -> usize {
        self.phase += now.saturating_duration_since(self.last).as_nanos() * u128::from(self.rate);
        self.last = now;
        let ticks = self.phase / 1_000_000_000;
        self.phase %= 1_000_000_000;
        self.dropped += ticks.saturating_sub(MAX_CATCH_UP);
        ticks.min(MAX_CATCH_UP) as usize
    }
    pub fn deadline(&self) -> Instant {
        self.last
            + Duration::from_nanos(
                (1_000_000_000 - self.phase).div_ceil(u128::from(self.rate)) as u64
            )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_background_events_do_not_starve_pending_redraws() {
        let start = Instant::now();
        let mut retry = start + Duration::from_millis(50);
        let mut presented = Vec::new();
        // Every event requests another redraw. Throttling accepts at 50 ms
        // intervals even though no quiet 50 ms period ever occurs.
        for ms in 1..=210 {
            if claim_retry(&mut retry, start + Duration::from_millis(ms)) {
                presented.push(ms);
            }
        }
        assert_eq!(presented, [50, 100, 150, 200]);
        assert!(!claim_retry(&mut retry, start + Duration::from_millis(210)));
        assert!(claim_retry(&mut retry, start + Duration::from_secs(1)));
        assert!(!claim_retry(&mut retry, start + Duration::from_secs(1)));
    }
    #[test]
    fn fractional_ticks_have_no_per_tick_rounding_drift() {
        let start = Instant::now();
        let mut clock = Clock::new(144, start);
        let mut ticks = 0;
        for ms in 1..=10_000 {
            ticks += clock.due(start + Duration::from_millis(ms));
        }
        assert_eq!(ticks, 1440);
        assert_eq!(clock.dropped, 0);
    }
    #[test]
    fn overload_and_resume_are_bounded() {
        let start = Instant::now();
        let mut clock = Clock::new(60, start);
        assert_eq!(clock.due(start + Duration::from_secs(10)), 4);
        assert_eq!(clock.dropped, 596);
        clock.reset(start + Duration::from_secs(30));
        assert_eq!(clock.due(start + Duration::from_secs(30)), 0);
        assert_eq!(clock.due(start + Duration::from_secs(31)), 4);
    }
}
