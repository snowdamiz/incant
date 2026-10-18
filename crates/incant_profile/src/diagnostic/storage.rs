use super::{MAX_FRAMES, NUMERIC_REGIONS, REGIONS};
use crate::{Error, MAX_SOURCES};
use std::thread::{self, ThreadId};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Identity {
    pub generation: u64,
    pub frame: u64,
    pub ordinal: u64,
}
#[derive(Clone)]
pub(crate) struct RawRow {
    pub id: Identity,
    pub regions_ns: [u64; REGIONS.len()],
    pub numeric_ns: [u64; NUMERIC_REGIONS.len()],
    pub numeric_calls: [u16; NUMERIC_REGIONS.len()],
    #[cfg(feature = "diagnostic-navigation")]
    pub navigation: super::navigation::Counters,
    #[cfg(feature = "diagnostic-script-budget")]
    pub budget_checks: super::budget::Counters,
    #[cfg(feature = "diagnostic-script-queries")]
    pub queries: super::queries::Counters,
    pub source_ns: [u64; MAX_SOURCES],
    pub source_calls: [u16; MAX_SOURCES],
    pub trace_ns: u64,
    pub whole_tick_ns: u64,
    pub whole_tick_calls: u8,
    pub success: bool,
    pub valid: bool,
    pub unwound: bool,
}
impl RawRow {
    pub fn new(id: Identity) -> Self {
        Self {
            id,
            regions_ns: [0; REGIONS.len()],
            numeric_ns: [0; NUMERIC_REGIONS.len()],
            numeric_calls: [0; NUMERIC_REGIONS.len()],
            #[cfg(feature = "diagnostic-navigation")]
            navigation: Default::default(),
            #[cfg(feature = "diagnostic-script-budget")]
            budget_checks: Default::default(),
            #[cfg(feature = "diagnostic-script-queries")]
            queries: Default::default(),
            source_ns: [0; MAX_SOURCES],
            source_calls: [0; MAX_SOURCES],
            trace_ns: 0,
            whole_tick_ns: 0,
            whole_tick_calls: 0,
            success: false,
            valid: id.frame != 0,
            unwound: false,
        }
    }
}
// Real native storage, including source aggregates and padding, must meet this
// compile-time bound. There are no 64 pairs of Instant values retained per row.
const _: () = assert!(std::mem::size_of::<RawRow>() <= super::MAX_RAW_ROW_BYTES);
#[derive(Debug, serde::Serialize)]
pub struct Row {
    pub frame: u64,
    pub ordinal: u64,
    pub success: bool,
    pub valid: bool,
    pub unwound: bool,
    pub trace_ns: u64,
    pub whole_tick_ns: u64,
    pub whole_tick_calls: u8,
    pub regions_ns: [u64; REGIONS.len()],
    pub numeric_ns: [u64; NUMERIC_REGIONS.len()],
    pub numeric_calls: [u16; NUMERIC_REGIONS.len()],
    #[cfg(feature = "diagnostic-navigation")]
    pub navigation: super::navigation::Counters,
    #[cfg(feature = "diagnostic-script-budget")]
    pub budget_checks: super::budget::Counters,
    #[cfg(feature = "diagnostic-script-queries")]
    pub queries: super::queries::Counters,
    /// Slots join the existing report's exact source identities.
    #[serde(serialize_with = "serialize_array")]
    pub source_ns: [u64; MAX_SOURCES],
    #[serde(serialize_with = "serialize_array")]
    pub source_calls: [u16; MAX_SOURCES],
}
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub diagnostic_only: bool,
    pub eligible_for_performance_gate: bool,
    pub schema: u32,
    pub max_rows: usize,
    pub raw_row_bytes: usize,
    pub raw_storage_bound_bytes: usize,
    pub attempted: u64,
    pub open: u64,
    pub complete: bool,
    pub regions: [&'static str; REGIONS.len()],
    pub numeric_regions: [&'static str; NUMERIC_REGIONS.len()],
    #[cfg(feature = "diagnostic-navigation")]
    pub navigation_regions: [&'static str; 3],
    #[cfg(feature = "diagnostic-script-budget")]
    pub budget_check_regions: [&'static str; 4],
    #[cfg(feature = "diagnostic-script-queries")]
    pub query_capabilities: [&'static str; 6],
    #[cfg(feature = "diagnostic-script-queries")]
    pub query_regions: [&'static str; 4],
    pub interpretation: &'static str,
    pub rows: Vec<Row>,
}
pub(crate) struct Storage {
    owner: Option<ThreadId>,
    rows: Vec<RawRow>,
    attempted: u64,
    open: u64,
    valid: bool,
}
impl Storage {
    pub const fn empty() -> Self {
        Self {
            owner: None,
            rows: Vec::new(),
            attempted: 0,
            open: 0,
            valid: true,
        }
    }
    pub fn allocated() -> Result<Self, Error> {
        let mut result = Self::empty();
        result.owner = Some(thread::current().id());
        result
            .rows
            .try_reserve_exact(MAX_FRAMES)
            .map_err(|_| Error::Storage)?;
        if result.rows.capacity() > MAX_FRAMES {
            return Err(Error::Storage);
        }
        Ok(result)
    }
    pub fn start(&mut self, generation: u64, frame: Option<u64>) -> Option<Identity> {
        let Some(attempted) = self.attempted.checked_add(1) else {
            self.valid = false;
            return None;
        };
        self.attempted = attempted;
        if self.owner != Some(thread::current().id()) || attempted > MAX_FRAMES as u64 {
            self.valid = false;
            return None;
        }
        self.open += 1;
        Some(Identity {
            generation,
            frame: frame.unwrap_or(0),
            ordinal: attempted,
        })
    }
    #[cfg(feature = "diagnostic-script-budget")]
    pub fn reject_foreign_thread(&mut self) {
        if self
            .owner
            .is_some_and(|owner| owner != thread::current().id())
        {
            self.valid = false;
        }
    }
    pub fn reject(&mut self) {
        self.valid = false;
    }
    pub fn append(&mut self, row: RawRow) {
        self.open = self.open.saturating_sub(1);
        self.valid &= self.owner == Some(thread::current().id())
            && self
                .rows
                .last()
                .is_none_or(|old| old.id.frame < row.id.frame)
            && row.valid
            && row.success
            && !row.unwound;
        if self.rows.len() < MAX_FRAMES {
            self.rows.push(row);
        } else {
            self.valid = false;
        }
    }
    pub fn report(&self) -> Report {
        Report {
            diagnostic_only: true,
            eligible_for_performance_gate: false,
            schema: 4 * u32::from(cfg!(feature = "diagnostic-script-queries"))
                + match (
                    cfg!(feature = "diagnostic-navigation"),
                    cfg!(feature = "diagnostic-script-budget"),
                ) {
                    (false, false) => 1,
                    (true, false) => 2,
                    (false, true) => 3,
                    (true, true) => 4,
                },
            max_rows: MAX_FRAMES,
            raw_row_bytes: std::mem::size_of::<RawRow>(),
            raw_storage_bound_bytes: MAX_FRAMES * std::mem::size_of::<RawRow>(),
            attempted: self.attempted,
            open: self.open,
            complete: self.valid
                && self.attempted > 0
                && self.open == 0
                && self.attempted == self.rows.len() as u64,
            regions: REGIONS,
            numeric_regions: NUMERIC_REGIONS,
            #[cfg(feature = "diagnostic-navigation")]
            navigation_regions: super::navigation::REGIONS,
            #[cfg(feature = "diagnostic-script-budget")]
            budget_check_regions: super::budget::REGIONS,
            #[cfg(feature = "diagnostic-script-queries")]
            query_capabilities: super::queries::CAPABILITIES,
            #[cfg(feature = "diagnostic-script-queries")]
            query_regions: super::queries::REGIONS,
            interpretation: if cfg!(feature = "diagnostic-script-queries") {
                "Major regions partition trace_ns. Query call/parse/execute/materialize wall durations are nested in callback time; stages are nested in calls. Call residual includes unchanged admission, limits, deadline checks and observer work. Recursive getter calls retain inclusive totals but invalidate exclusive coverage; never add child and parent times. Source masks reference existing report source slots, not per-source durations. Other collects all remaining native query kinds. Navigation/budget observations, if enabled, remain nested too. Failures/overflow/clock anomalies remain explicit; no gate, interpreter-only remainder or saving inference."
            } else if cfg!(feature = "diagnostic-script-budget") {
                "Major regions partition trace_ns. Budget-check durations are nested monotonic wall observations around unchanged interrupt/explicit lock/check boundaries, split inside/outside source callbacks; clock_reads counts existing thread-CPU clock attempts within those checks, excluding initialization/reset. Checks inside navigation are also nested in its spans when enabled. Refused checks stay visible; observer work remains included and is not an exclusive CPU cost, saving or performance gate."
            } else if cfg!(feature = "diagnostic-navigation") {
                "Major regions partition trace_ns. Navigation queries are nested in source callbacks; initial/repeated_start lookup spans are nested in queries. They are never additive exclusive time. Initial_start excludes corridor endpoint scans; failed means an error/absent observed result, not necessarily a failed optional shortcut. whole_tick_ns is the existing ScriptTick in this same frame. Instrumentation stays included; no gate or speedup inference."
            } else {
                "Major regions partition trace_ns. Callback and numeric spans are nested, not additive. whole_tick_ns is the existing ScriptTick scope in this same frame. Instrumentation and residual overhead remain included; no gate or speedup inference."
            },
            rows: self
                .rows
                .iter()
                .map(|r| Row {
                    frame: r.id.frame,
                    ordinal: r.id.ordinal,
                    success: r.success,
                    valid: r.valid,
                    unwound: r.unwound,
                    trace_ns: r.trace_ns,
                    whole_tick_ns: r.whole_tick_ns,
                    whole_tick_calls: r.whole_tick_calls,
                    regions_ns: r.regions_ns,
                    numeric_ns: r.numeric_ns,
                    numeric_calls: r.numeric_calls,
                    #[cfg(feature = "diagnostic-navigation")]
                    navigation: r.navigation.clone(),
                    #[cfg(feature = "diagnostic-script-budget")]
                    budget_checks: r.budget_checks.clone(),
                    #[cfg(feature = "diagnostic-script-queries")]
                    queries: r.queries.clone(),
                    source_ns: r.source_ns,
                    source_calls: r.source_calls,
                })
                .collect(),
        }
    }
}

const _: () = assert!(std::mem::size_of::<Row>() <= super::MAX_RAW_ROW_BYTES);
fn serialize_array<S: serde::Serializer, T: serde::Serialize, const N: usize>(
    values: &[T; N],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(values.as_slice(), serializer)
}
