//! One latest callback and one outstanding event, independent of callback rate.
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(super) struct Pulse {
    pub arrived: Instant,
    pub timestamp: f64,
    pub target: f64,
    pub duration: f64,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Inbox {
    latest: Option<Pulse>,
    queued: bool,
    pub closed: bool,
    pub invalid: bool,
    pub callbacks: u64,
    pub coalesced: u64,
    pub consumed: u64,
    pub last: Option<Pulse>,
    pub first: Option<Pulse>,
}
impl Inbox {
    /// Returns true only when the caller must enqueue one event. No allocation.
    pub fn publish(&mut self, pulse: Pulse) -> bool {
        if self.closed {
            return false;
        }
        self.invalid |= !pulse.timestamp.is_finite()
            || pulse.timestamp < 0.0
            || !pulse.target.is_finite()
            || pulse.target <= pulse.timestamp
            || !pulse.duration.is_finite()
            || pulse.duration < 0.0
            || self
                .last
                .is_some_and(|p| pulse.arrived < p.arrived || pulse.target <= p.target);
        self.callbacks = self.callbacks.saturating_add(1);
        if self.latest.is_some() {
            self.coalesced = self.coalesced.saturating_add(1);
        }
        self.first.get_or_insert(pulse);
        self.last = Some(pulse);
        self.latest = Some(pulse);
        let notify = !self.queued;
        self.queued = true;
        notify
    }
    pub fn take(&mut self) -> Option<Pulse> {
        let result = self.latest.take();
        if result.is_some() {
            self.consumed = self.consumed.saturating_add(1);
        }
        result
    }
    pub fn acknowledge(&mut self) {
        self.queued = false;
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.latest = None;
        self.queued = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn pulse(start: Instant, i: u32) -> Pulse {
        Pulse {
            arrived: start + Duration::from_millis(u64::from(i)),
            timestamp: f64::from(i),
            target: f64::from(i) + 0.016,
            duration: 0.016,
        }
    }
    #[test]
    fn burst_has_one_wake_and_retains_only_latest_callback() {
        let start = Instant::now();
        let mut q = Inbox::default();
        assert!(q.publish(pulse(start, 0)));
        for i in 1..1000 {
            assert!(!q.publish(pulse(start, i)));
        }
        assert_eq!(q.take().unwrap().timestamp, 999.0);
        assert_eq!((q.callbacks, q.coalesced, q.consumed), (1000, 999, 1));
        assert!(q.take().is_none());
        q.acknowledge();
        assert!(q.publish(pulse(start, 1000)));
        assert!(!q.invalid);
    }
    #[test]
    fn invalid_and_duplicate_timestamps_remain_sticky_across_good_callbacks() {
        let now = Instant::now();
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let mut q = Inbox::default();
            let mut p = pulse(now, 0);
            p.target = invalid;
            q.publish(p);
            q.take();
            q.publish(pulse(now, 1));
            assert!(q.invalid);
        }
        let mut q = Inbox::default();
        q.publish(pulse(now, 1));
        q.take();
        q.publish(pulse(now, 1));
        assert!(q.invalid);
    }
    #[test]
    fn closed_owner_never_queues_or_delivers_late_callbacks() {
        let mut q = Inbox::default();
        let p = pulse(Instant::now(), 0);
        q.publish(p);
        q.close();
        assert!(!q.publish(p));
        assert!(q.take().is_none());
        assert_eq!(q.callbacks, 1);
    }
    #[test]
    fn reading_latest_does_not_release_an_outstanding_event() {
        let start = Instant::now();
        let mut q = Inbox::default();
        assert!(q.publish(pulse(start, 0)));
        q.take();
        assert!(!q.publish(pulse(start, 1)));
        q.acknowledge();
        assert!(q.publish(pulse(start, 2)));
    }
}
