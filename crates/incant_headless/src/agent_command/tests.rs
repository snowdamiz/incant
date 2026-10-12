use super::*;
use clap::Parser;
use incant_agent::{NoViewport, Usage, provider::Completion};
use incant_cmd::{Actor, Command};
use incant_doc::{AgentBudgetSettings, Entity, Scene};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    state: PathBuf,
    project: Project,
}
impl Fixture {
    fn new(cap: u64) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("project.json");
        let state = temp.path().join("usage");
        let mut project = Project::empty("Synthetic CLI project");
        project.settings.agent_budget.max_tokens = cap;
        let mut scene = Scene::new("Main");
        let entity = Entity::new("Before");
        scene.entities.insert(entity.id.clone(), entity);
        project.scenes.insert(scene.id.clone(), scene);
        fs::write(&source, project.canonical_text().unwrap()).unwrap();
        Self {
            _temp: temp,
            source,
            state,
            project,
        }
    }
    fn options(&self, limit: u64) -> Options {
        Options {
            project: self.source.clone(),
            prompt: "Inspect the fixture".into(),
            model: "synthetic".into(),
            max_tokens: limit,
            max_output_tokens: 25000,
            journal: None,
        }
    }
    fn open(&self, id: &str) -> std::result::Result<Ledger, LedgerError> {
        Ledger::open_in(&self.state, id)
    }
    fn snapshot(&self) -> incant_agent::ledger::LedgerSnapshot {
        self.open(&self.project.id)
            .unwrap()
            .snapshot(self.project.settings.agent_budget.max_tokens)
            .unwrap()
    }
    fn seed(&self, used: u64) {
        self.open(&self.project.id)
            .unwrap()
            .reserve(1_000_000)
            .unwrap()
            .complete(
                &Budget {
                    max_tokens: 1_000_000,
                    used_tokens: used,
                },
                false,
            )
            .unwrap();
    }
    fn run(
        &self,
        limit: u64,
        behavior: Behavior,
        tokens: u64,
        calls: Arc<AtomicUsize>,
    ) -> Result<Receipt> {
        execute(
            self.options(limit),
            |id| self.open(id),
            worker(behavior, tokens, calls, None),
            save,
        )
    }
    fn bus(&self) -> CommandBus {
        CommandBus::persistent_with_sources(
            self.source.with_extension("journal.jsonl"),
            self.project.clone(),
            self.source.parent().unwrap(),
        )
        .unwrap()
    }
}
use std::fs;

enum Behavior {
    Success,
    MeasuredError,
    UnknownError,
    Panic,
    Cancel,
    ChangedAccount,
    Patch(Value),
}
struct Synthetic {
    behavior: Behavior,
    tokens: u64,
    calls: Arc<AtomicUsize>,
    context: TurnContext,
    inspect: Option<Box<dyn FnOnce() + Send>>,
}
impl Provider for Synthetic {
    fn check_active(&self) -> std::result::Result<(), AgentError> {
        if matches!(self.behavior, Behavior::ChangedAccount)
            && self.calls.load(Ordering::SeqCst) > 0
        {
            return Err(AgentError::Provider("Synthetic account changed".into()));
        }
        Ok(())
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        stream: &mut dyn FnMut(&str),
    ) -> std::result::Result<Completion, AgentError> {
        let count = self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(inspect) = self.inspect.take() {
            inspect();
        }
        let usage = Usage {
            input_tokens: self.tokens.saturating_sub(3),
            output_tokens: self.tokens.min(3),
        };
        match &self.behavior {
            Behavior::MeasuredError => {
                return Err(AgentError::Response {
                    reason: "Synthetic failed response",
                    usage: Some(usage),
                });
            }
            Behavior::UnknownError => {
                return Err(AgentError::Provider("Synthetic failed request".into()));
            }
            Behavior::Panic => panic!("synthetic provider panic"),
            Behavior::Cancel => self.context.cancellation().store(true, Ordering::Release),
            _ => (),
        }
        stream("Synthetic response");
        let output = match &self.behavior {
            Behavior::Patch(args) if count == 0 => vec![
                json!({"type":"function_call", "name":"doc_patch", "call_id":"fixture", "arguments":args.to_string()}),
            ],
            _ => vec![],
        };
        Ok(Completion { output, usage })
    }
    fn model(&self) -> &str {
        "synthetic"
    }
}
fn worker(
    behavior: Behavior,
    tokens: u64,
    calls: Arc<AtomicUsize>,
    inspect: Option<Box<dyn FnOnce() + Send>>,
) -> impl FnOnce(TurnContext, CommandBus, PathBuf) -> Work + Send + 'static {
    move |context, bus, root| {
        agent_work(
            context,
            bus,
            root,
            move |_, _, context| {
                Ok((
                    Synthetic {
                        behavior,
                        tokens,
                        calls,
                        context: context.clone(),
                        inspect,
                    },
                    NoViewport,
                ))
            },
            25000,
            |_, _| true,
            |_| {},
        )
    }
}
fn calls() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}

#[test]
fn cli_flags_remain_per_invocation_and_saturation_cannot_raise_project_cap() {
    let args = crate::Args::try_parse_from([
        "incant",
        "agent",
        "project.json",
        "Inspect",
        "--model",
        "synthetic",
    ])
    .unwrap();
    let crate::Cli::Agent(options) = args.command else {
        panic!("wrong CLI command")
    };
    assert_eq!(options.max_tokens, 64000);
    assert_eq!(options.max_output_tokens, 25000);
    assert_eq!(invocation_ceiling(1_000_000, 150_000, 64000), 214_000);
    assert_eq!(invocation_ceiling(200_000, 150_000, 100_000), 200_000);
    assert_eq!(
        invocation_ceiling(MAX_TOKEN_CAP, MAX_TOKEN_CAP - 1, u64::MAX),
        MAX_TOKEN_CAP
    );
}

#[test]
fn fresh_invocations_accumulate_usage_and_cannot_refund_it_with_a_cli_flag() {
    let fixture = Fixture::new(1_000_000);
    fixture.seed(150_000); // Cumulative usage can already exceed the per-command limit.
    let count = calls();
    let first = fixture
        .run(100_000, Behavior::Success, 11, count.clone())
        .unwrap();
    assert_eq!(first.budget.max_tokens, 250_000);
    assert_eq!(first.budget.used_tokens, 150_011);
    assert_eq!(
        first.report["usage"],
        json!({"input_tokens":8,"output_tokens":3})
    );
    let second = fixture
        .run(100_000, Behavior::Success, 13, count.clone())
        .unwrap();
    assert_eq!(second.budget.max_tokens, 250_011);
    assert_eq!(second.budget.used_tokens, 150_024);
    assert_eq!(second.project_max_tokens, 1_000_000);
    assert!(!second.accounting_uncertain);
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.snapshot().budget.used_tokens, 150_024);
    assert_eq!(
        fs::read_to_string(&fixture.source).unwrap(),
        fixture.project.canonical_text().unwrap()
    );
}

#[test]
fn zero_and_exhausted_allowances_refuse_before_constructing_a_provider() {
    for (cap, limit, used) in [
        (0, 100_000, 0),
        (1_000_000, 0, 0),
        (100_000, 1_000_000, 100_000),
        (100_000, 1_000_000, 100_001),
    ] {
        let fixture = Fixture::new(cap);
        if used > 0 {
            fixture.seed(used);
        }
        let count = calls();
        let constructed = count.clone();
        let error = execute(
            fixture.options(limit),
            |id| fixture.open(id),
            move |_, _, _| {
                constructed.fetch_add(1, Ordering::SeqCst);
                panic!("ineligible invocation reached provider setup")
            },
            save,
        )
        .err()
        .unwrap();
        assert!(matches!(
            error.downcast_ref::<CommandError>(),
            Some(CommandError::Allowance)
        ));
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.snapshot().budget.used_tokens, used);
    }
}

#[test]
fn authored_cap_after_journal_recovery_and_user_undo_never_restores_consumption() {
    let fixture = Fixture::new(1_000_000);
    fixture
        .run(100_000, Behavior::Success, 11, calls())
        .unwrap();
    let mut bus = fixture.bus();
    bus.execute(
        vec![Command::SetAgentBudget {
            settings: AgentBudgetSettings { max_tokens: 0 },
        }],
        Actor::user("fixture"),
        "Disable agent",
        Some(0),
    )
    .unwrap();
    drop(bus);
    let count = calls();
    assert!(
        fixture
            .run(100_000, Behavior::Success, 1, count.clone())
            .is_err()
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let mut bus = fixture.bus();
    bus.undo().unwrap();
    drop(bus);
    let receipt = fixture.run(100_000, Behavior::Success, 13, count).unwrap();
    assert_eq!(receipt.budget.used_tokens, 24);
    assert_eq!(receipt.project_max_tokens, 1_000_000);
    assert_eq!(fixture.snapshot().budget.used_tokens, 24);
}

#[test]
fn journal_cap_is_read_after_accounting_ownership_before_provider_setup() {
    let fixture = Fixture::new(1_000_000);
    let count = calls();
    let error = execute(
        fixture.options(100_000),
        |id| {
            // A previous owner/manual edit became authoritative at the acquisition boundary.
            let mut bus = fixture.bus();
            bus.execute(
                vec![Command::SetAgentBudget {
                    settings: AgentBudgetSettings { max_tokens: 0 },
                }],
                Actor::user("fixture"),
                "Disable at admission",
                Some(0),
            )
            .unwrap();
            drop(bus);
            fixture.open(id)
        },
        worker(Behavior::Success, 11, count.clone(), None),
        save,
    )
    .err()
    .unwrap();
    assert!(matches!(
        error.downcast_ref::<CommandError>(),
        Some(CommandError::Allowance)
    ));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.snapshot().budget.used_tokens, 0);
}

#[test]
fn actual_over_cap_usage_remains_charged_and_next_invocation_is_refused() {
    let fixture = Fixture::new(100_000);
    let count = calls();
    let error = fixture
        .run(1_000_000, Behavior::Success, 100_001, count.clone())
        .err()
        .unwrap();
    assert!(matches!(
        error.downcast_ref::<CommandError>(),
        Some(CommandError::Turn(Failure::BudgetExhausted))
    ));
    assert_eq!(fixture.snapshot().budget.used_tokens, 100_001);
    assert!(!fixture.snapshot().accounting_uncertain);
    assert!(
        fixture
            .run(1_000_000, Behavior::Success, 1, count.clone())
            .is_err()
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn measured_error_cancel_and_account_boundary_preserve_actual_usage() {
    for behavior in [
        Behavior::MeasuredError,
        Behavior::Cancel,
        Behavior::ChangedAccount,
    ] {
        let fixture = Fixture::new(1_000_000);
        fixture.seed(19);
        let count = calls();
        assert!(fixture.run(100_000, behavior, 11, count.clone()).is_err());
        let snapshot = fixture.snapshot();
        assert_eq!(snapshot.budget.used_tokens, 30);
        assert!(!snapshot.accounting_uncertain);
        assert!(!snapshot.recovered_reservation);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        fixture.run(100_000, Behavior::Success, 13, count).unwrap();
        assert_eq!(fixture.snapshot().budget.used_tokens, 43);
    }
}

#[test]
fn unmeasured_provider_failure_is_conservative_across_reopen() {
    let fixture = Fixture::new(1_000_000);
    fixture.seed(19);
    assert!(
        fixture
            .run(100_000, Behavior::UnknownError, 0, calls())
            .is_err()
    );
    let failed = fixture.snapshot();
    assert!(failed.budget.used_tokens > 19);
    assert!(failed.budget.used_tokens <= 100_019);
    assert!(failed.accounting_uncertain);
    assert!(!failed.recovered_reservation);
    let receipt = fixture
        .run(100_000, Behavior::Success, 11, calls())
        .unwrap();
    assert_eq!(receipt.budget.used_tokens, failed.budget.used_tokens + 11);
    assert!(receipt.accounting_uncertain);
}

#[test]
fn provider_setup_failure_has_zero_new_work_and_keeps_previous_charges() {
    let fixture = Fixture::new(1_000_000);
    fixture.seed(19);
    let error = execute(
        fixture.options(100_000),
        |id| fixture.open(id),
        |context, bus, root| {
            agent_work(
                context,
                bus,
                root,
                |_, _, _| {
                    Err::<(Synthetic, NoViewport), _>(AgentError::Provider(
                        "Synthetic account unavailable".into(),
                    ))
                },
                25000,
                |_, _| true,
                |_| {},
            )
        },
        save,
    )
    .err()
    .unwrap();
    assert!(matches!(
        error.downcast_ref::<CommandError>(),
        Some(CommandError::Turn(Failure::Provider))
    ));
    assert_eq!(fixture.snapshot().budget.used_tokens, 19);
    assert!(!fixture.snapshot().recovered_reservation);
}

#[test]
fn panic_or_inconsistent_receipt_retains_the_entire_effective_reservation() {
    for malformed in [false, true] {
        let fixture = Fixture::new(1_000_000);
        fixture.seed(19);
        let work = worker(
            if malformed {
                Behavior::Success
            } else {
                Behavior::Panic
            },
            11,
            calls(),
            None,
        );
        let error = execute(
            fixture.options(100_000),
            |id| fixture.open(id),
            move |context, bus, root| {
                let mut work = work(context, bus, root);
                work.budget.used_tokens = 0; // An actual work result cannot refund its measured usage.
                work
            },
            save,
        )
        .err()
        .unwrap();
        let expected = if malformed {
            Failure::BudgetAccounting
        } else {
            Failure::WorkerPanicked
        };
        assert!(
            matches!(error.downcast_ref::<CommandError>(), Some(CommandError::Turn(actual)) if *actual == expected)
        );
        let snapshot = fixture.snapshot();
        assert_eq!(snapshot.budget.used_tokens, 100_019);
        assert!(snapshot.accounting_uncertain);
        assert!(snapshot.recovered_reservation);
    }
}

#[test]
fn output_save_failure_after_provider_work_cannot_reset_accounting() {
    let fixture = Fixture::new(1_000_000);
    let output = fixture.source.with_extension("agent-result.json");
    fs::create_dir(&output).unwrap(); // Real atomic save cannot replace a directory.
    let error = fixture
        .run(100_000, Behavior::Success, 11, calls())
        .err()
        .unwrap();
    assert!(matches!(
        error.downcast_ref::<CommandError>(),
        Some(CommandError::OutputSave)
    ));
    assert_eq!(fixture.snapshot().budget.used_tokens, 11);
    assert!(!fixture.snapshot().accounting_uncertain);
    assert!(!fixture.snapshot().recovered_reservation);
    fs::remove_dir(output).unwrap();
    fixture
        .run(100_000, Behavior::Success, 13, calls())
        .unwrap();
    assert_eq!(fixture.snapshot().budget.used_tokens, 24);
}

#[test]
fn existing_ledger_owner_refuses_admission_and_live_provider_keeps_exclusive_ownership() {
    let fixture = Fixture::new(1_000_000);
    let owner = fixture.open(&fixture.project.id).unwrap();
    let count = calls();
    let error = fixture
        .run(100_000, Behavior::Success, 11, count.clone())
        .err()
        .unwrap();
    assert!(matches!(
        error.downcast_ref::<LedgerError>(),
        Some(LedgerError::Busy)
    ));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    drop(owner);
    let (started, waiting) = mpsc::sync_channel(1);
    let (finish, ending) = mpsc::sync_channel(1);
    let state = fixture.state.clone();
    let id = fixture.project.id.clone();
    let inspect = Box::new(move || {
        assert!(matches!(
            Ledger::open_in(&state, &id),
            Err(LedgerError::Busy)
        ));
        let record: Value =
            serde_json::from_slice(&fs::read(state.join(format!("{id}.json"))).unwrap()).unwrap();
        assert_eq!(record["charged_tokens"], 100_000);
        assert_eq!(record["pending"]["initial_tokens"], 0);
        started.send(()).unwrap();
        ending.recv_timeout(Duration::from_secs(10)).unwrap();
    });
    let options = fixture.options(100_000);
    let state = fixture.state.clone();
    let thread = thread::spawn(move || {
        execute(
            options,
            |id| Ledger::open_in(&state, id),
            worker(Behavior::Success, 11, count, Some(inspect)),
            save,
        )
        .unwrap()
    });
    waiting.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(matches!(
        fixture.open(&fixture.project.id),
        Err(LedgerError::Busy)
    ));
    finish.send(()).unwrap();
    let receipt = thread.join().unwrap();
    assert_eq!(receipt.budget.used_tokens, 11);
    assert_eq!(fixture.snapshot().budget.used_tokens, 11);
}

#[test]
fn successful_tool_edit_keeps_shared_journal_provenance_and_compatible_report_output() {
    let fixture = Fixture::new(1_000_000);
    let scene = fixture.project.scenes.values().next().unwrap();
    let entity = scene.entities.keys().next().unwrap();
    let patch = json!({"ops":[{"op":"rename_entity","scene_id":scene.id,"entity_id":entity,"name":"After"}],
        "description":"Fixture rename", "expected_revision":0});
    let receipt = fixture
        .run(100_000, Behavior::Patch(patch), 11, calls())
        .unwrap();
    assert_eq!(receipt.report["steps"], 2);
    assert_eq!(
        receipt.report["usage"],
        json!({"input_tokens":16,"output_tokens":6})
    );
    assert_eq!(receipt.report["tool_calls"][0]["name"], "doc_patch");
    assert_eq!(receipt.budget.used_tokens, 22);
    let bus = fixture.bus();
    assert_eq!(bus.history().len(), 1);
    assert_eq!(bus.history()[0].actor.origin, incant_doc::Origin::Agent);
    assert_eq!(
        bus.project().scenes[&scene.id].entities[entity].name,
        "After"
    );
    let derived = read_project(&receipt.output).unwrap();
    assert_eq!(derived, *bus.project());
    assert_eq!(read_project(&fixture.source).unwrap(), fixture.project);
    let output = serde_json::to_value(receipt).unwrap();
    assert!(output.get("report").is_some());
    assert!(output.get("output").is_some());
    assert!(output.get("journal").is_some());
    assert_eq!(output["budget"]["max_tokens"], 100_000);
    assert_eq!(output["project_max_tokens"], 1_000_000);
}
