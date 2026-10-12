use super::*;
use crate::agent::Connection;
use incant_agent::{
    Usage,
    session::{ApprovalChoice, Status, TurnContext, TurnOutcome, TurnRequest},
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{AgentBudgetSettings, Project};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
fn bus() -> Mutex<LoadState> {
    let mut project = Project::empty("Durable budget fixture");
    project.settings.agent_budget.max_tokens = 100;
    Mutex::new(LoadState::Ready(Box::new(
        CommandBus::new(project).unwrap(),
    )))
}
fn open(directory: &Path, scope: &Scope) -> Ledger {
    Ledger::open_in(directory, &scope.project_id).unwrap()
}
fn set_cap(bus: &Mutex<LoadState>, max_tokens: u64) {
    bus.lock()
        .unwrap()
        .bus_mut()
        .unwrap()
        .execute(
            vec![Command::SetAgentBudget {
                settings: AgentBudgetSettings { max_tokens },
            }],
            Actor::user("test"),
            "Change cap",
            None,
        )
        .unwrap();
}
fn request() -> TurnRequest {
    TurnRequest {
        prompt: "fixture".into(),
        model: "fixture".into(),
        approval: ApprovalChoice::Always,
    }
}
fn complete(turn: TurnContext, charge: u64) -> TurnOutcome {
    let mut budget = turn.budget;
    budget.used_tokens += charge;
    TurnOutcome {
        budget,
        result: Ok(incant_agent::TurnReport {
            steps: 1,
            usage: Usage {
                input_tokens: charge,
                output_tokens: 0,
            },
            transaction_ids: vec![],
            tool_calls: vec![],
        }),
    }
}
fn wait(service: &Service) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while service.session.snapshot().unwrap().worker_running {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn restores_real_usage_without_holding_project_or_admission_locks_during_io() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let lease = open(temp.path(), &scope).reserve(100).unwrap();
    let mut budget = lease.budget();
    budget.used_tokens = 17;
    lease.complete(&budget, false).unwrap();
    let service = Service::new();
    restore(&service, &bus, |id| {
        assert!(bus.try_lock().is_ok());
        assert!(service.closed.try_lock().is_ok());
        Ledger::open_in(temp.path(), id)
    })
    .unwrap();
    let first = service.session.snapshot().unwrap();
    assert_eq!(first.budget.max_tokens, 100);
    assert_eq!(first.budget.used_tokens, 17);
    restore(&service, &bus, |id| Ledger::open_in(temp.path(), id)).unwrap();
    assert_eq!(service.session.snapshot().unwrap().version, first.version);
}

#[test]
fn refuses_stale_project_cap_after_storage_read_without_overwriting_session() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let service = Service::new();
    let initial = service.session.snapshot().unwrap();
    let error = restore(&service, &bus, |id| {
        set_cap(&bus, 50);
        Ledger::open_in(temp.path(), id)
    })
    .unwrap_err();
    assert_eq!(error.code, "agent.project-changed");
    let after = service.session.snapshot().unwrap();
    assert_eq!(after.version, initial.version);
    assert_eq!(after.budget.max_tokens, initial.budget.max_tokens);
}

#[test]
fn another_window_owner_returns_explicit_busy_without_resetting_usage() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let held = open(temp.path(), &scope).reserve(100).unwrap();
    let service = Service::new();
    let error = restore(&service, &bus, |id| Ledger::open_in(temp.path(), id)).unwrap_err();
    assert_eq!(error.code, "agent.budget-busy");
    drop(held);
    restore(&service, &bus, |id| Ledger::open_in(temp.path(), id)).unwrap();
    let snapshot = service.session.snapshot().unwrap();
    assert_eq!(snapshot.budget.used_tokens, 100);
    assert!(snapshot.accounting_uncertain);
}

#[test]
fn rejected_admission_does_not_reserve_or_run_the_provider() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let service = Service::new();
    let called = Arc::new(AtomicBool::new(false));
    let seen = called.clone();
    assert!(
        service
            .start_with(
                request(),
                Some((open(temp.path(), &scope), 100)),
                || Err::<(Connection, ()), _>(Error::unavailable()),
                move |turn, _, _| {
                    seen.store(true, Ordering::Release);
                    complete(turn, 20)
                }
            )
            .is_err()
    );
    assert!(!called.load(Ordering::Acquire));
    let snapshot = open(temp.path(), &scope).snapshot(100).unwrap();
    assert_eq!(snapshot.budget.used_tokens, 0);
    assert!(!snapshot.accounting_uncertain);
}

#[test]
fn changing_cap_wakes_approval_and_settles_usage_before_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let service = Service::new();
    let (started, active) = mpsc::channel();
    service
        .start_with(
            request(),
            Some((open(temp.path(), &scope), 100)),
            || Ok((Connection::OAuth("synthetic".into()), ())),
            move |turn, _, _| {
                started.send(()).unwrap();
                assert!(!turn.approve("doc_patch", &serde_json::json!({"ops":[]})));
                assert!(turn.cancellation().load(Ordering::Acquire));
                complete(turn, 17)
            },
        )
        .unwrap();
    active.recv_timeout(Duration::from_secs(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while service.session.snapshot().unwrap().turns[0]
        .pending
        .is_none()
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    set_cap(&bus, 0);
    restore(&service, &bus, |_| {
        panic!("active turn must retain its own ledger")
    })
    .unwrap();
    wait(&service);
    let stopped = service.session.snapshot().unwrap();
    assert_eq!(stopped.turns[0].status, Status::Cancelled);
    assert_eq!(stopped.budget.used_tokens, 17);
    let reopened = Service::new();
    restore(&reopened, &bus, |id| Ledger::open_in(temp.path(), id)).unwrap();
    let restored = reopened.session.snapshot().unwrap();
    assert_eq!(restored.budget.max_tokens, 0);
    assert_eq!(restored.budget.used_tokens, 17);
    assert!(!restored.accounting_uncertain);
}

#[test]
fn budget_scope_ignores_ordinary_edits_but_fences_identity_and_cap() {
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    bus.lock()
        .unwrap()
        .bus_mut()
        .unwrap()
        .execute(
            vec![Command::SetMemory {
                section: "note".into(),
                text: "edit".into(),
            }],
            Actor::user("test"),
            "Ordinary edit",
            None,
        )
        .unwrap();
    assert!(scope.check_agent(&bus).is_ok());
    set_cap(&bus, 101);
    assert!(matches!(
        scope.check_agent(&bus),
        Err(AgentError::ProjectChanged)
    ));
    *bus.lock().unwrap() =
        LoadState::Ready(Box::new(CommandBus::new(Project::empty("Other")).unwrap()));
    assert!(matches!(
        scope.check_agent(&bus),
        Err(AgentError::ProjectChanged)
    ));
}

#[test]
fn delayed_cap_read_cannot_cancel_a_newer_worker_with_the_new_cap() {
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let service = Service::new();
    let (finish_a, ending_a) = mpsc::channel();
    service
        .start_with(
            request(),
            Some((open(temp.path(), &scope), 100)),
            || Ok((Connection::OAuth("synthetic".into()), ())),
            move |turn, _, _| {
                ending_a.recv().unwrap();
                complete(turn, 10)
            },
        )
        .unwrap();
    let observed_a = service.session.snapshot().unwrap();
    finish_a.send(()).unwrap();
    wait(&service);
    set_cap(&bus, 200);
    let (finish_b, ending_b) = mpsc::channel();
    let id_b = service
        .start_with(
            request(),
            Some((open(temp.path(), &scope), 200)),
            || Ok((Connection::OAuth("synthetic".into()), ())),
            move |turn, _, _| {
                ending_b.recv().unwrap();
                assert!(!turn.cancellation().load(Ordering::Acquire));
                complete(turn, 10)
            },
        )
        .unwrap();
    cancel_changed_cap(&service, &bus, &observed_a).unwrap();
    let current = service.session.snapshot().unwrap();
    assert_eq!(current.turns.last().unwrap().id, id_b);
    assert_eq!(current.turns.last().unwrap().status, Status::Running);
    finish_b.send(()).unwrap();
    wait(&service);
    assert_eq!(
        service
            .session
            .snapshot()
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        Status::Completed
    );
}

#[test]
fn project_cap_remains_locked_until_durable_admission_finishes() {
    struct AdmissionGuard<'a> {
        _load: std::sync::MutexGuard<'a, LoadState>,
        service: &'a Service,
        bus: &'a Mutex<LoadState>,
        checked: &'a AtomicBool,
    }
    impl Drop for AdmissionGuard<'_> {
        fn drop(&mut self) {
            // The ready scope must survive through actual worker admission.
            // Releasing it inside `ready` would allow a cap edit in that gap.
            assert!(self.bus.try_lock().is_err());
            let snapshot = self.service.session.snapshot().unwrap();
            assert!(snapshot.worker_running);
            assert_eq!(snapshot.budget.max_tokens, 100);
            self.checked.store(true, Ordering::Release);
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let bus = bus();
    let scope = Scope::read(&bus).unwrap();
    let service = Service::new();
    let checked = AtomicBool::new(false);
    let (finish, ending) = mpsc::channel();
    service
        .start_with(
            request(),
            Some((open(temp.path(), &scope), 100)),
            || {
                Ok((
                    Connection::OAuth("synthetic".into()),
                    AdmissionGuard {
                        _load: scope.guard(&bus)?,
                        service: &service,
                        bus: &bus,
                        checked: &checked,
                    },
                ))
            },
            move |turn, _, _| {
                ending.recv_timeout(Duration::from_secs(10)).unwrap();
                complete(turn, 10)
            },
        )
        .unwrap();
    assert!(checked.load(Ordering::Acquire));
    // Authoring becomes available immediately after acceptance, not after the
    // provider finishes. The worker owns no project or admission lock.
    assert!(bus.try_lock().is_ok());
    assert!(service.closed.try_lock().is_ok());
    finish.send(()).unwrap();
    wait(&service);
}
