use super::support::{Fixture, Mock, agent};
use incant_agent::{AgentError, ApprovalMode, ProjectSlot};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Project;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug)]
enum Change {
    Edit,
    Replace,
    SameRevision,
    Cancel,
}

// A deterministic editor slot boundary: approval arms the slot, the first
// following visit captures the source, and the second is final publication.
// No production hook, sleeps, races or filesystem polling are needed.
struct Slot<'a> {
    bus: CommandBus,
    change: Change,
    armed: &'a AtomicBool,
    cancel: &'a AtomicBool,
    visits: usize,
}
impl ProjectSlot for Slot<'_> {
    fn command_bus(&mut self) -> Result<&mut CommandBus, AgentError> {
        if self.armed.load(Ordering::Relaxed) {
            self.visits += 1;
            if self.visits == 2 {
                match self.change {
                    Change::Edit => {
                        self.bus
                            .execute(
                                vec![Command::SetMemory {
                                    section: "user".into(),
                                    text: "new".into(),
                                }],
                                Actor::user("editor"),
                                "Concurrent edit",
                                None,
                            )
                            .unwrap();
                    }
                    Change::Replace => {
                        self.bus = CommandBus::new(Project::empty("Replacement")).unwrap()
                    }
                    Change::SameRevision => {
                        let mut project = self.bus.project().clone();
                        project.name = "Replaced at the same ID and revision".into();
                        self.bus = CommandBus::new(project).unwrap();
                    }
                    Change::Cancel => self.cancel.store(true, Ordering::Relaxed),
                }
            }
        }
        Ok(&mut self.bus)
    }
}

#[test]
fn prepared_derivation_refuses_publication_after_edit_switch_or_cancellation() {
    for change in [
        Change::Edit,
        Change::Replace,
        Change::SameRevision,
        Change::Cancel,
    ] {
        let mut f = Fixture::new();
        // Start at revision zero so a same-ID/revision project replacement
        // isolates the service's full-document comparison, not its revision check.
        f.bus = CommandBus::new(f.bus.project().clone()).unwrap();
        let before = f.bus.project().clone();
        let mut host = f.host();
        let mut provider = Mock::new(f.args());
        let armed = AtomicBool::new(false);
        let cancel = AtomicBool::new(false);
        let slot = Mutex::new(Slot {
            bus: f.bus,
            change,
            armed: &armed,
            cancel: &cancel,
            visits: 0,
        });
        let result = agent(ApprovalMode::Destructive).run_shared(
            &mut provider,
            &slot,
            &mut host,
            "Derive",
            "publication",
            &cancel,
            &mut |_, _| {
                armed.store(true, Ordering::Relaxed);
                true
            },
            &mut |_| {},
        );
        let slot = slot.lock().unwrap();
        assert!(
            slot.visits >= 2,
            "final publication boundary was exercised: {change:?}"
        );
        match change {
            Change::Cancel => {
                assert!(matches!(result, Err(AgentError::Interrupted)));
                assert_eq!(slot.bus.project(), &before);
                assert!(slot.bus.history().is_empty());
            }
            Change::Replace => {
                assert!(matches!(result, Err(AgentError::ProjectChanged)));
                assert_ne!(slot.bus.project().id, before.id);
                assert!(slot.bus.history().is_empty());
            }
            Change::SameRevision => {
                assert!(!result.unwrap().tool_calls[0].succeeded);
                assert_eq!(slot.bus.revision(), 0);
                assert_eq!(slot.bus.project().id, before.id);
                assert_eq!(slot.bus.project().scenes, before.scenes);
                assert!(slot.bus.history().is_empty());
                assert!(
                    provider.result.unwrap()["error"]
                        .as_str()
                        .unwrap()
                        .contains("project or revision changed")
                );
            }
            Change::Edit => {
                assert!(!result.unwrap().tool_calls[0].succeeded);
                assert_eq!(slot.bus.revision(), 1);
                assert_eq!(slot.bus.project().scenes, before.scenes);
                assert_eq!(slot.bus.project().memory["user"], "new");
                assert_eq!(slot.bus.history().len(), 1);
                assert_eq!(slot.bus.history()[0].actor.origin, incant_doc::Origin::User);
                assert!(
                    provider.result.unwrap()["error"]
                        .as_str()
                        .unwrap()
                        .contains("project or revision changed")
                );
            }
        }
    }
}
