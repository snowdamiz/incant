//! Real Agent loop through the controller, with a deterministic provider double.
use incant_agent::{
    Agent, AgentError, Budget, NoViewport, Usage,
    provider::{Completion, Provider},
    session::*,
};
use incant_cmd::CommandBus;
use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
struct Fixture {
    calls: usize,
    fail: bool,
}
impl Provider for Fixture {
    fn model(&self) -> &str {
        "fixture-model"
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        text("Working é🙂");
        self.calls += 1;
        let usage = Usage {
            input_tokens: 10,
            output_tokens: 20,
        };
        if self.fail {
            return Err(AgentError::Response {
                reason: "private response body",
                usage: Some(usage),
            });
        }
        Ok(Completion {
            usage,
            output: if self.calls == 1 {
                vec![
                    json!({"type":"function_call","name":"doc_patch","call_id":"edit","arguments":json!({"expected_revision":0,"description":"Edit through controller","ops":[{"op":"set_memory","section":"fixture","text":"accepted"}]}).to_string()}),
                ]
            } else {
                vec![]
            },
        })
    }
}
fn run(c: TurnContext, bus: Arc<Mutex<CommandBus>>, fail: bool) -> TurnOutcome {
    let mut agent = Agent {
        budget: c.budget.clone(),
        approval: c.request.approval.mode(),
        max_steps: 4,
        max_output_tokens: 1000,
    };
    let result = agent.run_shared(
        &mut Fixture { calls: 0, fail },
        &bus,
        &mut NoViewport,
        &c.request.prompt,
        &format!("turn-{}", c.turn_id().0),
        c.cancellation(),
        &mut |tool, args| c.approve(tool, args),
        &mut |text| {
            let _ = c.stream(text);
        },
    );
    TurnOutcome {
        budget: agent.budget,
        result,
    }
}
#[test]
fn real_agent_enforces_each_approval_mode_and_keeps_one_undoable_bus() {
    for (approval, expected) in [
        (ApprovalChoice::Auto, 0),
        (ApprovalChoice::Destructive, 1),
        (ApprovalChoice::Always, 1),
    ] {
        let session = Session::new(Budget {
            max_tokens: 1_000_000,
            used_tokens: 123,
        });
        let bus = Arc::new(Mutex::new(
            CommandBus::new(Project::empty("Controller")).unwrap(),
        ));
        let worker_bus = bus.clone();
        session
            .start(
                TurnRequest {
                    prompt: "Edit".into(),
                    model: "fixture-model".into(),
                    approval,
                    view_context: None,
                },
                move |c| run(c, worker_bus, false),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut approvals = 0;
        let final_state = loop {
            let s = session.snapshot().unwrap();
            let t = s.turns.last().unwrap();
            if let Some(p) = &t.pending {
                session.decide(t.id, p.id, true).unwrap();
                approvals += 1;
            }
            if !s.worker_running {
                break s;
            }
            assert!(Instant::now() < deadline);
            thread::yield_now();
        };
        assert_eq!(approvals, expected);
        assert_eq!(final_state.budget.used_tokens, 183);
        let turn = &final_state.turns[0];
        assert_eq!(turn.status, Status::Completed);
        assert_eq!(turn.report.as_ref().unwrap().transaction_ids.len(), 1);
        assert_eq!(turn.text, "Working é🙂Working é🙂");
        let mut bus = bus.lock().unwrap();
        assert_eq!(bus.history().len(), 1);
        assert_eq!(bus.project().memory["fixture"], "accepted");
        assert_eq!(
            bus.history()[0].actor.model.as_deref(),
            Some("fixture-model")
        );
        bus.undo().unwrap();
        assert!(!bus.project().memory.contains_key("fixture"));
    }
}
#[test]
fn real_provider_failure_is_charged_and_redacted() {
    let session = Session::new(Budget {
        max_tokens: 1_000_000,
        used_tokens: 123,
    });
    let bus = Arc::new(Mutex::new(
        CommandBus::new(Project::empty("Controller")).unwrap(),
    ));
    session
        .start(
            TurnRequest {
                prompt: "Edit".into(),
                model: "fixture-model".into(),
                approval: ApprovalChoice::Auto,
                view_context: None,
            },
            move |c| run(c, bus, true),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let s = loop {
        let s = session.snapshot().unwrap();
        if !s.worker_running {
            break s;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    };
    assert_eq!(s.budget.used_tokens, 153);
    assert_eq!(s.turns[0].charged_tokens, 30);
    assert_eq!(s.turns[0].failure, Some(Failure::Provider));
    assert!(s.turns[0].report.is_none());
    assert!(
        !serde_json::to_string(&s)
            .unwrap()
            .contains("private response body")
    );
}
