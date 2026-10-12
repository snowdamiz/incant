use super::*;
use crate::{TurnReport, Usage};
use std::time::{Duration, Instant};

#[test]
fn post_publication_worker_panic_recomputes_conservative_charged_delta() {
    let session = Session::new(Budget {
        max_tokens: 1000,
        used_tokens: 10,
    });
    let request = TurnRequest {
        prompt: "Fixture".into(),
        model: "fixture-model".into(),
        approval: ApprovalChoice::Auto,
    };
    session
        .start(request.clone(), |context| TurnOutcome {
            budget: Budget {
                used_tokens: context.budget.used_tokens + 5,
                ..context.budget
            },
            result: Ok(TurnReport {
                steps: 1,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 0,
                },
                transaction_ids: vec![],
                tool_calls: vec![],
            }),
        })
        .unwrap();
    {
        let mut worker = session.worker.lock().unwrap();
        let published_worker = worker.take().unwrap();
        // Exercise a join failure after finish published successfully without
        // causing a thread-local destructor panic, which can abort the process.
        *worker = Some(thread::spawn(move || {
            published_worker.join().unwrap();
            panic!("synthetic post-publication failure");
        }));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let snapshot = loop {
        let snapshot = session.snapshot().unwrap();
        if !snapshot.worker_running {
            break snapshot;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    };
    assert!(snapshot.accounting_uncertain);
    assert_eq!(snapshot.budget.used_tokens, 1000);
    let turn = &snapshot.turns[0];
    assert_eq!(turn.status, Status::Failed);
    assert_eq!(turn.failure, Some(Failure::WorkerPanicked));
    assert!(turn.accounting_uncertain);
    assert_eq!(turn.charged_tokens, 990);
    assert_eq!(turn.report.as_ref().unwrap().usage.input_tokens, 5);
    assert_eq!(
        session.start(request, |_| unreachable!()),
        Err(SessionError::Budget)
    );
}
