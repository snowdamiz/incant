//! Fixed-size diagnostic wake ownership and acquisition records; never admission.
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum WakeCause {
    Unknown,
    Init,
    Poll,
    WaitCancelled,
    ResumeTimeReached,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct WakeEvent {
    pub sequence: u64,
    pub cause: WakeCause,
    pub arrived_ns: Option<u64>,
    pub requested_resume_supplied: bool,
    pub requested_resume_ns: Option<u64>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct PulseWake {
    pub sequence: u64,
    pub pulse_ns: Option<u64>,
    pub events_since_previous_pulse: u64,
    pub event: Option<WakeEvent>,
}
#[derive(Debug, Default, Serialize)]
pub(super) struct WakeCounts {
    pub events: u64,
    pub pulses: u64,
    pub superseded_events: u64,
    pub pulses_without_event: u64,
    pub overflow: bool,
}
#[derive(Default)]
pub(super) struct Wakes {
    pub counts: WakeCounts,
    pending: Option<WakeEvent>,
    pending_count: u64,
}
impl Wakes {
    pub fn event(
        &mut self,
        cause: WakeCause,
        arrived_ns: Option<u64>,
        requested_resume_supplied: bool,
        requested_resume_ns: Option<u64>,
    ) {
        let Some(sequence) = self.counts.events.checked_add(1) else {
            self.counts.overflow = true;
            self.pending = None;
            return;
        };
        self.counts.events = sequence;
        if self.pending.is_some() {
            match self.counts.superseded_events.checked_add(1) {
                Some(value) => self.counts.superseded_events = value,
                None => self.counts.overflow = true,
            }
        }
        match self.pending_count.checked_add(1) {
            Some(value) => self.pending_count = value,
            None => self.counts.overflow = true,
        }
        self.pending = Some(WakeEvent {
            sequence,
            cause,
            arrived_ns,
            requested_resume_supplied,
            requested_resume_ns,
        });
    }
    pub fn pulse(&mut self, pulse_ns: Option<u64>) -> Option<PulseWake> {
        let Some(sequence) = self.counts.pulses.checked_add(1) else {
            self.counts.overflow = true;
            self.pending = None;
            self.pending_count = 0;
            return None;
        };
        self.counts.pulses = sequence;
        let event = self.pending.take();
        if event.is_none() {
            match self.counts.pulses_without_event.checked_add(1) {
                Some(value) => self.counts.pulses_without_event = value,
                None => self.counts.overflow = true,
            }
        }
        let value = PulseWake {
            sequence,
            pulse_ns,
            events_since_previous_pulse: self.pending_count,
            event,
        };
        self.pending_count = 0;
        Some(value)
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::window) enum AcquisitionOutcome {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Acquisition {
    pub began_ns: Option<u64>,
    pub ended_ns: Option<u64>,
    pub outcome: Option<AcquisitionOutcome>,
}
