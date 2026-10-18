//! Mock-host behavior only. These tests start no compiler/provider/native device.
use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, NoViewport, ProjectHost, Usage,
    provider::{Completion, Provider},
};
use incant_cmd::CommandBus;
use incant_doc::Project;
use incant_module_build::{Backend, Service, SourceFile, SourcePackage};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
struct Fake(AtomicUsize);
impl Backend for Fake {
    fn inspect(
        &self,
        _: &Path,
        _: &str,
        entry: &str,
        _: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<SourcePackage> {
        Ok(SourcePackage {
            entry: entry.into(),
            files: vec![SourceFile {
                path: entry.into(),
                text: "untrusted source data".into(),
            }],
        })
    }
    fn build(
        &self,
        _: &SourcePackage,
        cancel: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<incant_module_build::backend::Built> {
        self.0.fetch_add(1, Ordering::Relaxed);
        if cancel() {
            return Err(incant_module_build::Error::Cancelled);
        }
        Ok(incant_module_build::backend::Built {
            diagnostics: "untrusted compiler text".into(),
            receipt: json!({"mock":true}),
            candidate: "private-fake-candidate".into(),
        })
    }
}
struct Call(Option<Value>);
impl Provider for Call {
    fn model(&self) -> &str {
        "mock-provider"
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        Ok(Completion {
            output: self.0.take().into_iter().collect(),
            usage: Usage::default(),
        })
    }
}
#[test]
fn shared_service_build_approval_provenance_and_zero_mutation() {
    for mode in [
        ApprovalMode::Always,
        ApprovalMode::Destructive,
        ApprovalMode::Auto,
    ] {
        for allow in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let mut bus = CommandBus::new(Project::empty("Build host")).unwrap();
            let before = bus.project().clone();
            let fake = Arc::new(Fake(AtomicUsize::new(0)));
            let jobs = Arc::new(Service::new(fake.clone()));
            jobs.bind(&before.id, root.path()).unwrap();
            let mut host = ProjectHost::new(&before, root.path(), NoViewport)
                .unwrap()
                .with_module_jobs(jobs.clone());
            let read = incant_agent::dispatch(
                &mut bus,
                &mut host,
                "module_source_read",
                json!({"package":"logic","entry":"module.rs","expected_revision":0}),
                incant_cmd::Actor::user("editor"),
            )
            .unwrap();
            let request = json!({"snapshot_id":read["snapshot_id"],"expected_revision":0,"description":"Check source"});
            let mut provider = Call(Some(
                json!({"type":"function_call","name":"module_build","call_id":"build-1","arguments":request.to_string()}),
            ));
            let mut agent = Agent {
                budget: Budget {
                    max_tokens: 100_000,
                    used_tokens: 0,
                },
                approval: mode,
                max_steps: 3,
                max_output_tokens: 64,
            };
            let mut approvals = 0;
            let report = agent
                .run(
                    &mut provider,
                    &mut bus,
                    &mut host,
                    "Check Rust",
                    "conversation",
                    &AtomicBool::new(false),
                    &mut |name, args| {
                        assert_eq!(name, "module_build");
                        assert_eq!(args, &request);
                        approvals += 1;
                        allow
                    },
                    &mut |_| {},
                )
                .unwrap();
            let permitted = allow || matches!(mode, ApprovalMode::Auto);
            assert_eq!(approvals, usize::from(!matches!(mode, ApprovalMode::Auto)));
            assert_eq!(fake.0.load(Ordering::Relaxed), usize::from(permitted));
            assert_eq!(report.tool_calls[0].succeeded, permitted);
            if permitted {
                let result = jobs
                    .status(
                        &before.id,
                        &incant_module_build::JobQuery {
                            snapshot_id: read["snapshot_id"].as_str().unwrap().into(),
                        },
                    )
                    .unwrap();
                assert_eq!(result.state, "complete");
                assert!(result.candidate_id.is_some());
            }
            assert_eq!(bus.project(), &before);
            assert_eq!(bus.revision(), 0);
            assert!(bus.history().is_empty());
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
    }
}
#[test]
fn missing_host_unknown_config_fields_and_stale_snapshots_refuse() {
    let root = tempfile::tempdir().unwrap();
    let mut bus = CommandBus::new(Project::empty("Build host")).unwrap();
    assert!(
        incant_agent::dispatch(
            &mut bus,
            &mut NoViewport,
            "module_source_read",
            json!({"package":"logic","entry":"module.rs","expected_revision":0}),
            incant_cmd::Actor::user("editor")
        )
        .is_err()
    );
    let fake = Arc::new(Fake(AtomicUsize::new(0)));
    let jobs = Arc::new(Service::new(fake.clone()));
    jobs.bind(&bus.project().id, root.path()).unwrap();
    let mut host = ProjectHost::new(bus.project(), root.path(), NoViewport)
        .unwrap()
        .with_module_jobs(jobs.clone());
    for args in [
        json!({"package":"logic","entry":"module.rs","expected_revision":0,"rustc":"/bin/sh"}),
        json!({"package":"logic","entry":"module.rs","expected_revision":9}),
    ] {
        assert!(
            incant_agent::dispatch(
                &mut bus,
                &mut host,
                "module_source_read",
                args,
                incant_cmd::Actor::user("editor")
            )
            .is_err()
        );
    }
    assert!(
        incant_agent::dispatch(
            &mut bus,
            &mut host,
            "module_build",
            json!({"snapshot_id":"old","expected_revision":0,"description":"Check"}),
            incant_cmd::Actor::user("editor")
        )
        .is_err()
    );
    let status = incant_agent::dispatch(
        &mut bus,
        &mut host,
        "module_build_status",
        json!({"snapshot_id":"foreign"}),
        incant_cmd::Actor::user("editor"),
    );
    assert!(status.is_err());
    assert_eq!(fake.0.load(Ordering::Relaxed), 0);
}
