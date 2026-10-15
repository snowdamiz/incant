//! Explicit diagnostic-only correlated ScriptTick attribution.
//!
//! Major regions partition one trace; source callbacks and numeric spans are
//! nested observations, never additional exclusive frame time. No guest values,
//! native views, ECS handles, or pointers are retained. Normal builds omit this
//! entire module and all call sites.
#[cfg(feature = "diagnostic-script-budget")]
pub mod budget;
#[cfg(feature = "diagnostic-navigation")]
pub mod navigation;
mod storage;
#[cfg(test)]
mod tests;
use std::{cell::RefCell, marker::PhantomData, rc::Rc, time::Instant};
pub(crate) use storage::{Identity, RawRow, Storage};
pub use storage::{Report, Row};

pub const MAX_FRAMES: usize = 4096;
pub const REGIONS: [&str; 14] = [
    "initial_checks",
    "snapshots",
    "group_setup",
    "api_setup",
    "capabilities",
    "undo_capture",
    "view_exposure",
    "callbacks",
    "revoke_checks",
    "validation",
    "transaction_stage",
    "scope_cleanup",
    "publication",
    "heap_observation",
];
pub const NUMERIC_REGIONS: [&str; 3] = ["prepare", "change_ticks", "publish_globals"];
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Region {
    InitialChecks,
    Snapshots,
    GroupSetup,
    ApiSetup,
    Capabilities,
    UndoCapture,
    ViewExposure,
    Callbacks,
    RevokeChecks,
    Validation,
    TransactionStage,
    ScopeCleanup,
    Publication,
    HeapObservation,
}
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum NumericRegion {
    Prepare,
    ChangeTicks,
    PublishGlobals,
}
struct Active {
    row: RawRow,
    start: Instant,
    previous: Instant,
    region: Region,
    visited: u16,
    callback: bool,
    numeric: u8,
    #[cfg(feature = "diagnostic-script-budget")]
    budget_check: Option<usize>,
    #[cfg(feature = "diagnostic-navigation")]
    navigation: u8,
}
impl Active {
    fn budget_idle(&self) -> bool {
        #[cfg(feature = "diagnostic-script-budget")]
        {
            self.budget_check.is_none()
        }
        #[cfg(not(feature = "diagnostic-script-budget"))]
        {
            true
        }
    }
    fn navigation_idle(&self) -> bool {
        #[cfg(feature = "diagnostic-navigation")]
        {
            self.navigation == 0
        }
        #[cfg(not(feature = "diagnostic-navigation"))]
        {
            true
        }
    }
}
thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}
fn elapsed(start: Instant, end: Instant, valid: &mut bool) -> u64 {
    match end
        .checked_duration_since(start)
        .and_then(|v| v.as_nanos().try_into().ok())
    {
        Some(value) => value,
        None => {
            *valid = false;
            0
        }
    }
}
fn add(target: &mut u64, value: u64, valid: &mut bool) {
    if let Some(sum) = target.checked_add(value) {
        *target = sum;
    } else {
        *valid = false;
    }
}
/// Must be declared before the ordinary ScriptTick scope, so its final drop
/// observes that scope's actual duration before completing the diagnostic row.
pub struct Frame(Option<Identity>, PhantomData<Rc<()>>);
impl Frame {
    pub fn start() -> Self {
        let start = Instant::now();
        let Some(id) = crate::enabled::diagnostic_start() else {
            return Self(None, PhantomData);
        };
        let installed = ACTIVE.with_borrow_mut(|slot| {
            if let Some(old) = slot.as_mut()
                && old.row.id.generation == id.generation
            {
                old.row.valid = false;
                return false;
            }
            // A retired capture cannot retain authority over its successor.
            *slot = Some(Active {
                row: RawRow::new(id),
                start,
                previous: start,
                region: Region::InitialChecks,
                visited: 1,
                callback: false,
                numeric: 0,
                #[cfg(feature = "diagnostic-script-budget")]
                budget_check: None,
                #[cfg(feature = "diagnostic-navigation")]
                navigation: 0,
            });
            true
        });
        if !installed {
            crate::enabled::diagnostic_reject(id.generation);
        }
        Self(installed.then_some(id), PhantomData)
    }
    pub fn finish(&mut self, success: bool) {
        ACTIVE.with_borrow_mut(|slot| {
            if let Some(active) = slot
                && Some(active.row.id) == self.0
            {
                active.row.success = success;
            }
        });
    }
}
impl Drop for Frame {
    fn drop(&mut self) {
        let Some(id) = self.0 else {
            return;
        };
        let row = ACTIVE.with_borrow_mut(|slot| {
            if slot.as_ref().is_none_or(|a| a.row.id != id) {
                return None;
            }
            let mut a = slot.take().expect("matched private diagnostic row");
            let now = Instant::now();
            a.row.regions_ns[a.region as usize] = elapsed(a.previous, now, &mut a.row.valid);
            a.row.trace_ns = elapsed(a.start, now, &mut a.row.valid);
            a.row.unwound = std::thread::panicking();
            a.row.valid &= a.visited == (1 << REGIONS.len()) - 1
                && !a.callback
                && a.numeric == 0
                && a.navigation_idle()
                && a.budget_idle()
                && a.row.whole_tick_calls == 1;
            Some(a.row)
        });
        if let Some(row) = row {
            crate::enabled::diagnostic_append(row);
        }
    }
}
/// Enter the next major region. Missing, repeated, or reversed boundaries are
/// retained as incomplete diagnostic data, never converted to successful rows.
pub fn mark(region: Region) {
    ACTIVE.with_borrow_mut(|slot| {
        if let Some(a) = slot {
            let now = Instant::now();
            a.row.valid &= region as usize == a.region as usize + 1
                && !a.callback
                && a.numeric == 0
                && a.navigation_idle()
                && a.budget_idle();
            a.row.regions_ns[a.region as usize] = elapsed(a.previous, now, &mut a.row.valid);
            a.previous = now;
            a.region = region;
            a.visited |= 1 << region as usize;
        }
    });
}
pub(crate) fn whole_tick(generation: u64, frame: Option<u64>, ns: u64) {
    ACTIVE.with_borrow_mut(|slot| {
        if let Some(a) = slot
            && a.row.id.generation == generation
        {
            a.row.valid &= frame == Some(a.row.id.frame) && a.row.whole_tick_calls == 0;
            a.row.whole_tick_calls = a.row.whole_tick_calls.saturating_add(1);
            a.row.whole_tick_ns = ns;
        }
    });
}
#[derive(Clone, Copy)]
enum Kind {
    Source(usize),
    Numeric(NumericRegion),
}
/// Nested timing only; its duration is already included in a major region.
pub struct Span {
    token: Option<(Identity, Kind, Instant, Region)>,
    _thread: PhantomData<Rc<()>>,
}
impl Span {
    pub fn callback(source: Option<[u8; 16]>) -> Self {
        let slot = crate::enabled::diagnostic_source(source);
        Self::start(slot.map(Kind::Source))
    }
    pub fn numeric(region: NumericRegion) -> Self {
        Self::start(Some(Kind::Numeric(region)))
    }
    fn start(kind: Option<Kind>) -> Self {
        let token = ACTIVE.with_borrow_mut(|slot| {
            let a = slot.as_mut()?;
            let Some(kind) = kind else {
                a.row.valid = false;
                return None;
            };
            match kind {
                Kind::Source(_) => {
                    a.row.valid &= a.region == Region::Callbacks
                        && !a.callback
                        && a.navigation_idle()
                        && a.budget_idle();
                    a.callback = true;
                }
                Kind::Numeric(n) => {
                    let expected = match n {
                        NumericRegion::Prepare => Region::Validation,
                        _ => Region::ScopeCleanup,
                    };
                    let bit = 1 << n as usize;
                    a.row.valid &= a.region == expected
                        && a.numeric & bit == 0
                        && a.navigation_idle()
                        && a.budget_idle();
                    a.numeric |= bit;
                }
            }
            Some((a.row.id, kind, Instant::now(), a.region))
        });
        Self {
            token,
            _thread: PhantomData,
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some((id, kind, start, region)) = self.token else {
            return;
        };
        let now = Instant::now();
        ACTIVE.with_borrow_mut(|slot| {
            if let Some(a) = slot
                && a.row.id == id
            {
                a.row.valid &= a.region == region;
                let ns = elapsed(start, now, &mut a.row.valid);
                let (total, count) = match kind {
                    Kind::Source(index) => {
                        a.row.valid &= a.navigation_idle() && a.budget_idle();
                        a.callback = false;
                        (&mut a.row.source_ns[index], &mut a.row.source_calls[index])
                    }
                    Kind::Numeric(n) => {
                        a.numeric &= !(1 << n as usize);
                        (
                            &mut a.row.numeric_ns[n as usize],
                            &mut a.row.numeric_calls[n as usize],
                        )
                    }
                };
                add(total, ns, &mut a.row.valid);
                if let Some(next) = count.checked_add(1) {
                    *count = next;
                } else {
                    a.row.valid = false;
                }
            }
        });
    }
}
