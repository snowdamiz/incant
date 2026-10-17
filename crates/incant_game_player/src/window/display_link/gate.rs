//! A display callback permits scheduling; input and watchdog events do not.
use super::inbox::Inbox;
use std::time::{Duration, Instant};

const WATCHDOG: Duration = Duration::from_millis(500);
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidOrClosed,
    MissingCallback,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidOrClosed => "invalid or closed native display wake",
            Self::MissingCallback => "native display wake missed its 500ms watchdog",
        })
    }
}
impl std::error::Error for Error {}

pub(super) struct Gate {
    active_since: Option<Instant>,
    started: Instant,
    eligible_after: Instant,
    notified: bool,
    pub gameplay_pulses: u64,
}
impl Gate {
    pub fn new(started: Instant) -> Self {
        Self {
            active_since: None,
            started,
            eligible_after: started,
            notified: false,
            gameplay_pulses: 0,
        }
    }
    pub fn boundary(&mut self, inbox: &mut Inbox, now: Instant) {
        // Discard stale data without freeing a still-queued Tao event token.
        inbox.take();
        self.notified = false;
        self.active_since = None;
        self.eligible_after = now;
    }
    pub fn acknowledge(&mut self, inbox: &mut Inbox) {
        inbox.acknowledge();
        self.notified = true;
    }
    pub fn poll(&mut self, inbox: &mut Inbox, now: Instant, active: bool) -> Result<bool, Error> {
        if active {
            self.active_since.get_or_insert(now);
        } else {
            self.active_since = None;
            self.eligible_after = now;
        }
        let pulse = if self.notified { inbox.take() } else { None };
        self.notified = false;
        if inbox.closed || inbox.invalid {
            return Err(Error::InvalidOrClosed);
        }
        if active && now >= self.deadline(inbox) {
            return Err(Error::MissingCallback);
        }
        let ready = active && pulse.is_some_and(|p| p.arrived >= self.eligible_after);
        if ready {
            self.gameplay_pulses = self.gameplay_pulses.saturating_add(1);
        }
        Ok(ready)
    }
    pub fn deadline(&self, inbox: &Inbox) -> Instant {
        let origin = self.active_since.unwrap_or(self.started);
        inbox.last.map_or(origin, |p| p.arrived.max(origin)) + WATCHDOG
    }
}

#[cfg(test)]
mod tests {
    use super::super::inbox::Pulse;
    use super::*;
    #[test]
    fn continuous_os_events_do_not_authorize_simulation_or_postpone_watchdog() {
        let start = Instant::now();
        let mut q = Inbox::default();
        let mut g = Gate::new(start);
        for ms in 0..500 {
            assert!(
                !g.poll(&mut q, start + Duration::from_millis(ms), true)
                    .unwrap()
            );
        }
        assert_eq!(
            g.poll(&mut q, start + WATCHDOG, true),
            Err(Error::MissingCallback)
        );
        assert_eq!(g.gameplay_pulses, 0);
    }
    #[test]
    fn only_delivered_callback_authorizes_one_schedule_opportunity() {
        let now = Instant::now();
        let mut q = Inbox::default();
        let mut g = Gate::new(now);
        // Target presentation time is deliberately unrelated to host time.
        q.publish(Pulse {
            arrived: now,
            timestamp: 12345.0,
            target: 12346.0,
            duration: 1.0 / 60.0,
        });
        assert!(!g.poll(&mut q, now, true).unwrap());
        g.acknowledge(&mut q);
        assert!(g.poll(&mut q, now, true).unwrap());
        assert!(!g.poll(&mut q, now, true).unwrap());
        assert_eq!(g.deadline(&q), now + WATCHDOG);
        assert_eq!(g.gameplay_pulses, 1);
    }
    #[test]
    fn queued_background_callback_cannot_authorize_same_batch_focus_gain() {
        let start = Instant::now();
        let mut q = Inbox::default();
        let mut g = Gate::new(start);
        g.poll(&mut q, start, true).unwrap();
        // Loss, callback delivery and focus gain all precede the next poll.
        g.boundary(&mut q, start + Duration::from_millis(1));
        assert!(q.publish(Pulse {
            arrived: start + Duration::from_millis(2),
            timestamp: 1.0,
            target: 2.0,
            duration: 0.016
        }));
        g.acknowledge(&mut q);
        g.boundary(&mut q, start + Duration::from_millis(3));
        assert!(
            !g.poll(&mut q, start + Duration::from_millis(4), true)
                .unwrap()
        );
        assert_eq!(g.gameplay_pulses, 0);
    }
    #[test]
    fn stale_queued_event_can_deliver_only_a_new_post_boundary_callback() {
        let start = Instant::now();
        let mut q = Inbox::default();
        let mut g = Gate::new(start);
        assert!(q.publish(Pulse {
            arrived: start,
            timestamp: 1.0,
            target: 2.0,
            duration: 0.016
        }));
        g.boundary(&mut q, start + Duration::from_millis(1));
        // The old event token is still outstanding, so no second token is sent.
        assert!(!q.publish(Pulse {
            arrived: start + Duration::from_millis(2),
            timestamp: 2.0,
            target: 3.0,
            duration: 0.016
        }));
        assert!(
            !g.poll(&mut q, start + Duration::from_millis(3), true)
                .unwrap()
        );
        g.acknowledge(&mut q);
        assert!(
            g.poll(&mut q, start + Duration::from_millis(3), true)
                .unwrap()
        );
        assert!(
            !g.poll(&mut q, start + Duration::from_millis(4), true)
                .unwrap()
        );
    }
    #[test]
    fn background_callback_is_drained_and_resume_starts_new_health_window() {
        let start = Instant::now();
        let mut q = Inbox::default();
        let mut g = Gate::new(start);
        g.poll(&mut q, start, true).unwrap();
        q.publish(Pulse {
            arrived: start,
            timestamp: 1.0,
            target: 2.0,
            duration: 1.0 / 60.0,
        });
        g.acknowledge(&mut q);
        assert!(!g.poll(&mut q, start, false).unwrap());
        let resume = start + Duration::from_secs(30);
        assert!(!g.poll(&mut q, resume, true).unwrap());
        assert_eq!(g.deadline(&q), resume + WATCHDOG);
        assert_eq!(
            g.poll(&mut q, resume + WATCHDOG, true),
            Err(Error::MissingCallback)
        );
        q.close();
        assert_eq!(g.poll(&mut q, resume, false), Err(Error::InvalidOrClosed));
    }
}
