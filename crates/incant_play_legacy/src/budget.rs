//! Per-execution thread CPU budget. Host suspension and other threads are not
//! charged to gameplay. Unsupported/failed clocks interrupt execution explicitly.
use crate::ScriptError;
use std::{thread::ThreadId, time::Duration};

#[derive(Debug, Clone, Copy)]
enum Failure {
    Unavailable,
    DifferentThread,
    ReversedClock,
    Expired,
}
fn now() -> Result<Duration, Failure> {
    #[cfg(any(unix, windows))]
    {
        cpu_time::ThreadTime::try_now()
            .map(|time| time.as_duration())
            .map_err(|_| Failure::Unavailable)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(Failure::Unavailable)
    }
}

pub(super) struct ExecutionBudget {
    start: Duration,
    owner: ThreadId,
    limit: Duration,
    failure: Option<Failure>,
}
impl ExecutionBudget {
    pub fn new(limit: Duration) -> Result<Self, ScriptError> {
        Ok(Self {
            start: now().map_err(|_| ScriptError::CpuClock {
                reason: "unavailable",
            })?,
            owner: std::thread::current().id(),
            limit,
            failure: None,
        })
    }
    pub fn reset(&mut self) -> Result<(), ScriptError> {
        *self = Self::new(self.limit)?;
        Ok(())
    }
    fn observe(&self) -> Option<Failure> {
        if self.owner != std::thread::current().id() {
            return Some(Failure::DifferentThread);
        }
        match now() {
            Err(error) => Some(error),
            Ok(now) => match now.checked_sub(self.start) {
                None => Some(Failure::ReversedClock),
                Some(elapsed) if elapsed >= self.limit => Some(Failure::Expired),
                _ => None,
            },
        }
    }
    pub fn check(&mut self) -> Result<(), ScriptError> {
        if self.failure.is_none() {
            self.failure = self.observe();
        }
        match self.failure {
            None => Ok(()),
            Some(Failure::Expired) => Err(ScriptError::ExecutionDeadline {
                budget_ms: self.limit.as_millis(),
            }),
            Some(failure) => Err(ScriptError::CpuClock {
                reason: match failure {
                    Failure::Unavailable => "unavailable",
                    Failure::DifferentThread => "execution moved to a different thread",
                    Failure::ReversedClock => "moved backwards",
                    Failure::Expired => unreachable!(),
                },
            }),
        }
    }
    pub fn expired(&mut self) -> bool {
        self.check().is_err()
    }
    #[cfg(test)]
    pub fn exhaust(&mut self) {
        self.failure = Some(Failure::Expired);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_failure_and_thread_changes_fail_closed_and_reset_starts_a_new_execution() {
        let mut budget = ExecutionBudget::new(Duration::from_secs(1)).unwrap();
        budget.failure = Some(Failure::Unavailable);
        assert!(budget.expired());
        assert!(matches!(
            budget.check(),
            Err(ScriptError::CpuClock {
                reason: "unavailable"
            })
        ));
        budget.reset().unwrap();
        assert!(budget.check().is_ok());
        std::thread::spawn(move || {
            assert!(matches!(
                budget.check(),
                Err(ScriptError::CpuClock {
                    reason: "execution moved to a different thread"
                })
            ));
            budget.reset().unwrap();
            assert!(budget.check().is_ok());
            budget.start = now().unwrap() + Duration::from_secs(5);
            assert!(matches!(
                budget.check(),
                Err(ScriptError::CpuClock {
                    reason: "moved backwards"
                })
            ));
        })
        .join()
        .unwrap();
    }
}
