//! The provider is a deterministic protocol double, never a live-provider gate.
use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, EngineHost, NoViewport, Usage,
    play::RunRequest,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Origin, Project};
use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

fn agent() -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 1_000_000,
            used_tokens: 0,
        },
        approval: ApprovalMode::Destructive,
        max_steps: 6,
        max_output_tokens: 1000,
    }
}

fn call(name: &str, args: Value) -> Value {
    json!({"type":"function_call","name":name,"call_id":name,"arguments":args.to_string()})
}

fn patch(revision: u64) -> Value {
    call(
        "doc_patch",
        json!({"ops":[{"op":"set_memory","section":"agent","text":"accepted"}],"description":"Agent edit","expected_revision":revision}),
    )
}

fn completion(output: Vec<Value>) -> Completion {
    Completion {
        output,
        usage: Usage {
            input_tokens: 20,
            output_tokens: 10,
        },
    }
}

fn edit(bus: &mut CommandBus, section: &str) {
    bus.execute(
        vec![Command::SetMemory {
            section: section.into(),
            text: "user value".into(),
        }],
        Actor::user("editor"),
        "User edit",
        Some(bus.revision()),
    )
    .unwrap();
}

type Response<'a> = dyn FnMut(usize, &[Value], &[Value], &mut dyn FnMut(&str)) -> Completion + 'a;
struct Responses<'a> {
    count: usize,
    respond: Box<Response<'a>>,
}
impl Provider for Responses<'_> {
    fn model(&self) -> &str {
        "fixture-model"
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        _: u64,
        text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        let result = (self.respond)(self.count, input, tools, text);
        self.count += 1;
        Ok(result)
    }
}
fn output(input: &[Value], tool: &str) -> Value {
    serde_json::from_str(
        input
            .iter()
            .rev()
            .find(|v| v["type"] == "function_call_output" && v["call_id"] == tool)
            .unwrap()["output"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn editor_edits_during_provider_wait_then_agent_requeries_and_commits_undoable_transaction() {
    let bus = Mutex::new(CommandBus::new(Project::empty("Shared editor")).unwrap());
    let (request, pending) = mpsc::sync_channel(1);
    let (release, resumed) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let mut provider = Responses {
                count: 0,
                respond: Box::new(move |step, input, tools, text| {
                    match step {
                        0 => {
                            // The real editor thread must acquire the bus while this
                            // provider call is still in progress. Timeout fails safely.
                            request.send(()).unwrap();
                            resumed.recv_timeout(Duration::from_secs(10)).unwrap();
                            assert!(!tools.iter().any(|v| v["name"] == "script_write"
                                || v["name"] == "asset_import"
                                || v["name"] == "play_run"));
                            text("Prepared edit");
                            completion(vec![patch(0)])
                        }
                        1 => {
                            assert!(output(input, "doc_patch")["error"].as_str().is_some());
                            completion(vec![call("doc_query", json!({"path":"/memory"}))])
                        }
                        2 => {
                            let data = output(input, "doc_query");
                            assert_eq!(data["revision"], 1);
                            assert_eq!(data["untrusted_project_data"]["editor"], "user value");
                            completion(vec![patch(1)])
                        }
                        3 => {
                            assert_eq!(output(input, "doc_patch")["revision"], 2);
                            completion(vec![])
                        }
                        _ => panic!("unexpected provider call"),
                    }
                }),
            };
            let mut approvals = 0;
            let mut text = String::new();
            let report = agent()
                .run_shared(
                    &mut provider,
                    &bus,
                    &mut NoViewport,
                    "edit",
                    "shared-turn",
                    &AtomicBool::new(false),
                    &mut |_, _| {
                        assert!(
                            bus.try_lock().is_ok(),
                            "approval must not own the project lock"
                        );
                        approvals += 1;
                        true
                    },
                    &mut |delta| {
                        assert!(
                            bus.try_lock().is_ok(),
                            "streaming must not own the project lock"
                        );
                        text.push_str(delta);
                    },
                )
                .unwrap();
            assert_eq!(approvals, 2);
            assert_eq!(text, "Prepared edit");
            assert_eq!(report.transaction_ids.len(), 1);
            assert_eq!(report.usage.input_tokens, 80);
            assert_eq!(
                report
                    .tool_calls
                    .iter()
                    .map(|v| v.succeeded)
                    .collect::<Vec<_>>(),
                [false, true, true]
            );
        });
        pending.recv_timeout(Duration::from_secs(10)).unwrap();
        // try_lock keeps a regression from deadlocking this test.
        edit(
            &mut bus.try_lock().expect("provider must release bus"),
            "editor",
        );
        release.send(()).unwrap();
        worker.join().unwrap();
    });
    let mut bus = bus.lock().unwrap();
    let tx = &bus.history()[1];
    assert_eq!(tx.actor.origin, Origin::Agent);
    assert_eq!(tx.actor.model.as_deref(), Some("fixture-model"));
    assert_eq!(tx.actor.conversation_id.as_deref(), Some("shared-turn"));
    assert_eq!(bus.project().memory["agent"], "accepted");
    bus.undo().unwrap();
    assert!(!bus.project().memory.contains_key("agent"));
    assert_eq!(bus.project().memory["editor"], "user value");
}

#[test]
fn approval_can_edit_the_project_and_stale_agent_patch_does_not_overwrite_it() {
    let bus = Mutex::new(CommandBus::new(Project::empty("Approval")).unwrap());
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|step, input, _, _| {
            if step == 0 {
                completion(vec![patch(0)])
            } else {
                assert!(output(input, "doc_patch")["error"].is_string());
                completion(vec![])
            }
        }),
    };
    let report = agent()
        .run_shared(
            &mut provider,
            &bus,
            &mut NoViewport,
            "edit",
            "approval",
            &AtomicBool::new(false),
            &mut |_, _| {
                edit(&mut bus.try_lock().unwrap(), "approval");
                true
            },
            &mut |_| {},
        )
        .unwrap();
    assert!(report.transaction_ids.is_empty());
    assert_eq!(bus.lock().unwrap().revision(), 1);
    assert!(!bus.lock().unwrap().project().memory.contains_key("agent"));
}

#[test]
fn cancellation_during_approval_or_provider_response_never_applies_pending_edits() {
    for in_approval in [false, true] {
        let bus = Mutex::new(CommandBus::new(Project::empty("Cancel")).unwrap());
        let cancel = AtomicBool::new(false);
        let mut provider = Responses {
            count: 0,
            respond: Box::new(|_, _, _, _| {
                if !in_approval {
                    cancel.store(true, Ordering::Relaxed);
                }
                completion(vec![patch(0), patch(0)])
            }),
        };
        let mut runner = agent();
        let result = runner.run_shared(
            &mut provider,
            &bus,
            &mut NoViewport,
            "edit",
            "cancel",
            &cancel,
            &mut |_, _| {
                assert!(in_approval);
                assert!(bus.try_lock().is_ok());
                cancel.store(true, Ordering::Relaxed);
                true
            },
            &mut |_| {},
        );
        assert!(matches!(result, Err(AgentError::Interrupted)));
        assert_eq!(provider.count, 1);
        assert_eq!(runner.budget.used_tokens, 30);
        assert_eq!(bus.lock().unwrap().revision(), 0);
    }
}

#[test]
fn project_switch_and_poison_fail_closed_without_executing_provider_patch() {
    let bus = Mutex::new(CommandBus::new(Project::empty("First")).unwrap());
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|_, _, _, _| {
            *bus.try_lock().unwrap() = CommandBus::new(Project::empty("Second")).unwrap();
            completion(vec![patch(0)])
        }),
    };
    let result = agent().run_shared(
        &mut provider,
        &bus,
        &mut NoViewport,
        "edit",
        "switch",
        &AtomicBool::new(false),
        &mut |_, _| true,
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::ProjectChanged)));
    assert_eq!(bus.lock().unwrap().revision(), 0);
    drop(provider);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = bus.lock().unwrap();
        panic!("synthetic editor failure");
    }));
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|_, _, _, _| panic!("poisoned project must not contact provider")),
    };
    let result = agent().run_shared(
        &mut provider,
        &bus,
        &mut NoViewport,
        "edit",
        "poison",
        &AtomicBool::new(false),
        &mut |_, _| true,
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::ProjectUnavailable)));
    assert_eq!(provider.count, 0);
}

struct SharedHost<'a> {
    bus: &'a Mutex<CommandBus>,
    seen: Vec<&'static str>,
}
impl EngineHost for SharedHost<'_> {
    fn native_play_available(&self) -> bool {
        true
    }
    fn shared_play_available(&self) -> bool {
        true
    }
    fn play_run_checked(
        &mut self,
        project: &Project,
        revision: u64,
        _: RunRequest,
        _: &AtomicBool,
        validate: &mut dyn FnMut() -> Result<(), AgentError>,
    ) -> Result<Value, AgentError> {
        assert_eq!(revision, 1);
        assert!(project.memory.contains_key("screenshot"));
        validate()?;
        edit(
            &mut self.bus.try_lock().expect("play host must not hold bus"),
            "play",
        );
        assert!(
            !project.memory.contains_key("play"),
            "host must see its original consistent snapshot"
        );
        self.seen.push("play");
        Ok(json!({"tick":3,"source_revision":revision}))
    }
    fn play_state_query(&mut self, _: &str) -> Result<Value, AgentError> {
        assert!(self.bus.try_lock().is_ok());
        self.seen.push("query");
        Ok(json!(3))
    }
    fn screenshot(
        &mut self,
        project: &Project,
        _: Option<&str>,
        _: u32,
        _: u32,
    ) -> Result<Value, AgentError> {
        assert!(project.memory.is_empty());
        edit(
            &mut self
                .bus
                .try_lock()
                .expect("screenshot host must not hold bus"),
            "screenshot",
        );
        assert!(project.memory.is_empty());
        self.seen.push("screenshot");
        // No fake pixels: this test checks dispatch/locking, not perception.
        Err(AgentError::Tool("test host has no pixels".into()))
    }
}

#[test]
fn slow_hosts_receive_consistent_authoring_snapshots_without_locking_editor() {
    let bus = Mutex::new(CommandBus::new(Project::empty("Host snapshots")).unwrap());
    let mut host = SharedHost {
        bus: &bus,
        seen: vec![],
    };
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|step, _, tools, _| {
            assert!(tools.iter().any(|v| v["name"] == "play_run"));
            completion(match step {
                0 => vec![call("view_screenshot", json!({"width":320,"height":200}))],
                1 => vec![call("play_run", json!({"seconds":0.05}))],
                2 => vec![call("play_state_query", json!({"path":"/tick"}))],
                3 => vec![],
                _ => panic!("extra provider request"),
            })
        }),
    };
    let report = agent()
        .run_shared(
            &mut provider,
            &bus,
            &mut host,
            "play",
            "hosts",
            &AtomicBool::new(false),
            &mut |_, _| true,
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(host.seen, ["screenshot", "play", "query"]);
    assert_eq!(
        report
            .tool_calls
            .iter()
            .map(|v| v.succeeded)
            .collect::<Vec<_>>(),
        [false, true, true]
    );
    assert_eq!(bus.lock().unwrap().revision(), 2);
}

#[test]
fn shared_asset_and_script_tools_replay_and_undo_through_the_existing_journal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("tone.ogg"),
        include_bytes!("../../incant_assets/tests/fixtures/tone-440hz.ogg"),
    )
    .unwrap();
    let mut initial = Project::empty("Shared sources");
    let script = incant_doc::ScriptSource {
        id: incant_doc::new_id(),
        name: "Movement".into(),
        path: "movement.ts".into(),
    };
    initial.scripts.insert(script.id.clone(), script.clone());
    let bus = Mutex::new(
        CommandBus::persistent_with_sources(
            dir.path().join("journal"),
            initial.clone(),
            dir.path(),
        )
        .unwrap(),
    );
    let mut host = incant_agent::ProjectHost::new(&initial, dir.path(), NoViewport).unwrap();
    let source = "export const speed = 7;";
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|step, input, tools, _| {
            assert!(bus.try_lock().is_ok());
            for name in ["asset_import", "script_read", "script_write"] {
                assert!(tools.iter().any(|v| v["name"] == name));
            }
            completion(match step {
                0 => vec![call(
                    "asset_import",
                    json!({"sources":[{"source":"tone.ogg"}],"expected_revision":0,"description":"Import sound"}),
                )],
                1 => {
                    assert_eq!(output(input, "asset_import")["revision"], 1);
                    vec![call(
                        "script_write",
                        json!({"script_id":script.id,"source":source,"expected_source":{"state":"missing"},"expected_revision":1,"description":"Write movement"}),
                    )]
                }
                2 => {
                    assert_eq!(output(input, "script_write")["revision"], 2);
                    vec![call("script_read", json!({"script_id":script.id}))]
                }
                3 => {
                    assert_eq!(
                        output(input, "script_read")["untrusted_source_data"]["source"],
                        source
                    );
                    vec![call(
                        "asset_import",
                        json!({"sources":[{"source":"tone.ogg"}],"expected_revision":2,"description":"No-op reimport"}),
                    )]
                }
                4 => {
                    let imported = output(input, "asset_import");
                    assert_eq!(imported["revision"], 2);
                    assert!(imported["transaction_id"].is_null());
                    assert_eq!(imported["untrusted_project_data"][0]["changed"], false);
                    vec![]
                }
                _ => panic!("extra request"),
            })
        }),
    };
    let report = agent()
        .run_shared(
            &mut provider,
            &bus,
            &mut host,
            "author",
            "source-turn",
            &AtomicBool::new(false),
            &mut |_, _| {
                assert!(bus.try_lock().is_ok());
                true
            },
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(report.transaction_ids.len(), 2);
    assert!(report.tool_calls.iter().all(|v| v.succeeded));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("movement.ts")).unwrap(),
        source
    );
    drop(provider);
    drop(host);
    drop(bus);
    let mut reopened =
        CommandBus::persistent_with_sources(dir.path().join("journal"), initial, dir.path())
            .unwrap();
    assert_eq!(reopened.revision(), 2);
    for tx in reopened.history() {
        assert_eq!(tx.actor, Actor::agent("fixture-model", "source-turn"));
    }
    assert_eq!(reopened.project().assets.len(), 1);
    reopened.undo().unwrap();
    assert!(!dir.path().join("movement.ts").exists());
    assert_eq!(reopened.project().assets.len(), 1);
    reopened.undo().unwrap();
    assert!(reopened.project().assets.is_empty());
}

#[test]
fn unchecked_play_hosts_are_not_advertised_or_dispatched_for_shared_projects() {
    struct Unchecked;
    impl EngineHost for Unchecked {
        fn native_play_available(&self) -> bool {
            true
        }
        fn play_run(
            &mut self,
            _: &Project,
            _: u64,
            _: RunRequest,
            _: &AtomicBool,
        ) -> Result<Value, AgentError> {
            panic!("a shared agent must not use an unchecked host");
        }
        fn screenshot(
            &mut self,
            _: &Project,
            _: Option<&str>,
            _: u32,
            _: u32,
        ) -> Result<Value, AgentError> {
            Err(AgentError::Tool("no pixels".into()))
        }
    }
    let bus = Mutex::new(CommandBus::new(Project::empty("Capabilities")).unwrap());
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|step, input, tools, _| {
            assert!(
                !tools
                    .iter()
                    .any(|v| v["name"] == "play_run" || v["name"] == "play_state_query")
            );
            completion(if step == 0 {
                vec![call("play_run", json!({"seconds":0.1}))]
            } else {
                assert!(output(input, "play_run")["error"].is_string());
                vec![]
            })
        }),
    };
    let report = agent()
        .run_shared(
            &mut provider,
            &bus,
            &mut Unchecked,
            "play",
            "capabilities",
            &AtomicBool::new(false),
            &mut |_, _| true,
            &mut |_| {},
        )
        .unwrap();
    assert!(!report.tool_calls[0].succeeded);
    assert_eq!(bus.lock().unwrap().revision(), 0);
}

#[test]
fn loading_project_container_uses_one_bus_and_refuses_a_failed_reload() {
    use incant_agent::ProjectSlot;
    struct Slot(Option<CommandBus>);
    impl ProjectSlot for Slot {
        fn command_bus(&mut self) -> Result<&mut CommandBus, AgentError> {
            self.0.as_mut().ok_or(AgentError::ProjectUnavailable)
        }
    }
    let slot = Mutex::new(Slot(None));
    let cancel = AtomicBool::new(false);
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|_, _, _, _| panic!("loading project must not call provider")),
    };
    assert!(matches!(
        agent().run_shared(
            &mut provider,
            &slot,
            &mut NoViewport,
            "Edit",
            "turn",
            &cancel,
            &mut |_, _| true,
            &mut |_| {},
        ),
        Err(AgentError::ProjectUnavailable)
    ));
    slot.lock().unwrap().0 = Some(CommandBus::new(Project::empty("Loaded")).unwrap());
    let mut provider = Responses {
        count: 0,
        respond: Box::new(|step, _, _, _| {
            assert!(slot.try_lock().is_ok());
            match step {
                0 => completion(vec![patch(0)]),
                1 => {
                    let mut state = slot.lock().unwrap();
                    let bus = state.command_bus().unwrap();
                    assert_eq!(bus.revision(), 1);
                    assert_eq!(bus.history().len(), 1);
                    assert_eq!(bus.project().memory["agent"], "accepted");
                    bus.undo().unwrap();
                    assert!(!bus.project().memory.contains_key("agent"));
                    state.0 = None; // A failed reload invalidates this turn.
                    completion(vec![patch(2)])
                }
                _ => panic!("unexpected provider call"),
            }
        }),
    };
    assert!(matches!(
        agent().run_shared(
            &mut provider,
            &slot,
            &mut NoViewport,
            "Edit",
            "turn",
            &cancel,
            &mut |_, _| true,
            &mut |_| {},
        ),
        Err(AgentError::ProjectUnavailable)
    ));
    assert_eq!(provider.count, 2);
}
