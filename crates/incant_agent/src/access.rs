//! One command path for exclusively owned headless projects and shared editors.
use crate::AgentError;
use incant_cmd::CommandBus;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(crate) enum ProjectAccess<'a> {
    Exclusive(&'a mut CommandBus),
    Shared {
        bus: &'a Mutex<CommandBus>,
        project_id: String,
    },
}

impl<'a> ProjectAccess<'a> {
    pub(crate) fn is_shared(&self) -> bool {
        matches!(self, Self::Shared { .. })
    }

    pub(crate) fn shared(
        bus: &'a Mutex<CommandBus>,
        cancel: &AtomicBool,
    ) -> Result<Self, AgentError> {
        check_cancel(cancel)?;
        let guard = bus.lock().map_err(|_| AgentError::ProjectUnavailable)?;
        check_cancel(cancel)?;
        Ok(Self::Shared {
            bus,
            project_id: guard.project().id.clone(),
        })
    }

    /// The closure must return owned data; a shared guard cannot escape. Slow
    /// host work is prepared here and executed after this call returns.
    pub(crate) fn with_bus<T>(
        &mut self,
        cancel: &AtomicBool,
        f: impl FnOnce(&mut CommandBus) -> Result<T, AgentError>,
    ) -> Result<T, AgentError> {
        check_cancel(cancel)?;
        match self {
            Self::Exclusive(bus) => f(bus),
            Self::Shared { bus, project_id } => {
                let mut guard = bus.lock().map_err(|_| AgentError::ProjectUnavailable)?;
                // A stop or project switch while waiting must not apply a tool
                // from the previous turn to the current project.
                check_cancel(cancel)?;
                if guard.project().id != *project_id {
                    return Err(AgentError::ProjectChanged);
                }
                f(&mut guard)
            }
        }
    }
}

pub(crate) fn check_cancel(cancel: &AtomicBool) -> Result<(), AgentError> {
    if cancel.load(Ordering::Relaxed) {
        Err(AgentError::Interrupted)
    } else {
        Ok(())
    }
}
