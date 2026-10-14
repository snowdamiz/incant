//! Private diagnostic: exact drawable observations, never a performance verdict.
//! All row storage is allocated before acquisition; callbacks only mutate a row.
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{fmt, mem::size_of};
use parking_lot::Mutex;

pub const MAX_SAMPLES: usize = 36_000;
const _: () = assert!(size_of::<Row>() <= 64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    Allocation,
    Context,
    Unavailable,
    Incomplete,
    Order,
}

/// Host-supplied identity, captured at acquisition, never read from a late callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub surface_generation: u64,
    pub segment: u64,
    pub host_step: u64,
}
impl Context {
    pub fn new(surface_generation: u64, segment: u64, host_step: u64) -> Result<Self, Error> {
        if surface_generation == 0 || segment == 0 {
            return Err(Error::Context);
        }
        Ok(Self {
            surface_generation,
            segment,
            host_step,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    Acquired,
    Requested,
    Presented(f64),
    /// Apple returns zero for a not-presented/dropped drawable. Not a timestamp.
    ReportedZero,
    Invalid(f64),
    Discarded,
    /// The HAL texture disappeared without explicit request or discard.
    Abandoned,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub context: Context,
    pub drawable_id: u64,
    pub outcome: Outcome,
}
struct State {
    rows: Box<[Row]>,
    used: usize,
    overflow: u64,
    protocol_errors: u64,
}
/// Does not own a drawable, layer, window, device, callback or renderer.
pub struct Collector(Mutex<State>);
impl fmt::Debug for Collector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.0.lock();
        f.debug_struct("PresentationCollector")
            .field("capacity", &s.rows.len())
            .field("used", &s.used)
            .field("overflow", &s.overflow)
            .finish()
    }
}
#[derive(Clone, Debug)]
pub struct Observation {
    collector: Arc<Collector>,
    context: Context,
}
impl Observation {
    pub fn new(collector: Arc<Collector>, context: Context) -> Result<Self, Error> {
        Context::new(
            context.surface_generation,
            context.segment,
            context.host_step,
        )?;
        Ok(Self { collector, context })
    }
    pub(crate) fn acquire(&self, drawable_id: u64) -> Option<Ticket> {
        let mut s = self.collector.0.lock();
        let at = s.used;
        if at == s.rows.len() {
            s.overflow = s.overflow.saturating_add(1);
            return None;
        }
        s.rows[at] = Row {
            context: self.context,
            drawable_id,
            outcome: Outcome::Acquired,
        };
        s.used += 1;
        Some(Ticket {
            collector: Arc::clone(&self.collector),
            at,
        })
    }
}
impl Collector {
    pub fn new(capacity: usize) -> Result<Arc<Self>, Error> {
        if capacity == 0 || capacity > MAX_SAMPLES {
            return Err(Error::Capacity);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation)?;
        rows.resize(
            capacity,
            Row {
                context: Context {
                    surface_generation: 0,
                    segment: 0,
                    host_step: 0,
                },
                drawable_id: 0,
                outcome: Outcome::Acquired,
            },
        );
        Ok(Arc::new(Self(Mutex::new(State {
            rows: rows.into_boxed_slice(),
            used: 0,
            overflow: 0,
            protocol_errors: 0,
        }))))
    }
    /// Snapshot after removing the surface binding. Pending callbacks remain
    /// explicitly Requested. A later snapshot may observe their completion.
    /// This allocates bounded output storage; do not call it inside frame ticks.
    pub fn snapshot(&self) -> Snapshot {
        let s = self.0.lock();
        Snapshot {
            rows: s.rows[..s.used].to_vec(),
            overflow: s.overflow,
            protocol_errors: s.protocol_errors,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Ticket {
    collector: Arc<Collector>,
    at: usize,
}
impl Ticket {
    /// Called once before the native presentation handler is registered.
    pub(crate) fn request(&self) -> Option<Completion> {
        let mut s = self.collector.0.lock();
        if s.rows[self.at].outcome != Outcome::Acquired {
            s.protocol_errors = s.protocol_errors.saturating_add(1);
            return None;
        }
        s.rows[self.at].outcome = Outcome::Requested;
        Some(Completion {
            collector: Arc::clone(&self.collector),
            at: self.at,
        })
    }
    pub(crate) fn discard(&self) {
        let mut s = self.collector.0.lock();
        if s.rows[self.at].outcome == Outcome::Acquired {
            s.rows[self.at].outcome = Outcome::Discarded;
        } else {
            s.protocol_errors = s.protocol_errors.saturating_add(1);
        }
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        let mut s = self.collector.0.lock();
        if s.rows[self.at].outcome == Outcome::Acquired {
            s.rows[self.at].outcome = Outcome::Abandoned;
        }
    }
}
/// Only this bounded collector and row index cross into the native callback.
pub(crate) struct Completion {
    collector: Arc<Collector>,
    at: usize,
}
impl Completion {
    pub(crate) fn presented(&self, time: f64) {
        let mut s = self.collector.0.lock();
        if s.rows[self.at].outcome != Outcome::Requested {
            s.protocol_errors = s.protocol_errors.saturating_add(1);
            return;
        }
        s.rows[self.at].outcome = if time == 0.0 {
            Outcome::ReportedZero
        } else if time.is_finite() && time > 0.0 {
            Outcome::Presented(time)
        } else {
            Outcome::Invalid(time)
        };
    }
}
#[derive(Debug)]
pub struct Snapshot {
    pub rows: Vec<Row>,
    pub overflow: u64,
    pub protocol_errors: u64,
}
/// Counts are observations, not a completeness or frame-rate verdict. In
/// particular, ReportedZero and an absent callback are different outcomes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub rows: usize,
    pub segments: usize,
    pub acquired: usize,
    pub requested: usize,
    pub presented: usize,
    pub reported_zero: usize,
    pub invalid: usize,
    pub discarded: usize,
    pub abandoned: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub from: usize,
    pub to: usize,
    pub seconds: f64,
}
impl Snapshot {
    pub fn coverage(&self) -> Coverage {
        let mut result = Coverage {
            rows: self.rows.len(),
            ..Coverage::default()
        };
        let mut previous = None;
        for row in &self.rows {
            let identity = (row.context.surface_generation, row.context.segment);
            if previous != Some(identity) {
                result.segments += 1;
                previous = Some(identity);
            }
            match row.outcome {
                Outcome::Acquired => result.acquired += 1,
                Outcome::Requested => result.requested += 1,
                Outcome::Presented(_) => result.presented += 1,
                Outcome::ReportedZero => result.reported_zero += 1,
                Outcome::Invalid(_) => result.invalid += 1,
                Outcome::Discarded => result.discarded += 1,
                Outcome::Abandoned => result.abandoned += 1,
            }
        }
        result
    }
    /// Raw intervals only. Zero/discarded drawables between displayed endpoints
    /// remain inside elapsed time. Segment boundaries never create an interval.
    /// There is deliberately no p95/gate method: rows include leading/trailing
    /// zero/discard events so the host must disclose exact window coverage.
    pub fn intervals(&self) -> Result<Vec<Interval>, Error> {
        if self.overflow != 0 || self.protocol_errors != 0 {
            return Err(Error::Incomplete);
        }
        let mut result = Vec::new();
        let mut previous: Option<usize> = None;
        let mut prior_row: Option<&Row> = None;
        for (i, row) in self.rows.iter().enumerate() {
            if let Some(p) = prior_row {
                if row.context.surface_generation < p.context.surface_generation
                    || (row.context.surface_generation == p.context.surface_generation
                        && (p.drawable_id >= row.drawable_id
                            || row.context.segment < p.context.segment
                            || (row.context.segment == p.context.segment
                                && row.context.host_step < p.context.host_step)))
                {
                    return Err(Error::Order);
                }
                if p.context.surface_generation != row.context.surface_generation
                    || p.context.segment != row.context.segment
                {
                    previous = None;
                }
            }
            prior_row = Some(row);
            match row.outcome {
                Outcome::Presented(t) => {
                    if !t.is_finite() || t <= 0.0 {
                        return Err(Error::Incomplete);
                    }
                    if let Some(p) = previous {
                        if let Outcome::Presented(old) = self.rows[p].outcome {
                            let seconds = t - old;
                            if !seconds.is_finite() || seconds <= 0.0 {
                                return Err(Error::Order);
                            }
                            result.push(Interval {
                                from: p,
                                to: i,
                                seconds,
                            });
                        }
                    }
                    previous = Some(i);
                }
                Outcome::ReportedZero | Outcome::Discarded => {}
                _ => return Err(Error::Incomplete),
            }
        }
        Ok(result)
    }
}
#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
