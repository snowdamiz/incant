//! Bounded simulation-time scheduling, independent of project documents and VM
//! objects. Only accepted script updates advance this clock. The candidate is
//! committed with the native numeric/structural transaction, never before it.
use crate::ScriptError;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    #[default]
    Global,
    Source(incant_runtime::StableId),
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key {
    pub scope: Scope,
    pub id: String,
}
pub(crate) struct ScopedAction {
    pub scope: Scope,
    pub action: Action,
}

pub(crate) const MAX_TIMERS: usize = 128;
pub(crate) const MAX_TIMER_ACTIONS: usize = 256;
pub(crate) const MAX_TICK: u64 = (1 << 53) - 1;

#[derive(Clone, Default)]
pub(crate) struct Schedule {
    pub tick: u64,
    pub elapsed_seconds: f64,
    pub timers: BTreeMap<Key, Timer>,
}
#[derive(Clone)]
pub(crate) struct Timer {
    pub due: u64,
    pub interval: Option<u32>,
}
#[derive(Clone)]
pub(crate) struct Event {
    pub scope: Scope,
    pub id: String,
    pub scheduled_tick: u64,
}
pub(crate) enum Action {
    Set {
        id: String,
        delay: u32,
        interval: Option<u32>,
    },
    Cancel(String),
}
fn due(tick: u64, delay: u32) -> Result<u64, ScriptError> {
    tick.checked_add(u64::from(delay))
        .filter(|&next| delay > 0 && next <= MAX_TICK)
        .ok_or(ScriptError::TimerRange)
}
impl Schedule {
    pub fn advance(&self, dt: f64) -> Result<(Self, Vec<Event>), ScriptError> {
        let tick = due(self.tick, 1)?;
        let elapsed_seconds = self.elapsed_seconds + dt;
        if !dt.is_finite() || dt <= 0. || !elapsed_seconds.is_finite() {
            return Err(ScriptError::TimerRange);
        }
        // At most 128 small ID/counter records; no component/world/JSON cloning.
        let mut next = Self {
            tick,
            elapsed_seconds,
            timers: self.timers.clone(),
        };
        let mut events = Vec::new();
        for (id, timer) in &mut next.timers {
            if timer.due == tick {
                events.push(Event {
                    scope: id.scope,
                    id: id.id.clone(),
                    scheduled_tick: tick,
                });
                if let Some(interval) = timer.interval {
                    timer.due = due(tick, interval)?;
                }
            }
        }
        // The BTreeMap defines stable ASCII ID order for simultaneous events.
        next.timers.retain(|_, timer| timer.due > tick);
        Ok((next, events))
    }
    pub fn cancel_scope(&mut self, scope: Scope) {
        self.timers.retain(|key, _| key.scope != scope);
    }
    pub fn apply(&mut self, actions: Vec<ScopedAction>) -> Result<(), ScriptError> {
        for ScopedAction { scope, action } in actions {
            match action {
                Action::Set {
                    id,
                    delay,
                    interval,
                } => {
                    let id = Key { scope, id };
                    let timer = Timer {
                        due: due(self.tick, delay)?,
                        interval,
                    };
                    if !self.timers.contains_key(&id) && self.timers.len() == MAX_TIMERS {
                        return Err(ScriptError::InputLimit);
                    }
                    self.timers.insert(id, timer);
                }
                Action::Cancel(id) => {
                    self.timers.remove(&Key { scope, id });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_and_periodic_deadlines_never_round_past_the_js_integer_limit() {
        let mut schedule = Schedule {
            tick: MAX_TICK - 1,
            ..Default::default()
        };
        schedule
            .apply(vec![ScopedAction {
                scope: Scope::Global,
                action: Action::Set {
                    id: "once".into(),
                    delay: 1,
                    interval: None,
                },
            }])
            .unwrap();
        let (end, events) = schedule.advance(0.5).unwrap();
        assert_eq!(events[0].scheduled_tick, MAX_TICK);
        assert!(end.timers.is_empty());
        assert!(matches!(end.advance(0.5), Err(ScriptError::TimerRange)));
        schedule
            .apply(vec![ScopedAction {
                scope: Scope::Global,
                action: Action::Set {
                    id: "repeat".into(),
                    delay: 1,
                    interval: Some(1),
                },
            }])
            .unwrap();
        assert!(matches!(
            schedule.advance(0.5),
            Err(ScriptError::TimerRange)
        ));
        assert_eq!(schedule.tick, MAX_TICK - 1);
    }
}
