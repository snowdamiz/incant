//! One command path for exclusively owned headless projects and shared editors.
use crate::AgentError;
use incant_cmd::CommandBus;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

/// A native project container that may still be loading or may have failed.
/// Returning its existing command bus preserves the editor's journal and history;
/// implementations must never substitute a cloned bus or create a second one.
pub trait ProjectSlot {
    fn command_bus(&mut self) -> Result<&mut CommandBus, AgentError>;
}
impl ProjectSlot for CommandBus {
    fn command_bus(&mut self) -> Result<&mut CommandBus, AgentError> {
        Ok(self)
    }
}

pub(crate) trait SharedBus {
    fn visit(
        &self,
        operation: &mut dyn FnMut(&mut CommandBus) -> Result<(), AgentError>,
    ) -> Result<(), AgentError>;
}
impl<T: ProjectSlot> SharedBus for Mutex<T> {
    fn visit(
        &self,
        operation: &mut dyn FnMut(&mut CommandBus) -> Result<(), AgentError>,
    ) -> Result<(), AgentError> {
        let mut guard = self.lock().map_err(|_| AgentError::ProjectUnavailable)?;
        operation(guard.command_bus()?)
    }
}

// SharedBus is internal and invokes the operation exactly once while holding its
// guard. The owned return value cannot retain a reference to the locked project.
fn visit<T>(
    bus: &dyn SharedBus,
    operation: impl FnOnce(&mut CommandBus) -> Result<T, AgentError>,
) -> Result<T, AgentError> {
    let mut operation = Some(operation);
    let mut value = None;
    bus.visit(&mut |bus| {
        value = Some(operation.take().ok_or(AgentError::ProjectUnavailable)?(
            bus,
        )?);
        Ok(())
    })?;
    value.ok_or(AgentError::ProjectUnavailable)
}

pub(crate) enum ProjectAccess<'a> {
    Exclusive(&'a mut CommandBus),
    Shared {
        bus: &'a dyn SharedBus,
        project_id: String,
    },
}

impl<'a> ProjectAccess<'a> {
    pub(crate) fn is_shared(&self) -> bool {
        matches!(self, Self::Shared { .. })
    }

    pub(crate) fn shared(bus: &'a dyn SharedBus, cancel: &AtomicBool) -> Result<Self, AgentError> {
        check_cancel(cancel)?;
        let project_id = visit(bus, |bus| {
            check_cancel(cancel)?;
            Ok(bus.project().id.clone())
        })?;
        Ok(Self::Shared { bus, project_id })
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
            Self::Shared { bus, project_id } => visit(*bus, |bus| {
                // A stop or project switch while waiting must not apply a tool
                // from the previous turn to the current project.
                check_cancel(cancel)?;
                if bus.project().id != *project_id {
                    return Err(AgentError::ProjectChanged);
                }
                f(bus)
            }),
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
