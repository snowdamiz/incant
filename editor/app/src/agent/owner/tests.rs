use super::*;
use crate::agent::{Connection, ContextKey, Service};
use incant_agent::{
    AgentError, Usage,
    provider::{Completion, Provider},
    session::*,
};
use incant_cmd::CommandBus;
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

type Bus = Arc<Mutex<crate::project::LoadState>>;
fn request() -> TurnRequest {
    TurnRequest {
        prompt: "Test native play".into(),
        model: "fixture".into(),
        approval: ApprovalChoice::Auto,
        view_context: None,
    }
}
fn connection() -> Connection {
    Connection::OAuth("fixture-account".into())
}
fn wait(service: &Service) -> SessionSnapshot {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let state = service.session.snapshot().unwrap();
        if !state.worker_running {
            return state;
        }
        assert!(Instant::now() < until, "native owner did not settle");
        thread::sleep(Duration::from_millis(2));
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
struct Calls {
    next: Option<Vec<Value>>,
    result: mpsc::Sender<(thread::ThreadId, Vec<Value>)>,
}
impl Provider for Calls {
    fn model(&self) -> &str {
        "fixture"
    }
    fn complete(
        &mut self,
        input: &[Value],
        _: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        let output = match self.next.take() {
            Some(calls) => calls,
            None => {
                self.result
                    .send((thread::current().id(), input.to_vec()))
                    .unwrap();
                vec![]
            }
        };
        Ok(Completion {
            output,
            usage: Usage {
                input_tokens: 3,
                output_tokens: 2,
            },
        })
    }
}
fn tool(name: &str, args: Value) -> Value {
    json!({"type":"function_call","call_id":"current","name":name,"arguments":args.to_string()})
}
fn turn(
    service: &Service,
    bus: &Bus,
    root: Option<PathBuf>,
    who: Connection,
    call: Value,
) -> (Value, thread::ThreadId) {
    let (send, receive) = mpsc::channel();
    let owned = bus.clone();
    service
        .start(
            request(),
            || Ok(who),
            move |t, c, client| {
                client.run(t, move |state, t| {
                    state.run_with(
                        t,
                        ContextKey::Fixture(c),
                        owned,
                        root,
                        &mut Calls {
                            next: Some(vec![call]),
                            result: send,
                        },
                    )
                })
            },
        )
        .unwrap();
    let (id, input) = receive.recv_timeout(Duration::from_secs(10)).unwrap();
    let snapshot = wait(service);
    assert!(matches!(
        snapshot.turns.last().unwrap().status,
        Status::Completed
    ));
    let output = input
        .iter()
        .rev()
        .find(|v| v["type"] == "function_call_output")
        .unwrap();
    (
        serde_json::from_str(output["output"].as_str().unwrap()).unwrap(),
        id,
    )
}
fn fixture(root: &std::path::Path) -> (Bus, String) {
    let mut project = Project::empty("Persistent native host");
    project.settings.tick_rate = 10;
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    let id = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[2.,0.,0.]}));
    scene.entities.insert(id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    std::fs::write(root.join("project.json"), project.canonical_text().unwrap()).unwrap();
    std::fs::write(root.join("untouched.ts"), "// source stays authored\n").unwrap();
    let bus =
        CommandBus::persistent_with_sources(root.join("project.journal"), project, root).unwrap();
    (
        Arc::new(Mutex::new(crate::project::LoadState::Ready(Box::new(bus)))),
        id,
    )
}
fn files(root: &std::path::Path) -> Vec<Vec<u8>> {
    ["project.json", "untouched.ts", "project.journal"]
        .map(|p| std::fs::read(root.join(p)).unwrap_or_default())
        .to_vec()
}

#[test]
fn real_native_play_survives_turns_and_failed_candidates_without_authoring_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let (bus, entity) = fixture(temp.path());
    let before = files(temp.path());
    let document = bus
        .lock()
        .unwrap()
        .bus_mut()
        .unwrap()
        .project()
        .canonical_text()
        .unwrap();
    let service = Service::new();
    let root = Some(temp.path().to_path_buf());
    let (played, owner_thread) = turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool("play_run", json!({"seconds":0.5})),
    );
    assert_eq!(played["state"]["tick"], 5);
    assert_eq!(played["state"]["script_source_parsing"], false);
    assert_eq!(played["state"]["gpu_submitted"], false);
    let path = format!("/entities/{entity}/transform/translation");
    let (queried, second_thread) = turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool("play_state_query", json!({"path":path})),
    );
    assert_eq!(queried["tick"], 5);
    assert_eq!(queried["untrusted_runtime_data"], json!([1., 0., 0.]));
    assert_eq!(owner_thread, second_thread);
    // This candidate gets through construction and fails on a real disconnected
    // gamepad event; the old VM/world must remain retained on the owner thread.
    let (failed, third_thread) = turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool(
            "play_run",
            json!({"seconds":0.2,
        "inputs_script":{"format":"incant-input","version":1,"tick_rate":10,"start_tick":0,"ticks":2,
            "frames":[{"tick":1,"events":[{"type":"gamepad_axis","id":0,"axis":"left_x","value":0.5}]}]}}),
        ),
    );
    assert!(
        failed["error"].as_str().unwrap().contains("connected"),
        "{failed}"
    );
    assert_eq!(third_thread, owner_thread);
    let (retained, _) = turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool("play_state_query", json!({"path":path})),
    );
    assert_eq!(retained, queried);
    let (replacement, _) = turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool("play_run", json!({"seconds":0.2})),
    );
    assert_eq!(replacement["state"]["tick"], 2);
    let (new, _) = turn(
        &service,
        &bus,
        root,
        connection(),
        tool("play_state_query", json!({"path":path})),
    );
    assert_eq!(new["tick"], 2);
    assert!((new["untrusted_runtime_data"][0].as_f64().unwrap() - 0.4).abs() < 1e-12);
    let mut load = bus.lock().unwrap();
    let bus = load.bus_mut().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), document);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert_eq!(files(temp.path()), before);
    service.shutdown();
}

#[test]
fn account_project_bus_and_root_changes_discard_retained_play() {
    let temp = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (bus, _) = fixture(temp.path());
    let root = Some(temp.path().to_path_buf());
    let service = Service::new();
    let play = || tool("play_run", json!({"seconds":0.1}));
    let query = || tool("play_state_query", json!({"path":"/tick"}));
    turn(&service, &bus, root.clone(), connection(), play());
    let changed_account = Connection::OAuth("replacement-receipt".into());
    let (empty, _) = turn(
        &service,
        &bus,
        root.clone(),
        changed_account.clone(),
        query(),
    );
    assert!(empty["error"].as_str().unwrap().contains("no play session"));
    turn(
        &service,
        &bus,
        root.clone(),
        changed_account.clone(),
        play(),
    );
    let (empty, _) = turn(
        &service,
        &bus,
        Some(other.path().to_path_buf()),
        changed_account.clone(),
        query(),
    );
    assert!(empty["error"].as_str().unwrap().contains("no play session"));
    turn(
        &service,
        &bus,
        root.clone(),
        changed_account.clone(),
        play(),
    );
    // Same project ID on another bus must not reuse the old native state.
    let project = bus.lock().unwrap().bus_mut().unwrap().project().clone();
    let new_bus = Arc::new(Mutex::new(crate::project::LoadState::Ready(Box::new(
        CommandBus::new(project).unwrap(),
    ))));
    let (empty, _) = turn(
        &service,
        &new_bus,
        root.clone(),
        changed_account.clone(),
        query(),
    );
    assert!(empty["error"].as_str().unwrap().contains("no play session"));
    turn(
        &service,
        &bus,
        root.clone(),
        changed_account.clone(),
        play(),
    );
    *bus.lock().unwrap() = crate::project::LoadState::Ready(Box::new(
        CommandBus::new(Project::empty("Another")).unwrap(),
    ));
    let (empty, _) = turn(&service, &bus, root, changed_account, query());
    assert!(empty["error"].as_str().unwrap().contains("no play session"));
    service.shutdown();
}

struct Exit(Option<mpsc::Sender<()>>);
impl Drop for Exit {
    fn drop(&mut self) {
        if let Some(send) = self.0.take() {
            let _ = send.send(());
        }
    }
}
thread_local! { static EXIT: RefCell<Exit> = const { RefCell::new(Exit(None)) }; }
fn signal_exit(send: mpsc::Sender<()>) {
    EXIT.with(|slot| slot.borrow_mut().0 = Some(send));
}

#[test]
fn dropping_idle_owner_disconnects_retained_clients_and_exits_native_thread() {
    let service = Service::new();
    let client = service.owner.client();
    let retained = client.clone();
    let (exited, observed) = mpsc::channel();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, _, _| {
                client.run(turn, move |_, turn| {
                    signal_exit(exited);
                    empty(turn)
                })
            },
        )
        .unwrap();
    wait(&service);
    drop(service);
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    let next = Service::new();
    next.start(
        request(),
        || Ok(connection()),
        move |turn, _, _| retained.run(turn, |_, _| panic!("must not execute")),
    )
    .unwrap();
    let snapshot = wait(&next);
    assert_eq!(snapshot.turns[0].failure, Some(Failure::WorkerPanicked));
    assert!(snapshot.accounting_uncertain);
}

#[test]
fn owner_panic_closes_channel_and_exhausts_unknown_usage_after_thread_exit() {
    let service = Service::new();
    let (exited, observed) = mpsc::channel();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, _, owner| {
                owner.run(turn, move |_, _| {
                    signal_exit(exited);
                    panic!("injected owner panic");
                })
            },
        )
        .unwrap();
    let snapshot = wait(&service);
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(snapshot.turns[0].failure, Some(Failure::WorkerPanicked));
    assert!(snapshot.accounting_uncertain);
    assert_eq!(snapshot.budget.used_tokens, snapshot.budget.max_tokens);
    assert_eq!(
        service
            .start(request(), || Ok(connection()), |turn, _, _| empty(turn))
            .unwrap_err()
            .code,
        "agent.budget"
    );
}

#[test]
fn blocked_owner_keeps_session_busy_and_shutdown_cancels_without_joining() {
    let service = Service::new();
    let (started, entered) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (exited, observed) = mpsc::channel();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, _, owner| {
                owner.run(turn, move |_, turn| {
                    signal_exit(exited);
                    started.send(()).unwrap();
                    resume.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert!(turn.cancellation().load(Ordering::Acquire));
                    assert_eq!(turn.stream("late"), Err(SessionError::StaleTurn));
                    empty(turn)
                })
            },
        )
        .unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        service
            .start(request(), || Ok(connection()), |turn, _, _| empty(turn))
            .unwrap_err()
            .code,
        "agent.busy"
    );
    service.shutdown();
    assert!(service.session.snapshot().unwrap().worker_running);
    release.send(()).unwrap();
    let snapshot = wait(&service);
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(snapshot.turns[0].status, Status::Cancelled);
    assert!(!snapshot.accounting_uncertain);
    assert_eq!(snapshot.budget.used_tokens, 0);
}

#[test]
fn shutdown_wakes_approval_on_the_dedicated_owner() {
    let service = Service::new();
    let (exited, observed) = mpsc::channel();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, _, owner| {
                owner.run(turn, move |_, turn| {
                    signal_exit(exited);
                    assert!(!turn.approve("doc_patch", &json!({"ops":[]})));
                    empty(turn)
                })
            },
        )
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while service.session.snapshot().unwrap().turns[0]
        .pending
        .is_none()
    {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(2));
    }
    service.shutdown();
    let snapshot = wait(&service);
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(snapshot.turns[0].status, Status::Cancelled);
    assert!(snapshot.turns[0].pending.is_none());
}

#[test]
fn project_change_during_receipt_check_cannot_use_previous_host() {
    struct Switch {
        bus: Bus,
        changed: AtomicBool,
    }
    impl Provider for Switch {
        fn model(&self) -> &str {
            "fixture"
        }
        fn check_active(&self) -> Result<(), AgentError> {
            if !self.changed.swap(true, Ordering::Relaxed) {
                *self.bus.lock().unwrap() = crate::project::LoadState::Ready(Box::new(
                    CommandBus::new(Project::empty("Switched")).unwrap(),
                ));
            }
            Ok(())
        }
        fn complete(
            &mut self,
            _: &[Value],
            _: &[Value],
            _: u64,
            _: &mut dyn FnMut(&str),
        ) -> Result<Completion, AgentError> {
            panic!("stale host must not request a response")
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (bus, _) = fixture(temp.path());
    let root = Some(temp.path().to_path_buf());
    let service = Service::new();
    turn(
        &service,
        &bus,
        root.clone(),
        connection(),
        tool("play_run", json!({"seconds":0.1})),
    );
    let owned = bus.clone();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, who, owner| {
                owner.run(turn, move |state, turn| {
                    state.run_with(
                        turn,
                        ContextKey::Fixture(who),
                        owned.clone(),
                        root,
                        &mut Switch {
                            bus: owned,
                            changed: AtomicBool::new(false),
                        },
                    )
                })
            },
        )
        .unwrap();
    let snapshot = wait(&service);
    assert_eq!(
        snapshot.turns.last().unwrap().failure,
        Some(Failure::ProjectChanged)
    );
    assert_eq!(snapshot.turns.last().unwrap().charged_tokens, 0);
    service.shutdown();
}

#[test]
fn shutdown_before_enqueue_remains_a_known_uncharged_cancellation() {
    let service = Service::new();
    let (started, entered) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    service
        .start(
            request(),
            || Ok(connection()),
            move |turn, _, owner| {
                started.send(()).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
                owner.run(turn, |_, _| panic!("cancelled work must not start"))
            },
        )
        .unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    service.shutdown();
    release.send(()).unwrap();
    let snapshot = wait(&service);
    assert_eq!(snapshot.turns[0].status, Status::Cancelled);
    assert_eq!(snapshot.turns[0].charged_tokens, 0);
    assert!(!snapshot.accounting_uncertain);
    assert!(service.owner.client.0.lock().unwrap().thread.is_none());
}
