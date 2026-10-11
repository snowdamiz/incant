//! Worker-side usage restoration and admission. The ledger lock precedes the
//! short service gate; no filesystem work occurs under the authoring mutex.
use super::{Error, Service};
#[cfg(test)]
mod tests;
use crate::project::LoadState;
use incant_agent::{
    AgentError,
    ledger::{Ledger, LedgerError},
    session::{SessionError, SessionSnapshot},
};
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Scope {
    pub project_id: String,
    pub max_tokens: u64,
}
impl Scope {
    pub fn read(bus: &Mutex<LoadState>) -> Result<Self, Error> {
        Self::from_load(&*bus.lock().map_err(|_| Error::unavailable())?)
    }
    fn from_load(load: &LoadState) -> Result<Self, Error> {
        match load {
            LoadState::Ready(bus) => Ok(Self {
                project_id: bus.project().id.clone(),
                max_tokens: bus.project().settings.agent_budget.max_tokens,
            }),
            _ => Err(Error::unavailable()),
        }
    }
    /// Hold the identity/cap fixed through the caller's in-memory operation.
    /// The caller must not do filesystem or provider work while this is held.
    pub fn guard<'a>(&self, bus: &'a Mutex<LoadState>) -> Result<MutexGuard<'a, LoadState>, Error> {
        let guard = bus.lock().map_err(|_| Error::unavailable())?;
        if Self::from_load(&guard)? == *self {
            Ok(guard)
        } else {
            Err(Error {
                code: "agent.project-changed",
                message: "The project or token cap changed. Retry the agent request.",
            })
        }
    }
    pub fn check_agent(&self, bus: &Mutex<LoadState>) -> Result<(), AgentError> {
        let current = Self::read(bus).map_err(|_| AgentError::ProjectUnavailable)?;
        if current.project_id != self.project_id {
            Err(AgentError::ProjectChanged)
        } else if current.max_tokens != self.max_tokens {
            // Any cap change starts a fresh budget scope, even an increase.
            Err(AgentError::Budget)
        } else {
            Ok(())
        }
    }
}
impl From<LedgerError> for Error {
    fn from(error: LedgerError) -> Self {
        match error {
            LedgerError::Busy => Self {
                code: "agent.budget-busy",
                message: "Another Incant window is using this project's agent budget.",
            },
            LedgerError::Exhausted => Self {
                code: "agent.budget",
                message: "The project's agent token allowance is exhausted.",
            },
            _ => Self {
                code: "agent.budget-storage",
                message: "Saved agent usage is unavailable. It has not been reset.",
            },
        }
    }
}

/// Caller is a blocking worker. The owned ledger remains locked through the
/// identity check and snapshot installation, then is dropped before return.
pub(super) fn restore(
    service: &Service,
    bus: &Mutex<LoadState>,
    open: impl FnOnce(&str) -> Result<Ledger, LedgerError>,
) -> Result<(), Error> {
    let current = service.session.snapshot()?;
    if current.worker_running {
        // This also wakes a pending approval when the user changes the cap.
        // Already in-flight provider usage is still settled by its worker.
        cancel_changed_cap(service, bus, &current)?;
        return Ok(());
    }
    let scope = Scope::read(bus)?;
    let ledger = match open(&scope.project_id) {
        Ok(ledger) => ledger,
        Err(LedgerError::Busy) if service.session.snapshot()?.worker_running => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let snapshot = ledger.snapshot(scope.max_tokens)?;
    let closed = service.closed.lock().map_err(|_| Error::unavailable())?;
    if *closed {
        return Err(Error::unavailable());
    }
    let _scope_guard = scope.guard(bus)?;
    match service.session.restore_budget_if_idle(snapshot) {
        Ok(()) | Err(SessionError::Busy) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn cancel_changed_cap(
    service: &Service,
    bus: &Mutex<LoadState>,
    observed: &SessionSnapshot,
) -> Result<(), Error> {
    if Scope::read(bus)?.max_tokens != observed.budget.max_tokens
        && let Some(turn) = observed.turns.last()
    {
        // A read may outlive that turn. Never retarget cancellation to a newer
        // worker admitted under a new cap while this read was waiting.
        let _ = service.session.cancel(turn.id);
    }
    Ok(())
}
