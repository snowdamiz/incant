use super::*;
use incant_agent::{
    Agent, AgentError, NoViewport, Usage,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Project, Scene};
use serde_json::{Value, json};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn request(text: &str) -> TurnRequest {
    TurnRequest {
        prompt: text.into(),
        model: "fixture".into(),
        approval: ApprovalChoice::Destructive,
    }
}
fn bus() -> Arc<Mutex<crate::project::LoadState>> {
    Arc::new(Mutex::new(crate::project::LoadState::Ready(Box::new(
        CommandBus::new(Project::empty("Agent editor")).unwrap(),
    ))))
}
fn connection() -> Connection {
    Connection::OAuth("fixture-account".into())
}
fn wait(service: &Service, predicate: impl Fn(&SessionSnapshot) -> bool) -> SessionSnapshot {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = service.session.snapshot().unwrap();
        if predicate(&snapshot) {
            return snapshot;
        }
        assert!(Instant::now() < until, "native agent session timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn empty(turn: TurnContext) -> TurnOutcome {
    TurnOutcome {
        budget: turn.budget,
        result: Ok(incant_agent::TurnReport {
            steps: 0,
            usage: Usage::default(),
            transaction_ids: vec![],
            tool_calls: vec![],
        }),
    }
}

#[test]
fn admission_account_change_and_shutdown_fence_physical_workers() {
    let service = Service::new();
    let (tx, rx) = mpsc::channel();
    let (finish, ending) = mpsc::channel();
    let id = service
        .start(
            request("wait"),
            || Ok(connection()),
            move |turn, _, _| {
                tx.send(()).unwrap();
                ending.recv().unwrap();
                assert!(turn.cancellation().load(Ordering::Acquire));
                assert_eq!(turn.stream("late"), Err(SessionError::StaleTurn));
                empty(turn)
            },
        )
        .unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    service.account_operation(|| {
        assert!(matches!(
            service.session.snapshot().unwrap().turns[0].status,
            Status::Stopping
        ));
    });
    assert_eq!(
        service
            .start(request("second"), || Ok(connection()), |t, _, _| empty(t))
            .unwrap_err()
            .code,
        "agent.busy"
    );
    assert_eq!(
        service.session.decide(id, ApprovalId(999), true),
        Err(SessionError::StaleTurn)
    );
    finish.send(()).unwrap();
    let snapshot = wait(&service, |s| !s.worker_running);
    assert!(matches!(snapshot.turns[0].status, Status::Cancelled));
    assert!(snapshot.turns[0].text.is_empty());
    service.shutdown();
    let called = AtomicBool::new(false);
    assert_eq!(
        service
            .start(
                request("closed"),
                || {
                    called.store(true, Ordering::Relaxed);
                    Ok(connection())
                },
                |t, _, _| empty(t)
            )
            .unwrap_err()
            .code,
        "agent.unavailable"
    );
    assert!(!called.load(Ordering::Relaxed));
}

struct Patch {
    calls: usize,
    expected_revision: u64,
    seen: Arc<Mutex<Vec<Vec<Value>>>>,
}
impl Provider for Patch {
    fn model(&self) -> &str {
        "fixture"
    }
    fn complete(
        &mut self,
        input: &[Value],
        _: &[Value],
        _: u64,
        text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.seen.lock().unwrap().push(input.to_vec());
        self.calls += 1;
        text("Working");
        Ok(Completion {
            usage: Usage {
                input_tokens: 7,
                output_tokens: 3,
            },
            output: if self.calls == 1 {
                vec![
                    json!({"type":"function_call","name":"doc_patch","call_id":"edit","arguments":json!({"expected_revision":self.expected_revision,"description":"Editor agent edit","ops":[{"op":"set_memory","section":"agent","text":"changed"}]}).to_string()}),
                ]
            } else {
                vec![
                    json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"Done"}]}),
                ]
            },
        })
    }
}
fn job(
    turn: TurnContext,
    connection: Connection,
    context: Arc<Mutex<Context>>,
    bus: Arc<Mutex<crate::project::LoadState>>,
    provider: &mut dyn Provider,
) -> TurnOutcome {
    let mut agent = Agent {
        budget: turn.budget.clone(),
        approval: turn.request.approval.mode(),
        max_steps: 4,
        max_output_tokens: 1024,
    };
    let result = worker::execute(
        &mut agent,
        &turn,
        &context,
        ContextKey::Fixture(connection),
        &bus,
        &mut NoViewport,
        provider,
    );
    TurnOutcome {
        budget: agent.budget,
        result,
    }
}

#[test]
fn streamed_approval_commits_to_existing_editor_bus_and_supports_undo() {
    let service = Service::new();
    let bus = bus();
    let owned = bus.clone();
    let seen = Arc::new(Mutex::new(vec![]));
    let id = service
        .start(
            request("edit memory"),
            || Ok(connection()),
            move |t, c, h| {
                job(
                    t,
                    c,
                    h,
                    owned,
                    &mut Patch {
                        calls: 0,
                        expected_revision: 0,
                        seen,
                    },
                )
            },
        )
        .unwrap();
    let pending = wait(&service, |s| s.turns.last().unwrap().pending.is_some());
    assert_eq!(pending.turns[0].text, "Working");
    let approval = pending.turns[0].pending.as_ref().unwrap();
    assert_eq!(approval.tool, "doc_patch");
    assert_eq!(approval.arguments["expected_revision"], 0);
    // A real editor read while approval waits proves the bus is not locked.
    assert_eq!(
        read(&service, &bus, Some(pending.version))
            .unwrap()
            .project_revision,
        Some(0)
    );
    service.session.decide(id, approval.id, true).unwrap();
    let snapshot = wait(&service, |s| !s.worker_running);
    assert!(matches!(snapshot.turns[0].status, Status::Completed));
    assert_eq!(snapshot.budget.used_tokens, 20);
    assert_eq!(
        snapshot.turns[0].report.as_ref().unwrap().transaction_count,
        1
    );
    let readback = read(&service, &bus, Some(snapshot.version)).unwrap();
    assert!(readback.session.is_none());
    assert_eq!(readback.project_revision, Some(1));
    let mut load = bus.lock().unwrap();
    let bus = load.bus_mut().unwrap();
    assert_eq!(bus.project().memory["agent"], "changed");
    let actor = serde_json::to_value(&bus.history()[0].actor).unwrap();
    assert_eq!(actor["origin"], "agent");
    let provenance = actor["conversation_id"].as_str().unwrap();
    assert!(provenance.parse::<ulid::Ulid>().is_ok());
    bus.undo().unwrap();
    assert!(!bus.project().memory.contains_key("agent"));
}

#[test]
fn concurrent_manual_edit_while_approval_waits_cannot_be_overwritten() {
    let service = Service::new();
    let bus = bus();
    let owned = bus.clone();
    service
        .start(
            request("stale edit"),
            || Ok(connection()),
            move |t, c, h| {
                job(
                    t,
                    c,
                    h,
                    owned,
                    &mut Patch {
                        calls: 0,
                        expected_revision: 0,
                        seen: Arc::new(Mutex::new(vec![])),
                    },
                )
            },
        )
        .unwrap();
    let pending = wait(&service, |s| s.turns[0].pending.is_some());
    bus.lock()
        .unwrap()
        .bus_mut()
        .unwrap()
        .execute(
            vec![Command::SetMemory {
                section: "manual".into(),
                text: "kept".into(),
            }],
            Actor::user("editor"),
            "Manual",
            Some(0),
        )
        .unwrap();
    let approval = pending.turns[0].pending.as_ref().unwrap();
    service
        .session
        .decide(approval.turn, approval.id, true)
        .unwrap();
    let final_state = wait(&service, |s| !s.worker_running);
    assert_eq!(
        final_state.turns[0]
            .report
            .as_ref()
            .unwrap()
            .transaction_count,
        0
    );
    assert!(!final_state.turns[0].report.as_ref().unwrap().tool_calls[0].succeeded);
    let mut load = bus.lock().unwrap();
    let bus = load.bus_mut().unwrap();
    assert_eq!(bus.revision(), 1);
    assert_eq!(bus.project().memory["manual"], "kept");
    assert!(!bus.project().memory.contains_key("agent"));
}

#[test]
fn context_is_retained_per_account_and_cleared_on_account_change() {
    let service = Service::new();
    let bus = bus();
    let seen = Arc::new(Mutex::new(vec![]));
    for (index, who) in [
        connection(),
        connection(),
        Connection::OAuth("other".into()),
    ]
    .into_iter()
    .enumerate()
    {
        let owned = bus.clone();
        let inputs = seen.clone();
        let mut request = request(&format!("turn {index}"));
        request.approval = ApprovalChoice::Auto;
        service
            .start(
                request,
                || Ok(who),
                move |t, c, h| {
                    job(
                        t,
                        c,
                        h,
                        owned,
                        &mut Patch {
                            calls: 1,
                            expected_revision: 0,
                            seen: inputs,
                        },
                    )
                },
            )
            .unwrap();
        wait(&service, |s| !s.worker_running);
    }
    let inputs = seen.lock().unwrap();
    assert!(inputs[1].iter().any(|v| v["content"] == "turn 0"));
    assert!(
        !inputs[2]
            .iter()
            .any(|v| v["content"] == "turn 0" || v["content"] == "turn 1")
    );
    assert_eq!(service.session.snapshot().unwrap().budget.used_tokens, 30);
}

#[test]
fn saved_project_host_runs_native_world_without_changing_authoring_state() {
    let temp = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Native bound host");
    project.settings.tick_rate = 10;
    let scene = Scene::new("Main");
    project.scenes.insert(scene.id.clone(), scene);
    let bus = Mutex::new(crate::project::LoadState::Ready(Box::new(
        CommandBus::new(project.clone()).unwrap(),
    )));
    let mut host = worker::host(&bus, Some(temp.path().to_path_buf())).unwrap();
    assert!(
        host.native_play_available()
            && host.shared_play_available()
            && host.project_files().is_some()
    );
    let result = host
        .play_run_checked(
            &project,
            0,
            serde_json::from_value(json!({"seconds":0.2})).unwrap(),
            &AtomicBool::new(false),
            &mut || Ok(()),
        )
        .unwrap();
    assert_eq!(result["state"]["tick"], 2);
    let mut load = bus.lock().unwrap();
    let bus = load.bus_mut().unwrap();
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert_eq!(
        serde_json::to_value(bus.project()).unwrap(),
        serde_json::to_value(project).unwrap()
    );
    let unsaved = worker::host(&Mutex::new(crate::project::LoadState::Loading), None).unwrap();
    assert!(!unsaved.native_play_available());
    assert!(unsaved.project_files().is_none());
}
