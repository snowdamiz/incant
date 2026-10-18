//! Bounded host identities, independent from callback timestamps and world tick.
use super::boundaries::{Acquisition, AcquisitionOutcome, PulseWake};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::window) enum Role {
    Initial,
    Background,
    UiResize,
    Gameplay,
    Committed,
    Simulation,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[repr(u8)]
pub(in crate::window) enum Boundary {
    Start,
    Focus,
    Suspend,
    Resume,
    Resize,
    Configure,
    Recreate,
    Retry,
    Restart,
    Role,
    Inactive,
}
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(in crate::window) enum Failure {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
    UnsupportedSize,
    Minimized,
    Reservation,
    Device,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::window) enum Status {
    Open,
    Deferred,
    RefreshCompleted,
    HostCommitted,
    Admitted,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Attempt {
    pub id: u64,
    pub role: Role,
    pub session_generation: u64,
    pub requested_host_step: u64,
    pub status: Status,
    pub host_timing: HostTiming,
    /// The drawable context keeps the first requested step. This receipt binds
    /// it to every accepted step and the final displayed state, without editing
    /// callback identities after acquisition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<Batch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<Display>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::window) enum DisplayReason {
    Pulse,
    Stop,
    Restart,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Display {
    pub reason: DisplayReason,
    pub callback_sequence: Option<u64>,
    pub callback_arrived_ns: Option<u64>,
    /// Native target metadata has no shared epoch with Rust Instant.
    pub callback_target_seconds: Option<f64>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Batch {
    pub planned_ticks: u8,
    pub accepted_ticks: u8,
    pub displayed_host_step: Option<u64>,
}
/// One host clock only. These values never share an epoch with Metal timestamps.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub(super) struct HostTiming {
    pub began_ns: Option<u64>,
    pub reserved_ns: Option<u64>,
    pub completed_ns: Option<u64>,
    pub finished_ns: Option<u64>,
    pub schedule: Option<Schedule>,
    pub wake: Option<PulseWake>,
    pub acquisitions: [Option<Acquisition>; 2],
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Schedule {
    pub pulse_ns: Option<u64>,
    pub deadline_ns: Option<u64>,
    pub forced_resume: bool,
    pub due: u8,
    pub ordinal: u8,
    pub discarded_ticks: Option<u64>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Segment {
    pub id: u64,
    pub surface_generation: u64,
    pub session_generation: u64,
    pub role: Role,
    pub boundary_bits: u16,
}
const _: () = assert!(std::mem::size_of::<Attempt>() <= 512);
const _: () = assert!(std::mem::size_of::<Segment>() <= 40);

#[derive(Serialize)]
pub(super) struct Model {
    pub attempts: Vec<Attempt>,
    pub segments: Vec<Segment>,
    pub boundary_counts: [u64; 11],
    pub acquisition_failures: [u64; 9],
    pub omitted_attempts: u64,
    pub exhausted: bool,
    pub timing_errors: u64,
    #[serde(skip)]
    capacity: usize,
    #[serde(skip)]
    active: Option<usize>,
    #[serde(skip)]
    finish_pending: Option<usize>,
    #[serde(skip)]
    pending: u16,
    #[serde(skip)]
    surface_generation: u64,
    #[serde(skip)]
    last_display_role: Option<Role>,
    #[serde(skip)]
    last_display_callback: Option<u64>,
}
impl Model {
    pub fn new(capacity: usize) -> Result<Self, std::collections::TryReserveError> {
        let mut attempts = Vec::new();
        let mut segments = Vec::new();
        attempts.try_reserve_exact(capacity)?;
        // Bounded acquisition has at most two tries, hence two segments/attempt.
        segments.try_reserve_exact(capacity * 2)?;
        let mut value = Self {
            attempts,
            segments,
            boundary_counts: [0; 11],
            acquisition_failures: [0; 9],
            omitted_attempts: 0,
            exhausted: false,
            timing_errors: 0,
            capacity,
            active: None,
            finish_pending: None,
            pending: 0,
            surface_generation: 1,
            last_display_role: None,
            last_display_callback: None,
        };
        value.boundary(Boundary::Start);
        Ok(value)
    }
    pub fn boundary(&mut self, reason: Boundary) {
        self.boundary_counts[reason as usize] =
            self.boundary_counts[reason as usize].saturating_add(1);
        self.pending |= 1 << (reason as u8);
    }
    pub fn recreated(&mut self) {
        self.boundary(Boundary::Recreate);
        match self.surface_generation.checked_add(1) {
            Some(next) => self.surface_generation = next,
            None => self.exhausted = true,
        }
    }
    pub fn begin(
        &mut self,
        role: Role,
        session_generation: u64,
        requested_host_step: u64,
        began_ns: Option<u64>,
    ) {
        self.active = None;
        self.finish_pending = None;
        if self.attempts.len() == self.capacity || self.exhausted || session_generation == 0 {
            self.exhausted = true;
            self.omitted_attempts = self.omitted_attempts.saturating_add(1);
            return;
        }
        if role != Role::Simulation {
            if self.last_display_role.is_some_and(|last| last != role) {
                self.boundary(Boundary::Role);
            }
            self.last_display_role = Some(role);
        }
        if self
            .attempts
            .last()
            .is_some_and(|a| a.session_generation != session_generation)
        {
            self.boundary(Boundary::Restart);
        }
        self.active = Some(self.attempts.len());
        self.attempts.push(Attempt {
            id: self.attempts.len() as u64 + 1,
            role,
            session_generation,
            requested_host_step,
            status: Status::Open,
            host_timing: HostTiming {
                began_ns,
                ..HostTiming::default()
            },
            batch: None,
            display: None,
        });
        if began_ns.is_none() {
            self.timing_errors = self.timing_errors.saturating_add(1);
        }
    }
    pub fn schedule(&mut self, schedule: Schedule) {
        let Some(i) = self.active else { return };
        let timing = &self.attempts[i].host_timing;
        let valid = matches!(self.attempts[i].role, Role::Gameplay | Role::Simulation)
            && timing.schedule.is_none()
            && (1..=4).contains(&schedule.due)
            && schedule.ordinal < schedule.due
            && schedule.pulse_ns.is_some()
            && schedule.discarded_ticks.is_some()
            && schedule.forced_resume == schedule.deadline_ns.is_none()
            && schedule
                .pulse_ns
                .zip(timing.began_ns)
                .is_some_and(|(p, b)| p <= b)
            && schedule
                .deadline_ns
                .zip(schedule.pulse_ns)
                .is_none_or(|(d, p)| d <= p);
        if !valid {
            self.timing_errors = self.timing_errors.saturating_add(1);
        }
        // Retain the first observation, including invalid/missing fields. The
        // error count prevents partial metadata from becoming a timing verdict.
        if timing.schedule.is_none() {
            self.attempts[i].host_timing.schedule = Some(schedule);
        }
    }
    pub fn display(&mut self, display: Display) {
        let Some(i) = self.active else { return };
        let a = &self.attempts[i];
        let valid = a.role == Role::Committed
            && a.display.is_none()
            && if display.reason == DisplayReason::Pulse {
                display.callback_sequence.is_some_and(|n| {
                    n > 0 && self.last_display_callback.is_none_or(|last| n > last)
                }) && display
                    .callback_arrived_ns
                    .zip(a.host_timing.began_ns)
                    .is_some_and(|(p, b)| p <= b)
                    && display
                        .callback_target_seconds
                        .is_some_and(|t| t.is_finite() && t > 0.0)
            } else {
                display.callback_sequence.is_none()
                    && display.callback_arrived_ns.is_none()
                    && display.callback_target_seconds.is_none()
            };
        if !valid {
            self.timing_errors = self.timing_errors.saturating_add(1);
        } else if display.reason == DisplayReason::Pulse {
            self.last_display_callback = display.callback_sequence;
        }
        if a.display.is_none() {
            self.attempts[i].display = Some(display);
        }
    }
    pub fn wake(&mut self, wake: Option<PulseWake>) {
        let Some(i) = self.active else { return };
        self.attempts[i].host_timing.wake = wake;
    }
    pub fn batch_begin(&mut self, due: usize) {
        let Some(i) = self.active else { return };
        let a = &mut self.attempts[i];
        if a.role != Role::Gameplay
            || a.batch.is_some()
            || !(1..=4).contains(&due)
            || a.host_timing
                .schedule
                .is_none_or(|s| s.ordinal != 0 || usize::from(s.due) != due)
        {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        }
        a.batch = Some(Batch {
            planned_ticks: due as u8,
            accepted_ticks: 0,
            displayed_host_step: None,
        });
    }
    pub fn batch_accept(&mut self, step: u64, displayed: bool) {
        let Some(i) = self.active else {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        };
        let a = &mut self.attempts[i];
        let Some(batch) = &mut a.batch else {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        };
        if batch.displayed_host_step.is_some()
            || batch.accepted_ticks >= batch.planned_ticks
            || a.requested_host_step
                .checked_add(u64::from(batch.accepted_ticks))
                != Some(step)
            || a.host_timing.reserved_ns.is_none()
        {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        }
        batch.accepted_ticks += 1;
        if displayed {
            batch.displayed_host_step = Some(step);
        }
    }
    pub fn acquisition_begin(&mut self, at_ns: Option<u64>) {
        let Some(i) = self.active else { return };
        let timing = self.attempts[i].host_timing;
        let Some(slot) = timing.acquisitions.iter().position(Option::is_none) else {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        };
        if slot > 0 && timing.acquisitions[slot - 1].is_some_and(|a| a.outcome.is_none()) {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        }
        let previous = if slot == 0 {
            timing.began_ns
        } else {
            timing.acquisitions[slot - 1].and_then(|a| a.ended_ns)
        };
        let began_ns = self.checked_time(previous, at_ns);
        self.attempts[i].host_timing.acquisitions[slot] = Some(Acquisition {
            began_ns,
            ended_ns: None,
            outcome: None,
        });
    }
    pub fn acquisition_end(&mut self, at_ns: Option<u64>, outcome: AcquisitionOutcome) {
        let Some(i) = self.active else { return };
        let timing = self.attempts[i].host_timing;
        let Some(slot) = timing
            .acquisitions
            .iter()
            .position(|a| a.is_some_and(|a| a.outcome.is_none()))
        else {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        };
        let mut acquisition = timing.acquisitions[slot].unwrap();
        acquisition.ended_ns = self.checked_time(acquisition.began_ns, at_ns);
        acquisition.outcome = Some(outcome);
        self.attempts[i].host_timing.acquisitions[slot] = Some(acquisition);
    }
    pub fn reserved(&mut self, at_ns: Option<u64>) {
        let Some(i) = self.active else { return };
        let timing = self.attempts[i].host_timing;
        if timing.reserved_ns.is_some() {
            self.timing_errors = self.timing_errors.saturating_add(1);
            return;
        }
        let checked = self.checked_time(timing.began_ns, at_ns);
        self.attempts[i].host_timing.reserved_ns = checked;
    }
    fn checked_time(&mut self, previous: Option<u64>, current: Option<u64>) -> Option<u64> {
        if current.is_none() || previous.zip(current).is_some_and(|(a, b)| b < a) {
            self.timing_errors = self.timing_errors.saturating_add(1);
            None
        } else {
            current
        }
    }
    /// Called just before each native acquisition, after configure/retry boundaries.
    pub fn context(&mut self) -> Option<(u64, u64, u64, u64)> {
        let a = self.attempts.get(self.active?)?;
        if self.exhausted || a.role == Role::Simulation {
            if a.role == Role::Simulation {
                self.timing_errors = self.timing_errors.saturating_add(1);
            }
            return None;
        }
        if self.pending != 0 || self.segments.is_empty() {
            if self.segments.len() == self.capacity * 2 {
                self.exhausted = true;
                return None;
            }
            self.segments.push(Segment {
                id: self.segments.len() as u64 + 1,
                surface_generation: self.surface_generation,
                session_generation: a.session_generation,
                role: a.role,
                boundary_bits: self.pending,
            });
            self.pending = 0;
        }
        Some((
            self.surface_generation,
            self.segments.last()?.id,
            a.requested_host_step,
            a.id,
        ))
    }
    pub fn failure(&mut self, failure: Failure) {
        self.acquisition_failures[failure as usize] =
            self.acquisition_failures[failure as usize].saturating_add(1);
    }
    pub fn complete(&mut self, status: Status, at_ns: Option<u64>) {
        if let Some(i) = self.active.take() {
            if let Some(batch) = self.attempts[i].batch {
                let valid = match status {
                    Status::HostCommitted => {
                        batch.accepted_ticks > 0 && batch.displayed_host_step.is_some()
                    }
                    Status::Deferred => {
                        batch.accepted_ticks == 0 && batch.displayed_host_step.is_none()
                    }
                    _ => false,
                };
                if !valid {
                    self.timing_errors = self.timing_errors.saturating_add(1);
                }
            }
            let timing = self.attempts[i].host_timing;
            if (status == Status::Admitted || self.attempts[i].role == Role::Simulation)
                && !(status == Status::Admitted
                    && self.attempts[i].role == Role::Simulation
                    && timing.reserved_ns.is_none()
                    && timing.acquisitions.iter().all(Option::is_none)
                    && timing.schedule.is_some()
                    && self.attempts[i].batch.is_none())
            {
                self.timing_errors = self.timing_errors.saturating_add(1);
            }
            let checked = self.checked_time(timing.reserved_ns.or(timing.began_ns), at_ns);
            self.attempts[i].status = status;
            self.attempts[i].host_timing.completed_ns = checked;
            if (status == Status::HostCommitted && self.attempts[i].role == Role::Gameplay)
                || (status == Status::Admitted && self.attempts[i].role == Role::Simulation)
            {
                self.finish_pending = Some(i);
            }
        }
    }
    pub fn host_finished(&mut self, at_ns: Option<u64>) {
        if let Some(i) = self.finish_pending.take() {
            let checked = self.checked_time(self.attempts[i].host_timing.completed_ns, at_ns);
            self.attempts[i].host_timing.finished_ns = checked;
        }
    }
}
