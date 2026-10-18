use super::support::{Fixture, Mock, TOOL, agent};
use incant_agent::{AgentError, ApprovalMode, NoViewport};
use incant_cmd::{Actor, Command};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn mock_provider_observes_normal_write_approval_and_engine_actor() {
    for mode in [
        ApprovalMode::Auto,
        ApprovalMode::Destructive,
        ApprovalMode::Always,
    ] {
        let f = Fixture::new();
        let mut host = f.host();
        let mut provider = Mock::new(f.args());
        let bus = Mutex::new(f.bus);
        let mut approvals = 0;
        let report = agent(mode)
            .run_shared(
                &mut provider,
                &bus,
                &mut host,
                "Paint tiles",
                "approved-turn",
                &AtomicBool::new(false),
                &mut |name, _| {
                    assert_eq!(name, TOOL);
                    assert!(
                        bus.try_lock().is_ok(),
                        "approval must not own the editor lock"
                    );
                    approvals += 1;
                    true
                },
                &mut |_| {},
            )
            .unwrap();
        assert_eq!(
            approvals,
            if matches!(mode, ApprovalMode::Auto) {
                0
            } else {
                1
            }
        );
        assert!(provider.advertised.iter().all(|v| *v));
        assert!(report.tool_calls[0].succeeded);
        assert_eq!(report.transaction_ids.len(), 1);
        assert!(provider.result.as_ref().unwrap()["untrusted_project_data"].is_object());
        let bus = bus.lock().unwrap();
        let tx = bus.history().last().unwrap();
        assert_eq!(
            tx.actor,
            Actor::agent("mock-tilemap-model", "approved-turn")
        );
        assert_eq!(report.transaction_ids[0], tx.id);
    }
}

#[test]
fn denied_or_cancelled_approval_preserves_document_and_journal() {
    for cancel_at_approval in [false, true] {
        let mut f = Fixture::new();
        let mut host = f.host();
        let before = f.bus.project().clone();
        let history = f.bus.history().to_vec();
        let journal = f.journal_bytes();
        let mut provider = Mock::new(f.args());
        let cancel = AtomicBool::new(false);
        let mut approvals = 0;
        let result = agent(ApprovalMode::Destructive).run(
            &mut provider,
            &mut f.bus,
            &mut host,
            "Paint",
            "denial",
            &cancel,
            &mut |name, _| {
                assert_eq!(name, TOOL);
                approvals += 1;
                if cancel_at_approval {
                    cancel.store(true, Ordering::Relaxed);
                }
                cancel_at_approval
            },
            &mut |_| {},
        );
        assert_eq!(approvals, 1);
        if cancel_at_approval {
            assert!(matches!(result, Err(AgentError::Interrupted)));
        } else {
            let report = result.unwrap();
            assert!(!report.tool_calls[0].succeeded);
            assert!(report.transaction_ids.is_empty());
            assert!(
                provider.result.unwrap()["error"]
                    .as_str()
                    .unwrap()
                    .contains("denied")
            );
        }
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
        assert_eq!(f.journal_bytes(), journal);
    }
}

#[test]
fn approval_edit_invalidates_revision_instead_of_overwriting_the_user() {
    let f = Fixture::new();
    let mut host = f.host();
    let mut provider = Mock::new(f.args());
    let bus = Mutex::new(f.bus);
    let before = bus.lock().unwrap().project().clone();
    let report = agent(ApprovalMode::Destructive)
        .run_shared(
            &mut provider,
            &bus,
            &mut host,
            "Paint",
            "stale",
            &AtomicBool::new(false),
            &mut |_, _| {
                bus.try_lock()
                    .unwrap()
                    .execute(
                        vec![Command::SetMemory {
                            section: "user".into(),
                            text: "Keep my edit".into(),
                        }],
                        Actor::user("editor"),
                        "User edit",
                        None,
                    )
                    .unwrap();
                true
            },
            &mut |_| {},
        )
        .unwrap();
    assert!(!report.tool_calls[0].succeeded);
    assert!(report.transaction_ids.is_empty());
    let bus = bus.lock().unwrap();
    assert_eq!(bus.project().scenes, before.scenes);
    assert_eq!(bus.project().memory["user"], "Keep my edit");
    assert!(
        provider.result.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("revision changed")
    );
}

#[test]
fn unscoped_host_hides_tool_and_rejects_direct_provider_request() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    let mut provider = Mock::new(f.args());
    let report = agent(ApprovalMode::Auto)
        .run(
            &mut provider,
            &mut f.bus,
            &mut NoViewport,
            "Try painting",
            "unscoped",
            &AtomicBool::new(false),
            &mut |_, _| panic!("Auto does not ask"),
            &mut |_| {},
        )
        .unwrap();
    assert!(provider.advertised.iter().all(|v| !v));
    assert!(!report.tool_calls[0].succeeded);
    assert!(report.transaction_ids.is_empty());
    assert_eq!(f.bus.project(), &before);
}
