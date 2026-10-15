use super::*;
use crate::{TurnReport, Usage};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Barrier, mpsc},
    time::{Duration, Instant},
};
const PROJECT: &str = "00000000000000000000000001";
fn ledger(directory: &Path) -> Ledger {
    Ledger::open_in(directory, PROJECT).unwrap()
}
fn record_path(directory: &Path) -> PathBuf {
    directory.join(format!("{PROJECT}.json"))
}
fn record(directory: &Path) -> Value {
    serde_json::from_slice(&fs::read(record_path(directory)).unwrap()).unwrap()
}
fn request() -> TurnRequest {
    TurnRequest {
        prompt: "fixture".into(),
        model: "fixture".into(),
        approval: ApprovalChoice::Auto,
        view_context: None,
    }
}
fn session() -> Session {
    Session::new(Budget {
        max_tokens: 10,
        used_tokens: 10,
    })
}
fn outcome(context: TurnContext, used: u64, reported: u64) -> TurnOutcome {
    TurnOutcome {
        budget: Budget {
            used_tokens: used,
            ..context.budget
        },
        result: Ok(TurnReport {
            steps: 1,
            usage: Usage {
                input_tokens: reported,
                output_tokens: 0,
            },
            transaction_ids: vec![],
            tool_calls: vec![],
        }),
    }
}
fn complete(session: &Session) -> SessionSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = session.snapshot().unwrap();
        if !snapshot.worker_running {
            return snapshot;
        }
        assert!(Instant::now() < deadline, "worker did not finish");
        thread::sleep(Duration::from_millis(1));
    }
}
fn seed(directory: &Path, used: u64) {
    let lease = ledger(directory).reserve(100).unwrap();
    lease
        .complete(
            &Budget {
                max_tokens: 100,
                used_tokens: used,
            },
            false,
        )
        .unwrap();
}

#[test]
fn durable_admission_restores_raised_cap_and_settles_before_terminal_publication() {
    let temp = tempfile::tempdir().unwrap();
    seed(temp.path(), 10);
    let session = session();
    let directory = temp.path().to_owned();
    session
        .start_durable(request(), ledger(temp.path()), 100, move |context| {
            assert_eq!(context.budget.used_tokens, 10);
            assert_eq!(context.budget.max_tokens, 100);
            let saved = record(&directory);
            assert_eq!(saved["charged_tokens"], 100);
            assert_eq!(saved["pending"]["initial_tokens"], 10);
            outcome(context, 17, 7)
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = session.snapshot().unwrap();
        if snapshot.turns[0].status == Status::Completed {
            // Even if thread teardown is still running, published completion is durable.
            assert_eq!(record(temp.path())["charged_tokens"], 17);
            assert!(record(temp.path())["pending"].is_null());
            break;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let snapshot = complete(&session);
    assert_eq!(snapshot.budget.used_tokens, 17);
    assert_eq!(snapshot.turns[0].charged_tokens, 7);
    assert!(!snapshot.accounting_uncertain);
    assert_eq!(
        ledger(temp.path())
            .snapshot(100)
            .unwrap()
            .budget
            .used_tokens,
        17
    );
}

#[test]
fn panic_and_inconsistent_receipts_keep_full_reservation_on_reopen() {
    for mode in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        seed(temp.path(), 4);
        let session = session();
        session
            .start_durable(
                request(),
                ledger(temp.path()),
                100,
                move |context| match mode {
                    0 => panic!("fixture job panic"),
                    1 => {
                        let mut result = outcome(context, 5, 1);
                        result.budget.max_tokens = 99;
                        result
                    }
                    2 => outcome(context, 3, 0),
                    _ => outcome(context, 5, 20),
                },
            )
            .unwrap();
        let snapshot = complete(&session);
        assert_eq!(snapshot.budget.used_tokens, 100);
        assert!(snapshot.accounting_uncertain);
        assert_eq!(snapshot.turns[0].charged_tokens, 96);
        assert_eq!(
            snapshot.turns[0].failure,
            Some(if mode == 0 {
                Failure::WorkerPanicked
            } else {
                Failure::BudgetAccounting
            })
        );
        let restored = ledger(temp.path()).snapshot(100).unwrap();
        assert_eq!(restored.budget.used_tokens, 100);
        assert!(restored.accounting_uncertain && restored.recovered_reservation);
    }
}

#[test]
fn cancellation_before_reservation_invokes_no_job_and_spends_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let session = session();
    let store = ledger(temp.path());
    let restored = store.snapshot(100).unwrap();
    let (go, wait) = mpsc::channel();
    let id = session
        .start_inner(
            request(),
            Some((store, restored)),
            |_| panic!("job must not run"),
            move |name, job| {
                thread::Builder::new().name(name).spawn(move || {
                    wait.recv().unwrap();
                    job();
                })
            },
        )
        .unwrap();
    session.cancel(id).unwrap();
    go.send(()).unwrap();
    let snapshot = complete(&session);
    assert_eq!(snapshot.turns[0].status, Status::Cancelled);
    assert_eq!(snapshot.budget.used_tokens, 0);
    assert!(!record_path(temp.path()).exists());
    assert_eq!(
        ledger(temp.path())
            .snapshot(100)
            .unwrap()
            .budget
            .used_tokens,
        0
    );
}

#[test]
fn rejected_metadata_and_spawn_do_not_reserve() {
    for mode in 0..2 {
        let temp = tempfile::tempdir().unwrap();
        let session = session();
        let store = ledger(temp.path());
        let restored = store.snapshot(100).unwrap();
        let mut req = request();
        if mode == 0 {
            req.prompt.clear();
        }
        let result = session.start_inner(
            req,
            Some((store, restored)),
            |_| unreachable!(),
            |_, _| Err(std::io::Error::other("injected spawn refusal")),
        );
        assert_eq!(
            result,
            Err(if mode == 0 {
                SessionError::InvalidRequest
            } else {
                SessionError::Spawn
            })
        );
        assert!(!record_path(temp.path()).exists());
        assert_eq!(
            ledger(temp.path())
                .snapshot(100)
                .unwrap()
                .budget
                .used_tokens,
            0
        );
    }
}

#[test]
fn busy_start_and_restore_cannot_replace_active_budget_or_reserve() {
    let temp = tempfile::tempdir().unwrap();
    let session = Session::new(Budget {
        max_tokens: 100,
        used_tokens: 20,
    });
    let (go, wait) = mpsc::channel();
    session
        .start(request(), move |context| {
            wait.recv().unwrap();
            outcome(context, 20, 0)
        })
        .unwrap();
    let before = session.snapshot().unwrap();
    assert_eq!(
        session.start_durable(request(), ledger(temp.path()), 200, |_| unreachable!()),
        Err(SessionError::Busy)
    );
    let store = ledger(temp.path());
    assert_eq!(
        session.restore_budget_if_idle(store.snapshot(200).unwrap()),
        Err(SessionError::Busy)
    );
    let after = session.snapshot().unwrap();
    assert_eq!(after.version, before.version);
    assert_eq!(after.budget.max_tokens, 100);
    assert_eq!(after.budget.used_tokens, 20);
    assert!(!record_path(temp.path()).exists());
    go.send(()).unwrap();
    complete(&session);
}

#[test]
fn idle_restore_is_version_stable_and_retains_recovered_uncertainty() {
    let temp = tempfile::tempdir().unwrap();
    drop(ledger(temp.path()).reserve(100).unwrap());
    let session = session();
    let store = ledger(temp.path());
    session
        .restore_budget_if_idle(store.snapshot(200).unwrap())
        .unwrap();
    let snapshot = session.snapshot().unwrap();
    assert_eq!(snapshot.budget.used_tokens, 100);
    assert!(snapshot.accounting_uncertain);
    session
        .restore_budget_if_idle(store.snapshot(200).unwrap())
        .unwrap();
    assert!(
        session
            .snapshot_if_changed(Some(snapshot.version))
            .unwrap()
            .is_none()
    );
    drop(store);
    session
        .start_durable(request(), ledger(temp.path()), 200, |context| {
            outcome(context, 110, 10)
        })
        .unwrap();
    let snapshot = complete(&session);
    assert_eq!(snapshot.budget.used_tokens, 110);
    assert!(snapshot.accounting_uncertain);
    assert!(!snapshot.turns[0].accounting_uncertain);
    assert!(
        ledger(temp.path())
            .snapshot(200)
            .unwrap()
            .accounting_uncertain
    );
}

#[test]
fn exhausted_and_invalid_caps_do_not_start_or_reserve() {
    let temp = tempfile::tempdir().unwrap();
    seed(temp.path(), 30);
    let before = fs::read(record_path(temp.path())).unwrap();
    let session = session();
    for max_tokens in [0, 30, MAX_TOKEN_CAP + 1] {
        assert_eq!(
            session.start_durable(
                request(),
                ledger(temp.path()),
                max_tokens,
                |_| unreachable!()
            ),
            Err(if max_tokens > MAX_TOKEN_CAP {
                SessionError::InvalidRequest
            } else {
                SessionError::Budget
            })
        );
        assert_eq!(fs::read(record_path(temp.path())).unwrap(), before);
        assert!(!session.snapshot().unwrap().worker_running);
    }
}

#[test]
fn provider_without_usage_keeps_conservative_receipt_and_uncertainty() {
    let temp = tempfile::tempdir().unwrap();
    let session = session();
    session
        .start_durable(request(), ledger(temp.path()), 100, |context| TurnOutcome {
            budget: Budget {
                used_tokens: 70,
                ..context.budget
            },
            result: Err(AgentError::Response {
                reason: "fixture",
                usage: None,
            }),
        })
        .unwrap();
    let snapshot = complete(&session);
    assert_eq!(snapshot.budget.used_tokens, 70);
    assert!(snapshot.accounting_uncertain);
    assert_eq!(snapshot.turns[0].failure, Some(Failure::Provider));
    let restored = ledger(temp.path()).snapshot(100).unwrap();
    assert_eq!(restored.budget.used_tokens, 70);
    assert!(restored.accounting_uncertain);
    assert!(!restored.recovered_reservation);
}

#[test]
fn over_cap_receipt_stays_charged_and_saturates_safely() {
    let temp = tempfile::tempdir().unwrap();
    let session = session();
    session
        .start_durable(request(), ledger(temp.path()), 100, |context| {
            outcome(context, u64::MAX, u64::MAX)
        })
        .unwrap();
    let snapshot = complete(&session);
    assert_eq!(snapshot.budget.used_tokens, MAX_TOKEN_CAP);
    assert!(snapshot.accounting_uncertain);
    assert!(snapshot.turns[0].accounting_uncertain);
    assert_eq!(snapshot.turns[0].failure, Some(Failure::BudgetExhausted));
    assert_eq!(
        ledger(temp.path())
            .snapshot(MAX_TOKEN_CAP)
            .unwrap()
            .budget
            .remaining(),
        0
    );
}

#[test]
fn reservation_failure_invokes_no_job_and_reports_static_accounting_failure() {
    let temp = tempfile::tempdir().unwrap();
    let session = session();
    let store = ledger(temp.path());
    fs::create_dir(record_path(temp.path())).unwrap();
    session
        .start_durable(request(), store, 100, |_| panic!("job must not run"))
        .unwrap();
    let snapshot = complete(&session);
    assert_eq!(snapshot.turns[0].failure, Some(Failure::BudgetAccounting));
    assert_eq!(snapshot.budget.used_tokens, 100);
    assert!(snapshot.accounting_uncertain);
    assert!(record_path(temp.path()).is_dir());
}

#[test]
fn settlement_failure_keeps_pending_receipt_and_never_publishes_success() {
    for measured in [12, 150, u64::MAX] {
        let temp = tempfile::tempdir().unwrap();
        let session = session();
        let directory = temp.path().to_owned();
        session
            .start_durable(request(), ledger(temp.path()), 100, move |context| {
                fs::rename(record_path(&directory), directory.join("reserved.json")).unwrap();
                fs::create_dir(record_path(&directory)).unwrap();
                outcome(context, measured, measured)
            })
            .unwrap();
        let snapshot = complete(&session);
        assert_eq!(snapshot.turns[0].status, Status::Failed);
        assert_eq!(snapshot.turns[0].failure, Some(Failure::BudgetAccounting));
        assert_eq!(
            snapshot.budget.used_tokens,
            measured.clamp(100, MAX_TOKEN_CAP)
        );
        assert!(snapshot.accounting_uncertain);
        assert!(snapshot.turns[0].accounting_uncertain);
        assert!(snapshot.turns[0].report.is_none());
        fs::remove_dir(record_path(temp.path())).unwrap();
        fs::rename(temp.path().join("reserved.json"), record_path(temp.path())).unwrap();
        // Failed storage cannot durably record an exact later overrun. Recovery
        // retains the original conservative reservation and its uncertainty.
        let restored = ledger(temp.path()).snapshot(100).unwrap();
        assert_eq!(restored.budget.used_tokens, 100);
        assert!(restored.recovered_reservation && restored.accounting_uncertain);
    }
}

#[test]
fn racing_idle_restore_and_start_have_one_atomic_budget_choice() {
    for _ in 0..16 {
        let temp = tempfile::tempdir().unwrap();
        seed(temp.path(), 20);
        let store = ledger(temp.path());
        let restored = store.snapshot(200).unwrap();
        let session = Arc::new(Session::new(Budget {
            max_tokens: 100,
            used_tokens: 0,
        }));
        let gate = Arc::new(Barrier::new(2));
        let other = session.clone();
        let ready = gate.clone();
        let restore_thread = thread::spawn(move || {
            ready.wait();
            other.restore_budget_if_idle(restored)
        });
        let (go, wait) = mpsc::channel();
        let (send, receive) = mpsc::channel();
        gate.wait();
        session
            .start(request(), move |context| {
                let used = context.budget.used_tokens;
                send.send(context.budget.clone()).unwrap();
                wait.recv().unwrap();
                outcome(context, used, 0)
            })
            .unwrap();
        let restored = restore_thread.join().unwrap();
        let selected = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        let snapshot = session.snapshot().unwrap();
        assert_eq!(
            (snapshot.budget.max_tokens, snapshot.budget.used_tokens),
            (selected.max_tokens, selected.used_tokens)
        );
        match restored {
            Ok(()) => assert_eq!((selected.max_tokens, selected.used_tokens), (200, 20)),
            Err(SessionError::Busy) => {
                assert_eq!((selected.max_tokens, selected.used_tokens), (100, 0))
            }
            other => panic!("unexpected restore outcome: {other:?}"),
        }
        go.send(()).unwrap();
        complete(&session);
    }
}
