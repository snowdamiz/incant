use super::*;
use crate::agent::{Service, budget};
use incant_agent::{Usage, ledger::Ledger, provider::Completion, session::*};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{AgentBudgetSettings, Project};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

struct CapChange {
    bus: Arc<Mutex<crate::project::LoadState>>,
    calls: Arc<AtomicUsize>,
}
fn set_cap(bus: &Mutex<crate::project::LoadState>, max_tokens: u64) {
    bus.lock()
        .unwrap()
        .bus_mut()
        .unwrap()
        .execute(
            vec![Command::SetAgentBudget {
                settings: AgentBudgetSettings { max_tokens },
            }],
            Actor::user("fixture"),
            "Change cap",
            None,
        )
        .unwrap();
}
impl Provider for CapChange {
    fn model(&self) -> &str {
        "fixture"
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Emulate the user changing the cap while a provider request is in flight.
        set_cap(&self.bus, 0);
        Ok(Completion {
            usage: Usage {
                input_tokens: 10,
                output_tokens: 7,
            },
            output: vec![
                json!({"type":"function_call","name":"doc_patch","call_id":"edit",
                "arguments":json!({"expected_revision":1,"description":"must not apply",
                    "ops":[{"op":"set_memory","section":"forbidden","text":"changed"}]}).to_string()}),
            ],
        })
    }
}

#[test]
fn cap_changes_fence_provider_and_tools_without_refunding_completed_usage() {
    for before_provider in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let mut project = Project::empty("Budget scope");
        project.settings.agent_budget.max_tokens = 1_000_000;
        let bus = Arc::new(Mutex::new(crate::project::LoadState::Ready(Box::new(
            CommandBus::new(project).unwrap(),
        ))));
        let scope = budget::Scope::read(&bus).unwrap();
        let ledger = Ledger::open_in(temp.path(), &scope.project_id).unwrap();
        let service = Service::new();
        let owned = bus.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let called = calls.clone();
        let expected = scope.clone();
        service
            .start_with(
                TurnRequest {
                    prompt: "edit".into(),
                    model: "fixture".into(),
                    approval: ApprovalChoice::Auto,
                    view_context: None,
                },
                Some((ledger, 1_000_000)),
                || Ok((Connection::OAuth("synthetic".into()), scope.guard(&bus)?)),
                move |turn, connection, owner| {
                    owner.run(turn, move |state, turn| {
                        if before_provider {
                            set_cap(&owned, 0);
                        }
                        let mut provider = CapChange {
                            bus: owned.clone(),
                            calls: called,
                        };
                        state.run_with_scope(
                            turn,
                            ContextKey::Fixture(connection),
                            owned,
                            None,
                            &mut provider,
                            Some(expected),
                        )
                    })
                },
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let snapshot = loop {
            let snapshot = service.session.snapshot().unwrap();
            if !snapshot.worker_running {
                break snapshot;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        };
        let charge = if before_provider { 0 } else { 17 };
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(!before_provider));
        assert_eq!(snapshot.turns[0].failure, Some(Failure::ProjectChanged));
        assert_eq!(snapshot.budget.used_tokens, charge);
        assert!(!snapshot.accounting_uncertain);
        let mut load = bus.lock().unwrap();
        let document = load.bus_mut().unwrap();
        assert_eq!(document.revision(), 1);
        assert_eq!(document.project().settings.agent_budget.max_tokens, 0);
        assert!(!document.project().memory.contains_key("forbidden"));
        drop(load);
        let persisted = Ledger::open_in(temp.path(), &scope.project_id)
            .unwrap()
            .snapshot(0)
            .unwrap();
        assert_eq!(persisted.budget.used_tokens, charge);
        assert!(!persisted.accounting_uncertain);
    }
}
