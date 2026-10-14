//! Trusted, default-off eligibility snapshots. This performs bounded host I/O
//! and native window queries, so an enabled run is diagnostic, not a timing run.
#[cfg(test)]
mod tests;
use crate::session;
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    time::{Duration, Instant},
};

const MAX_RECORDS: usize = 512;
const MAX_BYTES: usize = 512 * 1024;
const MIN_INTERVAL: Duration = Duration::from_millis(250);
const HEARTBEAT: Duration = Duration::from_secs(1);
const TRUNCATION_RESERVE: usize = 256;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("window state log requires --stop-after 1..36000 and no lifecycle probe")]
    Bound,
    #[error(
        "window state log needs a nonempty trusted path of at most 4096 bytes without controls"
    )]
    Path,
    #[error("window state log clock moved backwards")]
    Clock,
    #[error("window state log encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("window state log I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub(super) fn validate(path: &str, bound: Option<u64>, probe: bool) -> Result<(), Error> {
    if probe || bound.is_none_or(|n| !(1..=36_000).contains(&n)) {
        return Err(Error::Bound);
    }
    if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
        return Err(Error::Path);
    }
    Ok(())
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Reason {
    Initial,
    Pulse,
    FocusGained,
    FocusLost,
    Suspended,
    Resumed,
    Resize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Reservation {
    Unattempted,
    Ready,
    Deferred,
}

#[derive(Clone, Copy, Serialize)]
pub(super) struct Snapshot {
    pub host_steps: u64,
    pub world_tick: u64,
    pub generation: u64,
    pub cached_focused: bool,
    pub native_focused: bool,
    pub platform_suspended: bool,
    pub simulation_suspended: bool,
    pub minimized: bool,
    pub supported_size: bool,
    pub physical_size: [u32; 2],
    pub retry_in_us: u64,
    pub surface_skipped: u64,
    pub surface_reconfigurations: u64,
    pub surface_recreations: u64,
}
impl Snapshot {
    fn key(self, reservation: Reservation) -> Key {
        Key {
            generation: self.generation,
            flags: [
                self.cached_focused,
                self.native_focused,
                self.platform_suspended,
                self.simulation_suspended,
                self.minimized,
                self.supported_size,
            ],
            physical_size: self.physical_size,
            reservation,
        }
    }
}
#[derive(PartialEq, Eq)]
struct Key {
    generation: u64,
    flags: [bool; 6],
    physical_size: [u32; 2],
    reservation: Reservation,
}

pub(super) struct Log {
    output: File,
    start: Instant,
    last_seen: Instant,
    last_written: Option<Instant>,
    key: Option<Key>,
    pending: Option<Reason>,
    reservation: Reservation,
    surface_attempts: u64,
    surface_deferrals: u64,
    focus_gained: u64,
    focus_lost: u64,
    coalesced_samples: u64,
    records: usize,
    bytes: usize,
    truncated: bool,
}
impl Log {
    pub fn open(path: &str, bound: Option<u64>, now: Instant) -> Result<Self, Error> {
        validate(path, bound, false)?;
        // The destination comes only from trusted CLI arguments, never package
        // data. create_new also refuses an existing final symlink. Parent paths
        // are trusted host storage; no parent directory is created here.
        let output = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut log = Self {
            output,
            start: now,
            last_seen: now,
            last_written: None,
            key: None,
            pending: None,
            reservation: Reservation::Unattempted,
            surface_attempts: 0,
            surface_deferrals: 0,
            focus_gained: 0,
            focus_lost: 0,
            coalesced_samples: 0,
            records: 0,
            bytes: 0,
            truncated: false,
        };
        log.write(&serde_json::json!({"kind":"header","format":"incant-window-state",
            "version":1,"requested_host_steps":bound,"max_records":MAX_RECORDS,
            "max_bytes":MAX_BYTES,"minimum_snapshot_interval_ms":250,"heartbeat_ms":1000,
            "diagnostic_overhead":true,"timing_or_display_gate":false,
            "coalescing":"transitions within 250 ms retain latest state and cumulative focus-event counts",
            "durability":"unbuffered completed writes survive process timeout, not a power-loss guarantee"}))?;
        Ok(log)
    }

    pub fn reservation(&mut self, ready: bool) {
        self.reservation = if ready {
            Reservation::Ready
        } else {
            Reservation::Deferred
        };
        self.surface_attempts = self.surface_attempts.saturating_add(1);
        self.surface_deferrals = self.surface_deferrals.saturating_add(u64::from(!ready));
    }

    pub fn observe(
        &mut self,
        now: Instant,
        reason: Reason,
        snapshot: Snapshot,
    ) -> Result<(), Error> {
        let elapsed = self.elapsed(now)?;
        match reason {
            Reason::FocusGained => self.focus_gained = self.focus_gained.saturating_add(1),
            Reason::FocusLost => self.focus_lost = self.focus_lost.saturating_add(1),
            _ => {}
        }
        if !matches!(reason, Reason::Pulse) {
            self.pending = Some(reason);
        }
        if self.truncated {
            return Ok(());
        }
        let key = snapshot.key(self.reservation);
        let changed = self.key.as_ref() != Some(&key) || self.pending.is_some();
        let interval = self.last_written.map(|last| now.duration_since(last));
        if !changed && interval.is_some_and(|d| d < HEARTBEAT) {
            return Ok(());
        }
        if interval.is_some_and(|d| d < MIN_INTERVAL) {
            self.coalesced_samples = self.coalesced_samples.saturating_add(1);
            return Ok(());
        }
        let row = serde_json::json!({"kind":"snapshot","elapsed_us":elapsed,
            "reason":self.pending.unwrap_or(reason),"state":snapshot,
            "eligible_by_cached_flags":snapshot.cached_focused && !snapshot.platform_suspended
                && !snapshot.minimized && snapshot.supported_size,
            "last_reservation":self.reservation,"surface_attempts":self.surface_attempts,
            "surface_deferrals":self.surface_deferrals,"focus_gained_events":self.focus_gained,
            "focus_lost_events":self.focus_lost,"coalesced_samples":self.coalesced_samples});
        if self.bounded_write(row, elapsed)? {
            self.key = Some(key);
            self.pending = None;
            self.last_written = Some(now);
        }
        Ok(())
    }

    pub fn finish(
        &mut self,
        now: Instant,
        host_steps: u64,
        returned_report: bool,
    ) -> Result<(), Error> {
        let elapsed = self.elapsed(now)?;
        if !self.truncated {
            self.bounded_write(
                serde_json::json!({"kind":"final","elapsed_us":elapsed,
                "host_steps":host_steps,"returned_report":returned_report}),
                elapsed,
            )?;
        }
        Ok(())
    }
    fn elapsed(&mut self, now: Instant) -> Result<u64, Error> {
        if now < self.last_seen {
            return Err(Error::Clock);
        }
        self.last_seen = now;
        Ok(u64::try_from(now.duration_since(self.start).as_micros()).unwrap_or(u64::MAX))
    }
    fn bounded_write(&mut self, row: serde_json::Value, elapsed: u64) -> Result<bool, Error> {
        let bytes = serde_json::to_vec(&row)?;
        if self.records >= MAX_RECORDS - 1
            || bytes
                .len()
                .saturating_add(self.bytes)
                .saturating_add(1 + TRUNCATION_RESERVE)
                > MAX_BYTES
        {
            self.write(&serde_json::json!({"kind":"truncated","elapsed_us":elapsed,
                "record_limit":MAX_RECORDS,"byte_limit":MAX_BYTES}))?;
            self.truncated = true;
            return Ok(false);
        }
        self.write_bytes(&bytes)?;
        Ok(true)
    }
    fn write(&mut self, row: &serde_json::Value) -> Result<(), Error> {
        self.write_bytes(&serde_json::to_vec(row)?)
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.output.write_all(bytes)?;
        self.output.write_all(b"\n")?;
        self.records += 1;
        self.bytes += bytes.len() + 1;
        Ok(())
    }
}

pub(super) fn preserve_result<T>(
    result: session::Result<T>,
    observed: Result<(), Error>,
) -> session::Result<T> {
    let value = result?;
    observed?;
    Ok(value)
}
