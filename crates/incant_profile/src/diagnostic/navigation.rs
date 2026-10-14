//! Same-frame navigation attribution only. Durations are nested within the
//! source callback; lookup durations are also nested within PathQuery.
use super::{ACTIVE, Identity, Region as MajorRegion, add, elapsed};
use std::{marker::PhantomData, rc::Rc, time::Instant};

pub const REGIONS: [&str; 3] = ["path_query", "initial_start", "repeated_start"];
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Region {
    PathQuery,
    InitialStart,
    RepeatedStart,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Counters {
    pub ns: [u64; 3],
    pub calls: [u16; 3],
    /// Error or absent path/nearest result, including a guard's early return.
    pub failed: [u16; 3],
}
/// No allocation, per-triangle timing or query mutation. A missing final guard
/// invalidates the row; a normal early error is a retained failed observation.
pub struct Span {
    token: Option<(Identity, Region, Instant)>,
    success: bool,
    _thread: PhantomData<Rc<()>>,
}
impl Span {
    pub fn start(region: Region) -> Self {
        let token = ACTIVE.with_borrow_mut(|slot| {
            let active = slot.as_mut()?;
            let expected = if matches!(region, Region::PathQuery) {
                0
            } else {
                1
            };
            active.row.valid &= active.region == MajorRegion::Callbacks
                && active.callback
                && active.navigation == expected;
            active.navigation |= 1 << region as usize;
            Some((active.row.id, region, Instant::now()))
        });
        Self {
            token,
            success: false,
            _thread: PhantomData,
        }
    }
    pub fn finish(mut self, success: bool) {
        self.success = success;
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some((id, region, start)) = self.token else {
            return;
        };
        let now = Instant::now();
        ACTIVE.with_borrow_mut(|slot| {
            let Some(active) = slot.as_mut().filter(|a| a.row.id == id) else {
                return;
            };
            let bit = 1 << region as usize;
            let expected = bit | 1;
            active.row.valid &= active.region == MajorRegion::Callbacks
                && active.callback
                && active.navigation == expected
                && !std::thread::panicking();
            active.navigation &= !bit;
            let ns = elapsed(start, now, &mut active.row.valid);
            let counters = &mut active.row.navigation;
            let i = region as usize;
            add(&mut counters.ns[i], ns, &mut active.row.valid);
            for (counter, increment) in [
                (&mut counters.calls[i], true),
                (&mut counters.failed[i], !self.success),
            ] {
                if increment {
                    if let Some(next) = counter.checked_add(1) {
                        *counter = next;
                    } else {
                        active.row.valid = false;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests;
