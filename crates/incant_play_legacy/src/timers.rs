//! Data-only fixed-tick scheduling. Callback code and VM handles are never saved.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

pub const MAX_TIMERS: usize = 128;
const MAX_ACTIONS: usize = 256;
const MAX_PAYLOAD_BYTES: usize = 16 * 1024;
const MAX_SCHEDULE_BYTES: usize = 64 * 1024;
#[derive(Debug, Error)]
pub enum TimerError {
    #[error(
        "timer ID must contain 1–64 ASCII letters, digits, dots, colons, hyphens or underscores"
    )]
    Id,
    #[error(
        "timer delays/intervals must be positive integers within u32, without overflowing the game clock"
    )]
    Range,
    #[error("timer schedule exceeds 128 timers, 256 actions, 16 KiB per payload or 64 KiB total")]
    Limit,
    #[error("timer schedule has an invalid clock or an already-due timer")]
    Clock,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScriptClock {
    #[schemars(range(min = 0, max = 9007199254740991_u64))]
    pub tick: u64,
    #[schemars(range(min = 0))]
    pub elapsed_seconds: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimerRequest {
    #[schemars(length(min = 1, max = 64), regex(pattern = r"^[A-Za-z0-9_.:-]+$"))]
    pub id: String,
    #[schemars(range(min = 1, max = 4294967295_u64))]
    pub delay_ticks: u64,
    #[serde(default)]
    #[schemars(range(min = 1, max = 4294967295_u64))]
    pub interval_ticks: Option<u64>,
    #[serde(default)]
    pub payload: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimerEvent {
    pub id: String,
    pub scheduled_tick: u64,
    pub payload: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Timer {
    due_tick: u64,
    interval_ticks: Option<u64>,
    payload: Value,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Schedule {
    pub clock: ScriptClock,
    timers: BTreeMap<String, Timer>,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Action {
    Set { timer: TimerRequest },
    Cancel { id: String },
}
fn valid_id(id: &str) -> Result<(), TimerError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        return Err(TimerError::Id);
    }
    Ok(())
}
fn interval(ticks: u64) -> Result<(), TimerError> {
    if ticks == 0 || ticks > u64::from(u32::MAX) {
        Err(TimerError::Range)
    } else {
        Ok(())
    }
}
fn due(tick: u64, delay: u64) -> Result<u64, TimerError> {
    interval(delay)?;
    tick.checked_add(delay)
        .filter(|n| *n <= super::saves::MAX_SAVE_TICK)
        .ok_or(TimerError::Range)
}
impl Schedule {
    pub fn at(tick: u64, elapsed_seconds: f64) -> Self {
        Self {
            clock: ScriptClock {
                tick,
                elapsed_seconds,
            },
            ..Self::default()
        }
    }
    pub fn has_timers(&self) -> bool {
        !self.timers.is_empty()
    }
    pub fn validate(&self) -> Result<(), TimerError> {
        if self.clock.tick > super::saves::MAX_SAVE_TICK
            || !self.clock.elapsed_seconds.is_finite()
            || self.clock.elapsed_seconds < 0.
        {
            return Err(TimerError::Clock);
        }
        if self.timers.len() > MAX_TIMERS || serde_json::to_vec(self)?.len() > MAX_SCHEDULE_BYTES {
            return Err(TimerError::Limit);
        }
        for (id, timer) in &self.timers {
            valid_id(id)?;
            if timer.due_tick <= self.clock.tick
                || timer.due_tick > super::saves::MAX_SAVE_TICK
                || timer.due_tick - self.clock.tick > u64::from(u32::MAX)
            {
                return Err(TimerError::Clock);
            }
            if let Some(ticks) = timer.interval_ticks {
                interval(ticks)?;
            }
            if serde_json::to_vec(&timer.payload)?.len() > MAX_PAYLOAD_BYTES {
                return Err(TimerError::Limit);
            }
        }
        Ok(())
    }
    pub fn advance(&self, dt: f64) -> Result<(Self, Vec<TimerEvent>), TimerError> {
        let mut next = self.clone();
        next.clock.tick = due(self.clock.tick, 1)?;
        next.clock.elapsed_seconds += dt;
        if !dt.is_finite() || dt <= 0. || !next.clock.elapsed_seconds.is_finite() {
            return Err(TimerError::Clock);
        }
        let mut events = Vec::new();
        let timers = std::mem::take(&mut next.timers);
        for (id, mut timer) in timers {
            if timer.due_tick <= next.clock.tick {
                events.push(TimerEvent {
                    id: id.clone(),
                    scheduled_tick: timer.due_tick,
                    payload: timer.payload.clone(),
                });
                if let Some(period) = timer.interval_ticks {
                    timer.due_tick = due(timer.due_tick, period)?;
                } else {
                    continue;
                }
            }
            next.timers.insert(id, timer);
        }
        events.sort_by(|a, b| (a.scheduled_tick, &a.id).cmp(&(b.scheduled_tick, &b.id)));
        Ok((next, events))
    }
    pub fn apply(&mut self, actions: Vec<Action>) -> Result<(), TimerError> {
        if actions.len() > MAX_ACTIONS {
            return Err(TimerError::Limit);
        }
        for action in actions {
            match action {
                Action::Set { timer } => {
                    valid_id(&timer.id)?;
                    let due_tick = due(self.clock.tick, timer.delay_ticks)?;
                    if let Some(period) = timer.interval_ticks {
                        interval(period)?;
                    }
                    if serde_json::to_vec(&timer.payload)?.len() > MAX_PAYLOAD_BYTES {
                        return Err(TimerError::Limit);
                    }
                    self.timers.insert(
                        timer.id,
                        Timer {
                            due_tick,
                            interval_ticks: timer.interval_ticks,
                            payload: timer.payload,
                        },
                    );
                    if self.timers.len() > MAX_TIMERS {
                        return Err(TimerError::Limit);
                    }
                }
                Action::Cancel { id } => {
                    valid_id(&id)?;
                    self.timers.remove(&id);
                }
            }
        }
        self.validate()
    }
}
