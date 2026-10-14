//! Bounded host identities, independent from callback timestamps and world tick.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Role {
    Initial,
    Background,
    UiResize,
    Gameplay,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[repr(u8)]
pub(super) enum Boundary {
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
pub(super) enum Failure {
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
pub(super) enum Status {
    Open,
    Deferred,
    RefreshCompleted,
    HostCommitted,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Attempt {
    pub id: u64,
    pub role: Role,
    pub session_generation: u64,
    pub requested_host_step: u64,
    pub status: Status,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Segment {
    pub id: u64,
    pub surface_generation: u64,
    pub session_generation: u64,
    pub role: Role,
    pub boundary_bits: u16,
}
const _: () = assert!(std::mem::size_of::<Attempt>() <= 48);
const _: () = assert!(std::mem::size_of::<Segment>() <= 40);

#[derive(Serialize)]
pub(super) struct Model {
    pub attempts: Vec<Attempt>,
    pub segments: Vec<Segment>,
    pub boundary_counts: [u64; 11],
    pub acquisition_failures: [u64; 9],
    pub omitted_attempts: u64,
    pub exhausted: bool,
    #[serde(skip)]
    capacity: usize,
    #[serde(skip)]
    active: Option<usize>,
    #[serde(skip)]
    pending: u16,
    #[serde(skip)]
    surface_generation: u64,
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
            capacity,
            active: None,
            pending: 0,
            surface_generation: 1,
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
    pub fn begin(&mut self, role: Role, session_generation: u64, requested_host_step: u64) {
        self.active = None;
        if self.attempts.len() == self.capacity || self.exhausted || session_generation == 0 {
            self.exhausted = true;
            self.omitted_attempts = self.omitted_attempts.saturating_add(1);
            return;
        }
        if self.attempts.last().is_some_and(|a| a.role != role) {
            self.boundary(Boundary::Role);
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
        });
    }
    /// Called just before each native acquisition, after configure/retry boundaries.
    pub fn context(&mut self) -> Option<(u64, u64, u64, u64)> {
        let a = self.attempts.get(self.active?)?;
        if self.exhausted {
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
    pub fn complete(&mut self, status: Status) {
        if let Some(i) = self.active.take() {
            self.attempts[i].status = status;
        }
    }
}
