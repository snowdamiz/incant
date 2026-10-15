//! Bounded nested observations around the existing CPU-budget lock/check.
//! The elapsed clock is monotonic wall time; it does not replace, sample less
//! often, or cache the thread-CPU clock used by enforcement.
use super::{ACTIVE, Identity, Region, add, elapsed};
use std::{marker::PhantomData, rc::Rc, time::Instant};

pub const REGIONS: [&str; 4] = [
    "interrupt_callback",
    "explicit_callback",
    "interrupt_other",
    "explicit_other",
];
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Origin {
    Interrupt,
    Explicit,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Counters {
    pub ns: [u64; 4],
    pub calls: [u32; 4],
    /// A refused check, including a previously latched enforcement failure.
    pub refused: [u32; 4],
    /// Existing thread-CPU clock attempts within checks, not reset/initialization.
    pub clock_reads: [u32; 4],
}
fn increment(counter: &mut u32, valid: &mut bool) {
    if let Some(next) = counter.checked_add(1) {
        *counter = next;
    } else {
        *valid = false;
    }
}
/// Annotate the existing clock attempt, immediately before the unchanged
/// provider call. A reset outside a check is intentionally not attributed.
pub fn clock_read() {
    ACTIVE.with_borrow_mut(|slot| {
        if let Some(active) = slot
            && let Some(index) = active.budget_check
        {
            increment(
                &mut active.row.budget_checks.clock_reads[index],
                &mut active.row.valid,
            );
        }
    });
}
/// Only one check can be active on its owner thread. This guard is not Send;
/// there is no allocation or stored value/pointer from the VM or native world.
pub struct Span {
    token: Option<(Identity, usize, Region, Instant, u32)>,
    accepted: Option<bool>,
    _thread: PhantomData<Rc<()>>,
}
impl Span {
    pub fn start(origin: Origin) -> Self {
        let token = ACTIVE.with_borrow_mut(|slot| {
            let active = slot.as_mut()?;
            if !active.budget_idle() {
                active.row.valid = false;
                return None;
            }
            let index = origin as usize + if active.callback { 0 } else { 2 };
            active.budget_check = Some(index);
            Some((
                active.row.id,
                index,
                active.region,
                Instant::now(),
                active.row.budget_checks.clock_reads[index],
            ))
        });
        if token.is_none() {
            // No mutex is added to the observed owner-thread check path. Outside
            // a trace, reject foreign-thread capture activity instead of silently
            // presenting another thread's missing checks as complete coverage.
            crate::enabled::diagnostic_budget_unscoped();
        }
        Self {
            token,
            accepted: None,
            _thread: PhantomData,
        }
    }
    pub fn finish(mut self, accepted: bool) {
        self.accepted = Some(accepted);
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some((id, index, region, start, clock_before)) = self.token else {
            return;
        };
        let now = Instant::now();
        ACTIVE.with_borrow_mut(|slot| {
            let Some(active) = slot.as_mut().filter(|a| a.row.id == id) else {
                return;
            };
            active.row.valid &= active.region == region
                && active.callback == (index < 2)
                && active.budget_check == Some(index)
                && self.accepted.is_some()
                && !std::thread::panicking();
            active.budget_check = None;
            let clocks = active.row.budget_checks.clock_reads[index].checked_sub(clock_before);
            // An accepted check reaches exactly one existing clock read. A
            // refusal may stop at owner/failure-latch admission before that read.
            active.row.valid &=
                matches!(clocks, Some(0 | 1)) && (self.accepted != Some(true) || clocks == Some(1));
            let ns = elapsed(start, now, &mut active.row.valid);
            let counters = &mut active.row.budget_checks;
            add(&mut counters.ns[index], ns, &mut active.row.valid);
            increment(&mut counters.calls[index], &mut active.row.valid);
            if self.accepted != Some(true) {
                increment(&mut counters.refused[index], &mut active.row.valid);
            }
        });
    }
}
#[cfg(test)]
mod tests;
