//! Synthesized shared-host behavior, not native compiler or sandbox evidence.
use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, NoViewport, ProjectHost, Usage, dispatch,
    provider::{Completion, Provider},
    tools,
};
use incant_cmd::{Actor, Command, CommandBus, SourceVersion};
use incant_doc::{Project, RustModuleSource, new_id};
use incant_module_build::{Backend, Service, SourceFile, SourcePackage};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
const TEXT: &str = "pub fn update() -> u32 { 7 }\n";
const NEXT: &str = "pub fn update() -> u32 { 9 }\n";
type Action = Box<dyn FnOnce() + Send>;
struct Fake {
    builds: AtomicUsize,
    inspects: AtomicUsize,
    action: Mutex<Option<Action>>,
}
impl Backend for Fake {
    fn inspect(
        &self,
        root: &Path,
        package: &str,
        entry: &str,
        cancel: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<SourcePackage> {
        if cancel() {
            return Err(incant_module_build::Error::Cancelled);
        }
        self.inspects.fetch_add(1, Ordering::Relaxed);
        Ok(SourcePackage {
            entry: entry.into(),
            files: vec![SourceFile {
                path: entry.into(),
                text: std::fs::read_to_string(root.join(package).join(entry))?,
            }],
        })
    }
    fn build(
        &self,
        _: &SourcePackage,
        cancel: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<incant_module_build::backend::Built> {
        self.builds.fetch_add(1, Ordering::Relaxed);
        if let Some(action) = self.action.lock().unwrap().take() {
            action();
        }
        if cancel() {
            return Err(incant_module_build::Error::Cancelled);
        }
        Ok(incant_module_build::backend::Built {
            diagnostics: "untrusted mock diagnostic".into(),
            receipt: json!({"mock":true}),
            candidate: "private-mock".into(),
        })
    }
}
struct Fixture {
    // All root-owning objects are dropped before TempDir on Windows too.
    bus: Arc<Mutex<CommandBus>>,
    jobs: Arc<Service>,
    fake: Arc<Fake>,
    project: Project,
    module: RustModuleSource,
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("logic")).unwrap();
        std::fs::write(root.path().join("logic/main.rs"), TEXT).unwrap();
        let module = RustModuleSource {
            id: new_id(),
            name: "Movement".into(),
            path: "logic/main.rs".into(),
        };
        let mut project = Project::empty("Registered module tools");
        project
            .module_sources
            .insert(module.id.clone(), module.clone());
        let bus = Arc::new(Mutex::new(
            CommandBus::persistent_with_sources(
                root.path().join("journal"),
                project.clone(),
                root.path(),
            )
            .unwrap(),
        ));
        let fake = Arc::new(Fake {
            builds: AtomicUsize::new(0),
            inspects: AtomicUsize::new(0),
            action: Mutex::new(None),
        });
        let jobs = Arc::new(Service::new(fake.clone()));
        jobs.bind(&project.id, root.path()).unwrap();
        Self {
            bus,
            jobs,
            fake,
            project,
            module,
            root,
        }
    }
    fn host(&self) -> ProjectHost<NoViewport> {
        ProjectHost::new(&self.project, self.root.path(), NoViewport)
            .unwrap()
            .with_module_jobs(self.jobs.clone())
    }
    fn observed(&self) -> Value {
        json!({"module_id":self.module.id,"expected_source":SourceVersion::of(Some(TEXT)),"expected_revision":0})
    }
    fn write(&self) -> Value {
        let mut a = self.observed();
        a["source"] = json!(NEXT);
        a["description"] = json!("Tune movement");
        a
    }
    fn inspect(&self) -> Value {
        dispatch(
            &mut self.bus.lock().unwrap(),
            &mut self.host(),
            "module_inspect",
            self.observed(),
            Actor::user("author"),
        )
        .unwrap()
    }
    fn request(&self) -> Value {
        let view = self.inspect();
        let mut a = self.observed();
        a["snapshot_id"] = view["snapshot_id"].clone();
        a["description"] = json!("Check movement");
        a
    }
}
fn reopen(bus: &Mutex<CommandBus>, root: &Path, project: &Project) {
    let mut held = bus
        .try_lock()
        .expect("approval/backend must not hold bus guard");
    let identity = held.session_id().to_owned();
    let revision = held.revision();
    let before = held.project().clone();
    // Release the old source-store/journal locks before reopening the same files.
    let old = std::mem::replace(&mut *held, CommandBus::new(project.clone()).unwrap());
    drop(old);
    *held =
        CommandBus::persistent_with_sources(root.join("journal"), project.clone(), root).unwrap();
    assert_eq!(held.project(), &before);
    assert_eq!(held.revision(), revision);
    assert_ne!(held.session_id(), identity);
}
struct Call {
    next: Option<Value>,
    names: Vec<String>,
}
impl Provider for Call {
    fn model(&self) -> &str {
        "mock-provider"
    }
    fn complete(
        &mut self,
        _: &[Value],
        tools: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.names = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap().into())
            .collect();
        Ok(Completion {
            output: self.next.take().into_iter().collect(),
            usage: Usage::default(),
        })
    }
}
fn provider(name: &str, args: &Value) -> Call {
    Call {
        next: Some(
            json!({"type":"function_call","name":name,"call_id":name,"arguments":args.to_string()}),
        ),
        names: vec![],
    }
}
fn agent(mode: ApprovalMode) -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 1_000_000,
            used_tokens: 0,
        },
        approval: mode,
        max_steps: 3,
        max_output_tokens: 64,
    }
}
#[test]
fn registered_source_write_uses_approval_actor_and_durable_undo_redo_without_worker() {
    for mode in [
        ApprovalMode::Always,
        ApprovalMode::Destructive,
        ApprovalMode::Auto,
    ] {
        for allow in [false, true] {
            let f = Fixture::new();
            let args = f.write();
            let mut p = provider("module_write", &args);
            let mut approvals = 0;
            let report = agent(mode)
                .run_shared(
                    &mut p,
                    f.bus.as_ref(),
                    &mut NoViewport,
                    "Tune",
                    "turn",
                    &AtomicBool::new(false),
                    &mut |name, a| {
                        assert_eq!(name, "module_write");
                        assert_eq!(a, &args);
                        assert!(f.bus.try_lock().is_ok());
                        approvals += 1;
                        allow
                    },
                    &mut |_| {},
                )
                .unwrap();
            let accepted = allow || matches!(mode, ApprovalMode::Auto);
            assert_eq!(approvals, usize::from(!matches!(mode, ApprovalMode::Auto)));
            assert_eq!(report.tool_calls[0].succeeded, accepted);
            assert!(p.names.iter().any(|n| n == "module_read"));
            assert!(p.names.iter().any(|n| n == "module_write"));
            assert!(!p.names.iter().any(|n| n == "module_inspect"));
            let mut bus = f.bus.lock().unwrap();
            assert_eq!(bus.revision(), u64::from(accepted));
            assert_eq!(
                std::fs::read_to_string(f.root.path().join(&f.module.path)).unwrap(),
                if accepted { NEXT } else { TEXT }
            );
            if accepted {
                assert_eq!(report.transaction_ids, [bus.history()[0].id.clone()]);
                assert_eq!(
                    bus.history()[0].actor,
                    Actor::agent("mock-provider", "turn")
                );
                assert_eq!(
                    bus.history()[0].source_edit.as_ref().unwrap().script_id,
                    f.module.id
                );
                bus.undo().unwrap();
                assert_eq!(
                    bus.read_module_source(&f.module.id)
                        .unwrap()
                        .source
                        .as_deref(),
                    Some(TEXT)
                );
                bus.redo().unwrap();
                assert_eq!(
                    bus.read_module_source(&f.module.id)
                        .unwrap()
                        .source
                        .as_deref(),
                    Some(NEXT)
                );
            } else {
                assert!(report.transaction_ids.is_empty());
            }
            assert_eq!(f.fake.builds.load(Ordering::Relaxed), 0);
        }
    }
}
#[test]
fn registered_read_and_write_reject_unknown_ids_paths_fields_stale_cas_and_revision() {
    let f = Fixture::new();
    let mut bus = f.bus.lock().unwrap();
    let read = dispatch(
        &mut bus,
        &mut NoViewport,
        "module_read",
        json!({"module_id":f.module.id}),
        Actor::user("author"),
    )
    .unwrap();
    assert_eq!(read["untrusted_source_data"]["source"], TEXT);
    assert_eq!(
        read["untrusted_source_data"]["version"],
        json!(SourceVersion::of(Some(TEXT)))
    );
    for args in [
        json!({"module_id":new_id()}),
        json!({"module_id":f.module.id,"path":"other.rs"}),
    ] {
        assert!(
            dispatch(
                &mut bus,
                &mut NoViewport,
                "module_read",
                args,
                Actor::user("author")
            )
            .is_err()
        );
    }
    for key in ["module_id", "expected_source", "expected_revision", "path"] {
        let mut args = f.write();
        args[key] = match key {
            "module_id" => json!(new_id()),
            "expected_source" => json!(SourceVersion::Missing),
            "expected_revision" => json!(1),
            _ => json!("/tmp/any.rs"),
        };
        assert!(
            dispatch(
                &mut bus,
                &mut NoViewport,
                "module_write",
                args,
                Actor::user("author")
            )
            .is_err()
        );
    }
    let mut too_large = f.write();
    too_large["source"] = json!("x".repeat(65537));
    assert!(
        dispatch(
            &mut bus,
            &mut NoViewport,
            "module_write",
            too_large,
            Actor::user("author")
        )
        .is_err()
    );
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert_eq!(
        std::fs::read_to_string(f.root.path().join(&f.module.path)).unwrap(),
        TEXT
    );
}
#[test]
fn approved_write_fences_external_source_edit_revision_registration_reopen_and_cancel() {
    for change in 0..5 {
        let f = Fixture::new();
        let mut p = provider("module_write", &f.write());
        let cancel = AtomicBool::new(false);
        let result = agent(ApprovalMode::Destructive).run_shared(
            &mut p,
            f.bus.as_ref(),
            &mut NoViewport,
            "Tune",
            "turn",
            &cancel,
            &mut |_, _| {
                match change {
                    0 => std::fs::write(f.root.path().join(&f.module.path), "// external").unwrap(),
                    1 => {
                        let mut bus = f.bus.try_lock().unwrap();
                        bus.execute(
                            vec![Command::SetMemory {
                                section: "editor".into(),
                                text: "changed".into(),
                            }],
                            Actor::user("editor"),
                            "Edit",
                            Some(0),
                        )
                        .unwrap();
                    }
                    2 => {
                        let mut bus = f.bus.try_lock().unwrap();
                        let mut module = f.module.clone();
                        module.path = "logic/other.rs".into();
                        bus.execute(
                            vec![Command::UpsertModuleSource { module }],
                            Actor::user("editor"),
                            "Retarget",
                            Some(0),
                        )
                        .unwrap();
                    }
                    3 => reopen(&f.bus, f.root.path(), &f.project),
                    _ => cancel.store(true, Ordering::Release),
                }
                true
            },
            &mut |_| {},
        );
        if matches!(change, 3 | 4) {
            assert!(matches!(
                result,
                Err(AgentError::ProjectChanged | AgentError::Interrupted)
            ));
        } else {
            assert!(!result.unwrap().tool_calls[0].succeeded);
        }
        let bus = f.bus.lock().unwrap();
        assert!(
            !bus.history()
                .iter()
                .any(|tx| tx.actor.model.as_deref() == Some("mock-provider"))
        );
        assert_ne!(
            std::fs::read_to_string(f.root.path().join(&f.module.path)).unwrap(),
            NEXT
        );
    }
}
#[test]
fn registered_build_approval_and_provenance_share_the_host_service_without_mutation() {
    for allow in [false, true] {
        let f = Fixture::new();
        let args = f.request();
        let mut p = provider("module_build_registered", &args);
        let report = agent(ApprovalMode::Destructive)
            .run_shared(
                &mut p,
                f.bus.as_ref(),
                &mut f.host(),
                "Build",
                "turn",
                &AtomicBool::new(false),
                &mut |name, observed| {
                    assert_eq!(name, "module_build_registered");
                    assert_eq!(observed, &args);
                    assert!(f.bus.try_lock().is_ok());
                    allow
                },
                &mut |_| {},
            )
            .unwrap();
        assert_eq!(report.tool_calls[0].succeeded, allow);
        assert!(report.transaction_ids.is_empty());
        assert_eq!(f.fake.builds.load(Ordering::Relaxed), usize::from(allow));
        assert_eq!(f.bus.lock().unwrap().project(), &f.project);
        assert_eq!(f.bus.lock().unwrap().revision(), 0);
        let state = f
            .jobs
            .status(
                &f.project.id,
                &incant_module_build::JobQuery {
                    snapshot_id: args["snapshot_id"].as_str().unwrap().into(),
                },
            )
            .unwrap();
        assert_eq!(state.state, if allow { "complete" } else { "ready" });
    }
    let f = Fixture::new();
    let args = f.request();
    let built = dispatch(
        &mut f.bus.lock().unwrap(),
        &mut f.host(),
        "module_build_registered",
        args,
        Actor::user("author"),
    )
    .unwrap();
    assert_eq!(
        built["provenance"]["registered_source"]["module_id"],
        f.module.id
    );
    assert_eq!(
        built["provenance"]["registered_source"]["bus_session_id"],
        f.bus.lock().unwrap().session_id()
    );
    assert_eq!(built["provenance"]["not_installed"], true);
    assert_eq!(built["untrusted_diagnostics"], "untrusted mock diagnostic");
}
#[test]
fn registered_build_rejects_registration_source_or_session_change_after_inspection() {
    for change in 0..3 {
        let f = Fixture::new();
        let mut args = f.request();
        match change {
            0 => {
                std::fs::write(f.root.path().join(&f.module.path), NEXT).unwrap();
                args["expected_source"] = json!(SourceVersion::of(Some(NEXT)));
            }
            1 => {
                let mut bus = f.bus.lock().unwrap();
                let mut module = f.module.clone();
                module.path = "logic/other.rs".into();
                std::fs::write(f.root.path().join(&module.path), TEXT).unwrap();
                bus.execute(
                    vec![Command::UpsertModuleSource { module }],
                    Actor::user("editor"),
                    "Retarget",
                    Some(0),
                )
                .unwrap();
                args["expected_revision"] = json!(1);
            }
            _ => {
                // Same ID/revision/source and even unchanged service binding must not inherit a snapshot.
                reopen(&f.bus, f.root.path(), &f.project);
            }
        }
        assert!(
            dispatch(
                &mut f.bus.lock().unwrap(),
                &mut f.host(),
                "module_build_registered",
                args,
                Actor::user("author")
            )
            .is_err()
        );
        assert_eq!(f.fake.builds.load(Ordering::Relaxed), 0);
    }
}
#[test]
fn registered_build_rechecks_authority_after_backend_and_discards_candidate() {
    for change in 0..3 {
        let f = Fixture::new();
        let args = f.request();
        let bus = f.bus.clone();
        let root = f.root.path().to_owned();
        let module = f.module.clone();
        let project = f.project.clone();
        *f.fake.action.lock().unwrap() = Some(Box::new(move || match change {
            0 => std::fs::write(root.join(module.path), NEXT).unwrap(),
            1 => {
                let mut bus = bus.lock().unwrap();
                bus.execute(
                    vec![Command::RemoveModuleSource {
                        module_id: module.id,
                    }],
                    Actor::user("editor"),
                    "Remove",
                    Some(0),
                )
                .unwrap();
            }
            _ => {
                reopen(&bus, &root, &project);
            }
        }));
        let mut p = provider("module_build_registered", &args);
        let report = agent(ApprovalMode::Auto)
            .run_shared(
                &mut p,
                f.bus.as_ref(),
                &mut f.host(),
                "Build",
                "turn",
                &AtomicBool::new(false),
                &mut |_, _| panic!("auto"),
                &mut |_| {},
            )
            .unwrap();
        assert!(!report.tool_calls[0].succeeded);
        assert_eq!(f.fake.builds.load(Ordering::Relaxed), 1);
        let state = f
            .jobs
            .status(
                &f.project.id,
                &incant_module_build::JobQuery {
                    snapshot_id: args["snapshot_id"].as_str().unwrap().into(),
                },
            )
            .unwrap();
        assert_eq!(state.state, "failed");
        assert!(state.candidate_id.is_none());
    }
}
#[test]
fn pending_source_recovery_refuses_read_write_inspect_and_build_before_backend() {
    let f = Fixture::new();
    let build = f.request();
    // This is an existing source-store marker, not an alternate recovery path.
    std::fs::write(
        f.root.path().join(".incant-source-transactions/pending"),
        "invalid pending data",
    )
    .unwrap();
    for (name, args) in [
        ("module_read", json!({"module_id":f.module.id})),
        ("module_write", f.write()),
        ("module_inspect", f.observed()),
        ("module_build_registered", build),
    ] {
        assert!(
            dispatch(
                &mut f.bus.lock().unwrap(),
                &mut f.host(),
                name,
                args,
                Actor::user("author")
            )
            .is_err()
        );
    }
    assert_eq!(f.fake.inspects.load(Ordering::Relaxed), 1);
    assert_eq!(f.fake.builds.load(Ordering::Relaxed), 0);
}
#[test]
fn registered_tools_expose_only_stable_id_and_observed_authority_arguments() {
    for name in [
        "module_read",
        "module_write",
        "module_inspect",
        "module_build_registered",
    ] {
        let t = tools()
            .into_iter()
            .find(|tool| tool["name"] == name)
            .unwrap();
        let p = &t["parameters"];
        assert_eq!(p["additionalProperties"], false);
        assert!(p["properties"]["module_id"].is_object());
        for forbidden in [
            "path",
            "package",
            "entry",
            "root",
            "rustc",
            "actor",
            "bus_session_id",
        ] {
            assert!(
                p["properties"].get(forbidden).is_none(),
                "{name}/{forbidden}"
            );
        }
    }
    let mut bus = CommandBus::new(Project::empty("No source store")).unwrap();
    let mut p = Call {
        next: None,
        names: vec![],
    };
    agent(ApprovalMode::Auto)
        .run(
            &mut p,
            &mut bus,
            &mut NoViewport,
            "Tools",
            "turn",
            &AtomicBool::new(false),
            &mut |_, _| false,
            &mut |_| {},
        )
        .unwrap();
    assert!(!p.names.iter().any(|n| matches!(
        n.as_str(),
        "module_read" | "module_write" | "module_inspect" | "module_build_registered"
    )));
}

#[test]
fn approved_build_rechecks_source_registration_session_and_cancel_before_backend() {
    for change in 0..4 {
        let f = Fixture::new();
        let args = f.request();
        let mut p = provider("module_build_registered", &args);
        let cancel = AtomicBool::new(false);
        let result = agent(ApprovalMode::Destructive).run_shared(
            &mut p,
            f.bus.as_ref(),
            &mut f.host(),
            "Build",
            "turn",
            &cancel,
            &mut |_, _| {
                match change {
                    0 => std::fs::write(f.root.path().join(&f.module.path), NEXT).unwrap(),
                    1 => {
                        let mut bus = f.bus.try_lock().unwrap();
                        bus.execute(
                            vec![Command::RemoveModuleSource {
                                module_id: f.module.id.clone(),
                            }],
                            Actor::user("editor"),
                            "Remove",
                            Some(0),
                        )
                        .unwrap();
                    }
                    2 => reopen(&f.bus, f.root.path(), &f.project),
                    _ => cancel.store(true, Ordering::Release),
                }
                true
            },
            &mut |_| {},
        );
        if matches!(change, 2 | 3) {
            assert!(matches!(
                result,
                Err(AgentError::ProjectChanged | AgentError::Interrupted)
            ));
        } else {
            assert!(!result.unwrap().tool_calls[0].succeeded);
        }
        assert_eq!(f.fake.builds.load(Ordering::Relaxed), 0);
        assert!(
            f.bus
                .lock()
                .unwrap()
                .history()
                .iter()
                .all(|tx| tx.actor.origin != incant_doc::Origin::Agent)
        );
    }
}
#[test]
fn registered_build_cancellation_during_backend_keeps_candidate_unpublished() {
    let f = Fixture::new();
    let args = f.request();
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    *f.fake.action.lock().unwrap() = Some(Box::new(move || trigger.store(true, Ordering::Release)));
    let mut p = provider("module_build_registered", &args);
    assert!(matches!(
        agent(ApprovalMode::Auto).run_shared(
            &mut p,
            f.bus.as_ref(),
            &mut f.host(),
            "Build",
            "turn",
            &cancel,
            &mut |_, _| false,
            &mut |_| {}
        ),
        Err(AgentError::Interrupted)
    ));
    let state = f
        .jobs
        .status(
            &f.project.id,
            &incant_module_build::JobQuery {
                snapshot_id: args["snapshot_id"].as_str().unwrap().into(),
            },
        )
        .unwrap();
    assert_eq!(state.state, "cancelled");
    assert!(state.candidate_id.is_none());
    assert_eq!(f.fake.builds.load(Ordering::Relaxed), 1);
}

#[test]
fn missing_registered_source_can_be_authored_but_cannot_be_inspected_for_build() {
    let f = Fixture::new();
    std::fs::remove_file(f.root.path().join(&f.module.path)).unwrap();
    std::fs::remove_dir(f.root.path().join("logic")).unwrap();
    let mut bus = f.bus.lock().unwrap();
    let read = dispatch(
        &mut bus,
        &mut NoViewport,
        "module_read",
        json!({"module_id":f.module.id}),
        Actor::user("author"),
    )
    .unwrap();
    assert_eq!(read["untrusted_source_data"]["source"], Value::Null);
    assert_eq!(
        read["untrusted_source_data"]["version"],
        json!(SourceVersion::Missing)
    );
    let mut args = f.observed();
    args["expected_source"] = json!(SourceVersion::Missing);
    assert!(
        dispatch(
            &mut bus,
            &mut f.host(),
            "module_inspect",
            args.clone(),
            Actor::user("author")
        )
        .is_err()
    );
    assert_eq!(f.fake.inspects.load(Ordering::Relaxed), 0);
    args["source"] = json!(NEXT);
    args["description"] = json!("Create registered source");
    dispatch(
        &mut bus,
        &mut NoViewport,
        "module_write",
        args,
        Actor::user("author"),
    )
    .unwrap();
    assert_eq!(
        bus.read_module_source(&f.module.id)
            .unwrap()
            .source
            .as_deref(),
        Some(NEXT)
    );
    assert!(
        bus.history()[0]
            .source_edit
            .as_ref()
            .unwrap()
            .created_tree
            .is_some()
    );
    bus.undo().unwrap();
    assert!(!f.root.path().join("logic").exists());
    assert!(
        bus.read_module_source(&f.module.id)
            .unwrap()
            .source
            .is_none()
    );
    bus.redo().unwrap();
    assert!(f.root.path().join("logic").is_dir());
    assert_eq!(
        bus.read_module_source(&f.module.id)
            .unwrap()
            .source
            .as_deref(),
        Some(NEXT)
    );
}
