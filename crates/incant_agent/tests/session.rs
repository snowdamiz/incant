//! Deterministic worker/channel controls; no provider account or network access.
use incant_agent::{AgentError, Budget, ToolObservation, TurnReport, Usage, session::*};
use serde_json::json;
use std::{
    sync::{Arc, Barrier, mpsc},
    thread,
    time::{Duration, Instant},
};
fn request() -> TurnRequest {
    TurnRequest {
        prompt: "Change the scene".into(),
        model: "fixture-model".into(),
        approval: ApprovalChoice::Destructive,
        view_context: None,
    }
}
fn session() -> Session {
    Session::new(Budget {
        max_tokens: 1000,
        used_tokens: 0,
    })
}
fn result(context: &TurnContext, spent: u64) -> TurnOutcome {
    TurnOutcome {
        budget: Budget {
            max_tokens: context.budget.max_tokens,
            used_tokens: context.budget.used_tokens + spent,
        },
        result: Ok(TurnReport {
            steps: 1,
            usage: Usage {
                input_tokens: spent,
                output_tokens: 0,
            },
            transaction_ids: vec![],
            tool_calls: vec![],
        }),
    }
}
fn observe(session: &Session, check: impl Fn(&SessionSnapshot) -> bool) -> SessionSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let s = session.snapshot().unwrap();
        if check(&s) {
            return s;
        }
        assert!(Instant::now() < deadline, "worker observation timed out");
        thread::yield_now();
    }
}
fn done(session: &Session) -> SessionSnapshot {
    observe(session, |s| !s.worker_running)
}

#[test]
fn competing_starts_have_exactly_one_worker_and_completion_releases_it() {
    let session = Arc::new(session());
    let barrier = Arc::new(Barrier::new(3));
    let (started, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let wait = Arc::new(std::sync::Mutex::new(wait));
    thread::scope(|scope| {
        let handles = (0..2)
            .map(|_| {
                let (s, b, tx, rx) = (
                    session.clone(),
                    barrier.clone(),
                    started.clone(),
                    wait.clone(),
                );
                scope.spawn(move || {
                    b.wait();
                    s.start(request(), move |c| {
                        tx.send(c.turn_id()).unwrap();
                        rx.lock().unwrap().recv().unwrap();
                        result(&c, 10)
                    })
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let results = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(results.contains(&Err(SessionError::Busy)));
        received.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(received.try_recv().is_err());
        release.send(()).unwrap();
    });
    let first = done(&session);
    assert_eq!(first.turns[0].status, Status::Completed);
    assert_eq!(first.budget.used_tokens, 10);
    let second = session
        .start(request(), |c| {
            assert_eq!(c.budget.used_tokens, 10);
            result(&c, 20)
        })
        .unwrap();
    assert!(second.0 > first.turns[0].id.0);
    assert_eq!(done(&session).budget.used_tokens, 30);
}
#[test]
fn cancelled_provider_stays_single_flight_and_late_usage_is_kept_without_late_output() {
    let s = session();
    let (started, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let id = s
        .start(request(), move |c| {
            started.send(c.clone()).unwrap();
            wait.recv().unwrap();
            TurnOutcome {
                budget: Budget {
                    used_tokens: 70,
                    ..c.budget
                },
                result: Err(AgentError::Provider(
                    "credential-like internal detail".into(),
                )),
            }
        })
        .unwrap();
    let c = received.recv_timeout(Duration::from_secs(5)).unwrap();
    c.stream("Accepted é🙂").unwrap();
    s.cancel(id).unwrap();
    assert!(c.cancellation().load(std::sync::atomic::Ordering::Acquire));
    assert!(c.stream("late").is_err());
    assert!(!c.approve("doc_patch", &json!({})));
    assert_eq!(
        s.start(request(), |c| result(&c, 0)),
        Err(SessionError::Busy)
    );
    let stopped = s.snapshot().unwrap();
    assert_eq!(stopped.turns[0].status, Status::Stopping);
    assert!(stopped.worker_running);
    release.send(()).unwrap();
    let final_state = done(&s);
    assert_eq!(final_state.budget.used_tokens, 70);
    assert_eq!(final_state.turns[0].status, Status::Cancelled);
    assert_eq!(final_state.turns[0].charged_tokens, 70);
    assert_eq!(final_state.turns[0].text, "Accepted é🙂");
    assert!(
        !serde_json::to_string(&final_state)
            .unwrap()
            .contains("credential-like")
    );
    s.start(request(), |c| {
        assert_eq!(c.budget.used_tokens, 70);
        result(&c, 1)
    })
    .unwrap();
    done(&s);
    assert!(c.stream("stale previous turn").is_err());
    assert_eq!(s.cancel(id), Err(SessionError::StaleTurn));
}
#[test]
fn approvals_have_explicit_ids_and_discard_source_after_decision_or_cancel() {
    let s = session();
    let (finished, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let source = "\u{1}".repeat(256 * 1024); // Escaped JSON is about1.5MiB, a real script capacity control.
    let id = s
        .start(request(), move |c| {
            let allowed = c.approve("script_write", &json!({"source":source}));
            finished.send(allowed).unwrap();
            wait.recv().unwrap();
            result(&c, 11)
        })
        .unwrap();
    let pending = observe(&s, |s| s.turns[0].pending.is_some());
    let approval = pending.turns[0].pending.as_ref().unwrap();
    assert!(pending.turns[0].approvals[0].bytes > 1024 * 1024);
    assert!(
        s.snapshot_if_changed(Some(pending.version))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        s.decide(TurnId(id.0 + 1), approval.id, true),
        Err(SessionError::StaleTurn)
    );
    assert_eq!(
        s.decide(id, ApprovalId(approval.id.0 + 1), true),
        Err(SessionError::StaleApproval)
    );
    s.decide(id, approval.id, true).unwrap();
    assert!(received.recv_timeout(Duration::from_secs(5)).unwrap());
    assert_eq!(
        s.decide(id, approval.id, true),
        Err(SessionError::StaleApproval)
    );
    let changed = s.snapshot().unwrap();
    assert!(changed.version > pending.version);
    assert!(changed.turns[0].pending.is_none());
    assert_eq!(changed.turns[0].approvals[0].decision, Decision::Allowed);
    assert!(serde_json::to_vec(&changed).unwrap().len() < 4096);
    release.send(()).unwrap();
    done(&s);
    let next = s
        .start(request(), |c| {
            assert!(!c.approve("doc_patch", &json!({"ops":[]})));
            result(&c, 3)
        })
        .unwrap();
    let pending = observe(&s, |s| s.turns.last().unwrap().pending.is_some());
    let new_id = pending.turns.last().unwrap().pending.as_ref().unwrap().id;
    assert_ne!(new_id, approval.id);
    s.cancel(next).unwrap();
    let stopped = done(&s);
    assert_eq!(stopped.turns.last().unwrap().status, Status::Cancelled);
    assert!(stopped.turns.last().unwrap().pending.is_none());
    assert_eq!(
        stopped.turns.last().unwrap().approvals[0].decision,
        Decision::Cancelled
    );
}
#[test]
fn unicode_stream_limit_interrupts_without_partial_codepoints_or_false_success() {
    let s = session();
    s.start(request(), |c| {
        let accepted = "🙂".repeat(MAX_TEXT_BYTES / 4);
        c.stream(&accepted).unwrap();
        assert_eq!(c.stream("é"), Err(SessionError::OutputLimit));
        assert!(c.stream("ignored").is_err());
        result(&c, 5)
    })
    .unwrap();
    let final_state = done(&s);
    let t = &final_state.turns[0];
    assert_eq!(t.text.len(), MAX_TEXT_BYTES);
    assert_eq!(t.text.chars().count(), MAX_TEXT_BYTES / 4);
    assert_eq!(t.status, Status::Failed);
    assert_eq!(t.failure, Some(Failure::OutputLimit));
    assert_eq!(final_state.budget.used_tokens, 5);
}
#[test]
fn failure_accounting_carries_across_turns_and_raw_errors_never_enter_snapshots() {
    let s = session();
    s.start(request(), |c| TurnOutcome {
        budget: Budget {
            used_tokens: 120,
            ..c.budget
        },
        result: Err(AgentError::Tool("private tool error body".into())),
    })
    .unwrap();
    let state = done(&s);
    assert_eq!(state.budget.used_tokens, 120);
    assert_eq!(state.turns[0].failure, Some(Failure::Tool));
    assert!(
        !serde_json::to_string(&state)
            .unwrap()
            .contains("private tool")
    );
    s.start(request(), |c| {
        assert_eq!(c.budget.used_tokens, 120);
        result(&c, 15)
    })
    .unwrap();
    let state = done(&s);
    assert_eq!(state.budget.used_tokens, 135);
    assert_eq!(state.turns[1].charged_tokens, 15);
}
#[test]
fn unmeasured_provider_charges_are_explicit_without_overwriting_cancellation() {
    for (charged, cancelled) in [(0, false), (42, false), (42, true)] {
        let s = Session::new(Budget {
            max_tokens: 1000,
            used_tokens: 5,
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let id = s
            .start(request(), move |c| {
                rx.recv().unwrap();
                TurnOutcome {
                    budget: Budget {
                        max_tokens: 1000,
                        used_tokens: c.budget.used_tokens + charged,
                    },
                    result: Err(AgentError::Response {
                        reason: "private transport failure",
                        usage: None,
                    }),
                }
            })
            .unwrap();
        if cancelled {
            s.cancel(id).unwrap();
        }
        tx.send(()).unwrap();
        let state = done(&s);
        assert_eq!(state.budget.used_tokens, 5 + charged);
        assert_eq!(state.accounting_uncertain, charged != 0);
        assert_eq!(state.turns[0].accounting_uncertain, charged != 0);
        assert_eq!(
            state.turns[0].status,
            if cancelled {
                Status::Cancelled
            } else {
                Status::Failed
            }
        );
        assert_eq!(
            state.turns[0].failure,
            if cancelled {
                None
            } else {
                Some(Failure::Provider)
            }
        );
        assert!(
            !serde_json::to_string(&state)
                .unwrap()
                .contains("private transport failure")
        );
    }
}
#[test]
fn account_fence_failure_after_measured_usage_does_not_mark_a_reservation() {
    let s = Session::new(Budget {
        max_tokens: 1000,
        used_tokens: 5,
    });
    s.start(request(), |c| TurnOutcome {
        budget: Budget {
            max_tokens: 1000,
            used_tokens: c.budget.used_tokens + 42,
        },
        result: Err(AgentError::Provider("account selection changed".into())),
    })
    .unwrap();
    let state = done(&s);
    assert_eq!(state.budget.used_tokens, 47);
    assert_eq!(state.turns[0].charged_tokens, 42);
    assert_eq!(state.turns[0].failure, Some(Failure::Provider));
    assert!(!state.accounting_uncertain);
    assert!(!state.turns[0].accounting_uncertain);
}
#[test]
fn inconsistent_budget_or_panicking_jobs_exhaust_allowance_explicitly() {
    for kind in 0..5 {
        let s = Session::new(Budget {
            max_tokens: 1000,
            used_tokens: 10,
        });
        s.start(request(), move |c| {
            if kind == 0 {
                panic!("synthetic worker panic");
            }
            let mut r = result(&c, 5);
            match kind {
                1 => r.budget.used_tokens = 0,
                2 => r.budget.max_tokens += 1,
                4 => {
                    r.budget.used_tokens = 10;
                    r.result = Err(AgentError::Response {
                        reason: "private response failure",
                        usage: Some(Usage {
                            input_tokens: 3,
                            output_tokens: 2,
                        }),
                    });
                }
                _ => r.budget.used_tokens = 10,
            }
            r
        })
        .unwrap();
        let state = done(&s);
        assert!(state.accounting_uncertain);
        assert_eq!(state.budget.remaining(), 0);
        assert_eq!(
            state.turns[0].failure,
            Some(if kind == 0 {
                Failure::WorkerPanicked
            } else {
                Failure::BudgetAccounting
            })
        );
        assert_eq!(
            s.start(request(), |c| result(&c, 0)),
            Err(SessionError::Budget)
        );
    }
}
#[test]
fn successful_over_budget_job_preserves_usage_and_fails_instead_of_completing() {
    let s = Session::new(Budget {
        max_tokens: 1000,
        used_tokens: 900,
    });
    s.start(request(), |c| result(&c, 125)).unwrap();
    let state = done(&s);
    let turn = &state.turns[0];
    assert_eq!(state.budget.used_tokens, 1025);
    assert_eq!(state.budget.remaining(), 0);
    assert!(!state.accounting_uncertain);
    assert_eq!(turn.status, Status::Failed);
    assert_eq!(turn.failure, Some(Failure::BudgetExhausted));
    assert_eq!(turn.charged_tokens, 125);
    assert!(!turn.accounting_uncertain);
    assert_eq!(turn.report.as_ref().unwrap().usage.input_tokens, 125);
    assert_eq!(
        s.start(request(), |c| result(&c, 0)),
        Err(SessionError::Budget)
    );
}
#[test]
fn oversized_and_deep_approval_payloads_refuse_without_unbounded_history() {
    for deep in [false, true] {
        let s = session();
        s.start(request(), move |c| {
            let mut value = json!({"source":"a".repeat(if deep{1}else{MAX_APPROVAL_BYTES+1})});
            if deep {
                for _ in 0..34 {
                    value = json!([value]);
                }
            }
            assert!(!c.approve("script_write", &value));
            assert!(c.cancellation().load(std::sync::atomic::Ordering::Acquire));
            result(&c, 2)
        })
        .unwrap();
        let state = done(&s);
        assert!(state.turns[0].pending.is_none());
        assert!(state.turns[0].approvals.is_empty());
        assert_eq!(
            state.turns[0].failure,
            Some(if deep {
                Failure::ApprovalInvalid
            } else {
                Failure::ApprovalPayloadLimit
            })
        );
    }
}
#[test]
fn aggregate_approval_bytes_and_count_are_bounded_after_resolved_bodies_are_dropped() {
    for large in [false, true] {
        let s = session();
        let id = s
            .start(request(), move |c| {
                let args = if large {
                    json!({"source":"a".repeat(MAX_APPROVAL_BYTES-100)})
                } else {
                    json!({})
                };
                for _ in 0..MAX_APPROVALS + 1 {
                    if !c.approve("script_write", &args) {
                        break;
                    }
                }
                result(&c, 1)
            })
            .unwrap();
        loop {
            let state = observe(&s, |s| !s.worker_running || s.turns[0].pending.is_some());
            if !state.worker_running {
                assert_eq!(
                    state.turns[0].failure,
                    Some(if large {
                        Failure::ApprovalTotalLimit
                    } else {
                        Failure::ApprovalCountLimit
                    })
                );
                break;
            }
            let p = state.turns[0].pending.as_ref().unwrap();
            s.decide(id, p.id, true).unwrap();
        }
    }
}
#[test]
fn history_report_and_request_metadata_are_bounded_and_idle_polls_are_empty() {
    let s = session();
    let initial = s.snapshot().unwrap();
    assert!(
        s.snapshot_if_changed(Some(initial.version))
            .unwrap()
            .is_none()
    );
    for bad in [
        TurnRequest {
            prompt: " ".into(),
            ..request()
        },
        TurnRequest {
            prompt: "é".repeat(MAX_PROMPT_BYTES),
            ..request()
        },
        TurnRequest {
            model: "x".repeat(129),
            ..request()
        },
        TurnRequest {
            model: "\n".into(),
            ..request()
        },
    ] {
        assert_eq!(
            s.start(bad, |c| result(&c, 0)),
            Err(SessionError::InvalidRequest)
        );
    }
    for _ in 0..MAX_HISTORY + 2 {
        s.start(request(), |c| {
            let mut r = result(&c, 1);
            let report = r.result.as_mut().unwrap();
            report.transaction_ids = vec!["t".repeat(129); 200];
            report.tool_calls = (0..200)
                .map(|_| ToolObservation {
                    name: "n".repeat(129),
                    succeeded: false,
                    selected_camera: None,
                })
                .collect();
            r
        })
        .unwrap();
        done(&s);
    }
    let state = s.snapshot().unwrap();
    assert_eq!(state.turns.len(), MAX_HISTORY);
    assert_eq!(state.budget.used_tokens, (MAX_HISTORY + 2) as u64);
    assert!(state.turns[0].id.0 > 1);
    let report = state.turns.last().unwrap().report.as_ref().unwrap();
    assert!(report.truncated);
    assert_eq!(report.transaction_count, 200);
    assert_eq!(report.tool_call_count, 200);
    assert!(report.transaction_ids.is_empty());
    assert_eq!(report.tool_calls.len(), 128);
    assert!(
        s.snapshot_if_changed(Some(state.version))
            .unwrap()
            .is_none()
    );
}
#[test]
fn session_drop_wakes_approval_waiter_and_revokes_retained_handles() {
    let s = session();
    let (finished, received) = mpsc::channel();
    s.start(request(), move |c| {
        let approved = c.approve("doc_patch", &json!({}));
        assert!(!approved);
        assert!(c.stream("late").is_err());
        finished.send(()).unwrap();
        result(&c, 1)
    })
    .unwrap();
    observe(&s, |s| s.turns[0].pending.is_some());
    drop(s);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn denial_is_explicit_and_cannot_be_replayed_over_a_later_approval() {
    let s = session();
    let id = s
        .start(request(), |c| {
            assert!(!c.approve("doc_patch", &json!({"description":"first"})));
            assert!(c.approve("doc_patch", &json!({"description":"second"})));
            result(&c, 8)
        })
        .unwrap();
    let first = observe(&s, |s| s.turns[0].pending.is_some()).turns[0]
        .pending
        .as_ref()
        .unwrap()
        .id;
    s.decide(id, first, false).unwrap();
    let second = observe(&s, |s| {
        s.turns[0].pending.as_ref().is_some_and(|p| p.id != first)
    })
    .turns[0]
        .pending
        .as_ref()
        .unwrap()
        .id;
    assert_eq!(s.decide(id, first, true), Err(SessionError::StaleApproval));
    s.decide(id, second, true).unwrap();
    let state = done(&s);
    let turn = &state.turns[0];
    assert_eq!(turn.status, Status::Completed);
    assert_eq!(
        turn.approvals
            .iter()
            .map(|a| a.decision)
            .collect::<Vec<_>>(),
        vec![Decision::Denied, Decision::Allowed]
    );
    assert_eq!(state.budget.used_tokens, 8);
}
