//! Inclusive query-stage wall attribution, never an execution policy. Recursive
//! guest getters are observed but make the row unsuitable for exclusive sums.
use super::{ACTIVE, Identity, Region as Major, add, elapsed};
use std::{marker::PhantomData, rc::Rc, time::Instant};

pub const CAPABILITIES: [&str; 6] = [
    "character_motion",
    "trigger_events",
    "navigation_path",
    "locale",
    "localize",
    "other",
];
const _: () = assert!(crate::MAX_SOURCES <= 64);
pub const REGIONS: [&str; 4] = ["call", "parse", "execute", "materialize"];
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Capability {
    Character,
    Triggers,
    Navigation,
    Locale,
    Localize,
    Other,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Stage {
    Parse = 1,
    Execute,
    Materialize,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Counters {
    /// [capability][region]. Stages belong to call; calls belong to callbacks.
    pub inclusive_ns: [[u64; 4]; 6],
    pub calls: [[u32; 4]; 6],
    pub failed: [[u32; 4]; 6],
    /// Bit positions join existing report source slots; not per-source timings.
    pub source_masks: [u64; 6],
    pub recursive_calls: [u32; 6],
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Context {
    sequence: u32,
    capability: Capability,
    stage: Option<Stage>,
    completed: u8,
    failed: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub current: Option<Context>,
    next: u32,
}
fn increment(target: &mut u32, valid: &mut bool) {
    if let Some(next) = target.checked_add(1) {
        *target = next;
    } else {
        *valid = false;
    }
}
fn record(
    active: &mut super::Active,
    capability: Capability,
    region: usize,
    start: Instant,
    now: Instant,
    success: bool,
) {
    let ns = elapsed(start, now, &mut active.row.valid);
    let counters = &mut active.row.queries;
    let i = capability as usize;
    add(
        &mut counters.inclusive_ns[i][region],
        ns,
        &mut active.row.valid,
    );
    increment(&mut counters.calls[i][region], &mut active.row.valid);
    if !success {
        increment(&mut counters.failed[i][region], &mut active.row.valid);
    }
}
/// One attempted native query. Default drop records an early refusal. No guest
/// value or world pointer is retained, and no allocation occurs in these hooks.
pub struct Call {
    token: Option<(Identity, Context, Option<Context>, Instant)>,
    success: bool,
    _thread: PhantomData<Rc<()>>,
}
impl Call {
    pub fn start(capability: Capability) -> Self {
        let token = ACTIVE.with_borrow_mut(|slot| {
            let a = slot.as_mut()?;
            if a.region != Major::Callbacks
                || !a.callback
                || !a.budget_idle()
                || !a.navigation_idle()
            {
                a.row.valid = false;
                return None;
            }
            let Some(source) = a.callback_source.filter(|s| *s < 64) else {
                a.row.valid = false;
                return None;
            };
            let Some(sequence) = a.query.next.checked_add(1) else {
                a.row.valid = false;
                return None;
            };
            a.query.next = sequence;
            let parent = a.query.current;
            if parent.is_some() {
                // The outer parse may include arbitrary reentrant JS/native work.
                // Preserve inclusive evidence; do not silently call it exclusive.
                increment(
                    &mut a.row.queries.recursive_calls[capability as usize],
                    &mut a.row.valid,
                );
                a.row.valid = false;
            }
            a.row.queries.source_masks[capability as usize] |= 1 << source;
            let context = Context {
                sequence,
                capability,
                stage: None,
                completed: 0,
                failed: false,
            };
            a.query.current = Some(context);
            Some((a.row.id, context, parent, Instant::now()))
        });
        Self {
            token,
            success: false,
            _thread: PhantomData,
        }
    }
    pub fn stage(&self, stage: Stage) -> Span {
        let token = self.token.and_then(|(id, original, _, _)| {
            ACTIVE.with_borrow_mut(|slot| {
                let a = slot.as_mut().filter(|a| a.row.id == id)?;
                let Some(mut current) = a.query.current.filter(|c| c.sequence == original.sequence)
                else {
                    a.row.valid = false;
                    return None;
                };
                if current.stage.is_some()
                    || current.failed
                    || current.completed != (1 << (stage as u8 - 1)) - 1
                {
                    a.row.valid = false;
                    return None;
                }
                current.stage = Some(stage);
                a.query.current = Some(current);
                Some((id, original, stage, Instant::now()))
            })
        });
        Span {
            token,
            success: None,
            _thread: PhantomData,
        }
    }
    pub fn finish(mut self, success: bool) {
        self.success = success;
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        let Some((id, original, parent, start)) = self.token else {
            return;
        };
        let now = Instant::now();
        ACTIVE.with_borrow_mut(|slot| {
            let Some(a) = slot.as_mut().filter(|a| a.row.id == id) else {
                return;
            };
            a.row.valid &= a.region == Major::Callbacks
                && a.callback
                && a.budget_idle()
                && a.navigation_idle()
                && a.query.current.is_some_and(|c| {
                    c.sequence == original.sequence
                        && c.stage.is_none()
                        && (!self.success || (c.completed == 7 && !c.failed))
                })
                && !std::thread::panicking();
            a.query.current = parent;
            record(a, original.capability, 0, start, now, self.success);
        });
    }
}
pub struct Span {
    token: Option<(Identity, Context, Stage, Instant)>,
    success: Option<bool>,
    _thread: PhantomData<Rc<()>>,
}
impl Span {
    pub fn finish(mut self, success: bool) {
        self.success = Some(success);
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some((id, original, stage, start)) = self.token else {
            return;
        };
        let now = Instant::now();
        ACTIVE.with_borrow_mut(|slot| {
            let Some(a) = slot.as_mut().filter(|a| a.row.id == id) else {
                return;
            };
            let current = a
                .query
                .current
                .filter(|c| c.sequence == original.sequence && c.stage == Some(stage));
            a.row.valid &= a.region == Major::Callbacks
                && a.callback
                && a.budget_idle()
                && a.navigation_idle()
                && current.is_some()
                && self.success.is_some()
                && !std::thread::panicking();
            if let Some(mut current) = current {
                current.stage = None;
                current.completed |= 1 << (stage as u8 - 1);
                current.failed |= self.success != Some(true);
                a.query.current = Some(current);
            }
            record(
                a,
                original.capability,
                stage as usize,
                start,
                now,
                self.success == Some(true),
            );
        });
    }
}
#[cfg(test)]
mod tests;
