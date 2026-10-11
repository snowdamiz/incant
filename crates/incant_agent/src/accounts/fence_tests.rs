//! Private metadata fixtures never open credentials or contact a provider.
use super::*;
use crate::{
    Agent, ApprovalMode, Budget, NoViewport, Usage,
    provider::{Completion, Provider},
};
use incant_cmd::CommandBus;
use incant_doc::Project;
use serde_json::{Value, json};
use std::{
    cell::Cell,
    sync::{Mutex, atomic::AtomicBool, mpsc},
};

fn account(id: &str) -> AccountMetadata {
    AccountMetadata {
        id: id.into(),
        client_id: format!("fixture-client-{id}"),
        subject: format!("fixture-subject-{id}"),
        label: "Fixture".into(),
    }
}
fn fixture() -> (tempfile::TempDir, AccountSelection) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    store.activate(account("a")).unwrap();
    let selection = store.selection().unwrap();
    (dir, selection)
}
#[test]
fn legacy_metadata_receipts_survive_reads_but_reject_aba_and_reconnect() {
    let dir = tempfile::tempdir().unwrap();
    let original = serde_json::to_vec(&json!({
        "host_id":"urn:uuid:11111111-1111-4111-8111-111111111111",
        "active":"a", "accounts":[account("a")], "api_key_connected":false
    }))
    .unwrap();
    let path = dir.path().join("accounts.json");
    fs::write(&path, &original).unwrap();
    let first = AccountStore::open_at(dir.path())
        .unwrap()
        .selection()
        .unwrap();
    assert_eq!(first.generation, 0);
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    assert!(store.selection().as_ref() == Some(&first));
    assert_eq!(fs::read(&path).unwrap(), original);
    store.activate(account("b")).unwrap();
    store.activate(account("a")).unwrap();
    assert!(store.check_selection(&first).is_err());
    let second = store.selection().unwrap();
    store.activate(account("a")).unwrap();
    assert!(store.check_selection(&second).is_err());
    let third = store.selection().unwrap();
    drop(store);
    let store = AccountStore::open_at(dir.path()).unwrap();
    assert!(store.check_selection(&third).is_ok());
    assert!(
        !String::from_utf8(fs::read(path).unwrap())
            .unwrap()
            .contains("token")
    );
}
#[test]
fn receipt_binds_registration_identity_and_rejects_invalid_selection() {
    let (dir, selection) = fixture();
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    store.data.accounts[0].subject = "different".into();
    assert!(store.check_selection(&selection).is_err());
    store.data.active = Some("missing".into());
    store.data.api_key_connected = true;
    assert!(store.selection().is_none());
    // Refuse before any real API-key fallback/credential lookup.
    assert!(store.provider("fixture-model".into()).is_err());
}
#[test]
fn failed_credential_deletion_preserves_selection_but_durably_invalidates_turns() {
    let (dir, selection) = fixture();
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    assert!(
        store
            .disconnect_with(
                |_| Err(error("synthetic credential failure")),
                || panic!("not the selected registration"),
            )
            .is_err()
    );
    drop(store);
    let store = AccountStore::open_at(dir.path()).unwrap();
    assert_eq!(store.selected().unwrap().id, "a");
    assert!(store.check_selection(&selection).is_err());
}
#[test]
fn invalidation_failure_prevents_credential_mutation() {
    let (dir, selection) = fixture();
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    let original = fs::read(&store.path).unwrap();
    store.path = dir.path().join("missing-directory/accounts.json");
    assert!(
        store
            .disconnect_with(
                |_| panic!("credential deletion must not begin"),
                || panic!("credential deletion must not begin"),
            )
            .is_err()
    );
    drop(store);
    let store = AccountStore::open_at(dir.path()).unwrap();
    assert_eq!(fs::read(&store.path).unwrap(), original);
    assert!(store.check_selection(&selection).is_ok());
}
#[test]
fn api_key_replacement_invalidates_receipt_and_generation_never_wraps() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AccountStore::open_at(dir.path()).unwrap();
    store.data.api_key_connected = true;
    store.save().unwrap();
    let selection = store.selection().unwrap();
    assert!(matches!(selection.kind, SelectionKind::ApiKey));
    store.invalidate_selection().unwrap();
    assert!(store.check_selection(&selection).is_err());
    let before = fs::read(&store.path).unwrap();
    store.data.selection_generation = u64::MAX;
    assert!(store.invalidate_selection().is_err());
    assert_eq!(fs::read(&store.path).unwrap(), before);
}

fn agent(approval: ApprovalMode) -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 1_000_000,
            used_tokens: 7,
        },
        approval,
        max_steps: 3,
        max_output_tokens: 1000,
    }
}
fn patches() -> Completion {
    Completion {
        output: (0..2).map(|n| json!({
            "type":"function_call", "name":"doc_patch", "call_id":format!("call-{n}"),
            "arguments":json!({"expected_revision":n,"description":"Fixture edit",
                "ops":[{"op":"set_memory","section":format!("part-{n}"),"text":"accepted"}]}).to_string()
        })).collect(),
        usage: Usage { input_tokens: 11, output_tokens: 13 },
    }
}
struct Fenced<'a> {
    dir: &'a Path,
    selection: AccountSelection,
    bus: &'a Mutex<CommandBus>,
    before_response: Box<dyn FnMut() + 'a>,
    switch_after_first_tool: bool,
    calls: Cell<usize>,
}
impl Provider for Fenced<'_> {
    fn model(&self) -> &str {
        "fixture-model"
    }
    fn check_active(&self) -> Result<(), AgentError> {
        let revision = self
            .bus
            .try_lock()
            .expect("metadata fence must not hold bus")
            .revision();
        let mut store = AccountStore::open_at(self.dir)?;
        if self.switch_after_first_tool && revision == 1 {
            store.activate(account("a"))?;
        }
        store.check_selection(&self.selection)
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        assert!(self.bus.try_lock().is_ok());
        self.calls.set(self.calls.get() + 1);
        (self.before_response)();
        Ok(patches())
    }
}
#[test]
fn stale_receipt_refuses_before_provider_request_without_charging_tokens() {
    let (dir, selection) = fixture();
    AccountStore::open_at(dir.path())
        .unwrap()
        .invalidate_selection()
        .unwrap();
    let bus = Mutex::new(CommandBus::new(Project::empty("Fixture")).unwrap());
    let mut provider = Fenced {
        dir: dir.path(),
        selection,
        bus: &bus,
        before_response: Box::new(|| panic!("a stale selection cannot send project data")),
        switch_after_first_tool: false,
        calls: Cell::new(0),
    };
    let mut runner = agent(ApprovalMode::Auto);
    let result = runner.run_shared(
        &mut provider,
        &bus,
        &mut NoViewport,
        "edit",
        "fenced",
        &AtomicBool::new(false),
        &mut |_, _| true,
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::Provider(_))));
    assert_eq!(runner.budget.used_tokens, 7);
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(bus.lock().unwrap().revision(), 0);
}
#[test]
fn account_aba_during_blocked_provider_refuses_entire_batch_after_charging_usage() {
    let (dir, selection) = fixture();
    let bus = Mutex::new(CommandBus::new(Project::empty("Fixture")).unwrap());
    let (started, waiting) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let mut provider = Fenced {
                dir: dir.path(),
                selection,
                bus: &bus,
                before_response: Box::new(move || {
                    started.send(()).unwrap();
                    resume.recv_timeout(Duration::from_secs(5)).unwrap();
                }),
                switch_after_first_tool: false,
                calls: Cell::new(0),
            };
            let mut runner = agent(ApprovalMode::Auto);
            let result = runner.run_shared(
                &mut provider,
                &bus,
                &mut NoViewport,
                "edit",
                "fenced",
                &AtomicBool::new(false),
                &mut |_, _| panic!("Auto has no approval"),
                &mut |_| {},
            );
            assert!(matches!(result, Err(AgentError::Provider(_))));
            assert_eq!(runner.budget.used_tokens, 31);
            assert_eq!(provider.calls.get(), 1);
        });
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut store = AccountStore::open_at(dir.path()).unwrap();
        store.activate(account("b")).unwrap();
        store.activate(account("a")).unwrap();
        drop(store);
        release.send(()).unwrap();
        worker.join().unwrap();
    });
    assert_eq!(bus.lock().unwrap().revision(), 0);
}
#[test]
fn account_change_while_approving_refuses_current_and_remaining_tools() {
    let (dir, selection) = fixture();
    let bus = Mutex::new(CommandBus::new(Project::empty("Fixture")).unwrap());
    let mut provider = Fenced {
        dir: dir.path(),
        selection,
        bus: &bus,
        before_response: Box::new(|| {}),
        switch_after_first_tool: false,
        calls: Cell::new(0),
    };
    let mut runner = agent(ApprovalMode::Always);
    let mut approvals = 0;
    let result = runner.run_shared(
        &mut provider,
        &bus,
        &mut NoViewport,
        "edit",
        "fenced",
        &AtomicBool::new(false),
        &mut |_, _| {
            approvals += 1;
            assert!(bus.try_lock().is_ok());
            AccountStore::open_at(dir.path())
                .unwrap()
                .invalidate_selection()
                .unwrap();
            true
        },
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::Provider(_))));
    assert_eq!(approvals, 1);
    assert_eq!(runner.budget.used_tokens, 31);
    assert_eq!(provider.calls.get(), 1);
    assert_eq!(bus.lock().unwrap().revision(), 0);
}
#[test]
fn auto_batch_rechecks_each_tool_and_preserves_earlier_committed_history() {
    let (dir, selection) = fixture();
    let bus = Mutex::new(CommandBus::new(Project::empty("Fixture")).unwrap());
    let mut provider = Fenced {
        dir: dir.path(),
        selection,
        bus: &bus,
        before_response: Box::new(|| {}),
        switch_after_first_tool: true,
        calls: Cell::new(0),
    };
    let mut runner = agent(ApprovalMode::Auto);
    let result = runner.run_shared(
        &mut provider,
        &bus,
        &mut NoViewport,
        "edit",
        "fenced",
        &AtomicBool::new(false),
        &mut |_, _| panic!("Auto has no approval"),
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::Provider(_))));
    assert_eq!(runner.budget.used_tokens, 31);
    assert_eq!(provider.calls.get(), 1);
    let mut bus = bus.lock().unwrap();
    assert_eq!(bus.revision(), 1);
    assert_eq!(bus.project().memory.len(), 1);
    assert_eq!(
        bus.history()[0].actor.model.as_deref(),
        Some("fixture-model")
    );
    bus.undo().unwrap();
    assert!(bus.project().memory.is_empty());
}

#[test]
fn failed_provider_request_distinguishes_reserved_usage_from_measured_fence_errors() {
    use std::sync::atomic::Ordering;
    struct Failed<'a> {
        kind: u8,
        cancel: &'a AtomicBool,
    }
    impl Provider for Failed<'_> {
        fn model(&self) -> &str {
            "fixture-model"
        }
        fn complete(
            &mut self,
            _: &[Value],
            _: &[Value],
            _: u64,
            _: &mut dyn FnMut(&str),
        ) -> Result<Completion, AgentError> {
            Err(match self.kind {
                0 => AgentError::Provider("private transport detail".into()),
                1 => {
                    self.cancel.store(true, Ordering::Release);
                    AgentError::Interrupted
                }
                _ => AgentError::Response {
                    reason: "fixture incomplete response",
                    usage: Some(Usage {
                        input_tokens: 5,
                        output_tokens: 7,
                    }),
                },
            })
        }
    }
    for kind in 0..3 {
        let cancel = AtomicBool::new(false);
        let mut provider = Failed {
            kind,
            cancel: &cancel,
        };
        let bus = Mutex::new(CommandBus::new(Project::empty("Fixture")).unwrap());
        let mut runner = agent(ApprovalMode::Auto);
        // Unknown usage consumes the entire allowance offered to this request.
        runner.max_output_tokens = runner.budget.max_tokens;
        let result = runner.run_shared(
            &mut provider,
            &bus,
            &mut NoViewport,
            "edit",
            "failed-request",
            &cancel,
            &mut |_, _| true,
            &mut |_| {},
        );
        let Err(AgentError::Response { reason, usage }) = result else {
            panic!("failed provider request must retain usage classification");
        };
        if kind < 2 {
            assert_eq!(reason, "Provider request failed");
            assert!(usage.is_none());
            assert_eq!(runner.budget.remaining(), 0);
        } else {
            assert_eq!(reason, "fixture incomplete response");
            let usage = usage.unwrap();
            assert_eq!(usage.input_tokens, 5);
            assert_eq!(usage.output_tokens, 7);
            assert_eq!(runner.budget.used_tokens, 19);
        }
        assert_eq!(cancel.load(Ordering::Acquire), kind == 1);
        assert_eq!(bus.lock().unwrap().revision(), 0);
    }
}
