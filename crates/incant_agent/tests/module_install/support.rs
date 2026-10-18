use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, NoViewport, ProjectHost, Usage,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, BinaryVersion, CommandBus};
use incant_doc::{Project, RustModuleSource, new_id};
use incant_module_build::{
    Backend, BuildRequest, RegisteredSource, Service, SourceFile, SourcePackage,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
pub const TEXT: &str = "pub fn update() -> u32 { 7 }\n";
pub const NEXT: &str = "pub fn update() -> u32 { 9 }\n";
pub type Action = Box<dyn FnOnce() + Send>;
pub struct Fake {
    pub private: PathBuf,
    pub bus: Weak<Mutex<CommandBus>>,
    pub action: Mutex<Option<Action>>,
    pub wrong_prepared: AtomicBool,
}
impl Backend for Fake {
    fn inspect(
        &self,
        root: &Path,
        package: &str,
        entry: &str,
        cancel: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<SourcePackage> {
        assert!(
            self.bus.upgrade().unwrap().try_lock().is_ok(),
            "backend recapture must run without bus guard"
        );
        if let Some(action) = self.action.lock().unwrap().take() {
            action();
        }
        if cancel() {
            return Err(incant_module_build::Error::Cancelled);
        }
        let held = incant_source::RustPackageSnapshot::capture(root, &format!("{package}/{entry}"))
            .map_err(|e| incant_module_build::Error::Worker(e.to_string()))?;
        Ok(SourcePackage {
            entry: entry.into(),
            files: held
                .files()
                .iter()
                .map(|(path, text)| SourceFile {
                    path: path.clone(),
                    text: text.clone(),
                })
                .collect(),
        })
    }
    fn build(
        &self,
        source: &SourcePackage,
        _: &dyn Fn() -> bool,
    ) -> incant_module_build::Result<incant_module_build::backend::Built> {
        assert!(self.bus.upgrade().unwrap().try_lock().is_ok());
        let value = if source.files.iter().any(|f| f.text == NEXT) {
            9
        } else {
            7
        };
        let raw=wat::parse_str(format!(r#"(module (memory (export "memory") 1) (func (export "incant_abi") (result i32) i32.const 1) (func (export "update") (param i64 f64)) (func (export "fixture_value") (result i32) i32.const {value}))"#)).unwrap();
        let captured = incant_wasm_cook::CapturedModule::capture(&raw).unwrap();
        let candidate = self.private.join(format!("{}.wasm", new_id()));
        fs::write(&candidate, &raw)?;
        Ok(incant_module_build::backend::Built {
            candidate,
            diagnostics: "synthesized valid Wasm; no compiler claim".into(),
            receipt: json!({"mock":true,"raw_sha256":format!("{:x}",Sha256::digest(&raw)),"raw_bytes":raw.len(),"prepared_sha256":if self.wrong_prepared.load(Ordering::Acquire){"0".repeat(64)}else{captured.prepared_sha256().into()},"policy_sha256":"a".repeat(64),"host_tool_manifest_sha256":"b".repeat(64),"host_closeout_sha256":"c".repeat(64),"compiler_version":"mock","sdk_sources":{},"service_sources":{}}),
        })
    }
}
pub struct Fixture {
    pub bus: Arc<Mutex<CommandBus>>,
    pub jobs: Arc<Service>,
    pub fake: Arc<Fake>,
    pub project: Project,
    pub module: RustModuleSource,
    pub root: tempfile::TempDir,
    pub private: tempfile::TempDir,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("logic")).unwrap();
        fs::write(root.path().join("logic/main.rs"), TEXT).unwrap();
        fs::write(root.path().join("logic/helper.rs"), "pub const X:u8=1;").unwrap();
        let module = RustModuleSource {
            id: new_id(),
            name: "Movement".into(),
            path: "logic/main.rs".into(),
        };
        let mut project = Project::empty("Installed Rust module");
        project
            .module_sources
            .insert(module.id.clone(), module.clone());
        fs::write(
            root.path().join("game.incant"),
            project.canonical_text().unwrap(),
        )
        .unwrap();
        let bus = Arc::new(Mutex::new(
            CommandBus::open_project_file(root.path().join("game.incant")).unwrap(),
        ));
        let fake = Arc::new(Fake {
            private: private.path().canonicalize().unwrap(),
            bus: Arc::downgrade(&bus),
            action: Mutex::new(None),
            wrong_prepared: AtomicBool::new(false),
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
            private,
        }
    }
    pub fn host(&self) -> ProjectHost<NoViewport> {
        ProjectHost::new(&self.project, self.root.path(), NoViewport)
            .unwrap()
            .with_module_jobs(self.jobs.clone())
    }
    pub fn authority(&self) -> (RegisteredSource, u64, BinaryVersion) {
        let bus = self.bus.lock().unwrap();
        let read = bus.read_module_source(&self.module.id).unwrap();
        (
            RegisteredSource {
                bus_session_id: bus.session_id().into(),
                module_id: self.module.id.clone(),
                path: read.path,
                version: read.version,
            },
            bus.revision(),
            bus.module_binary_version(&self.module.id).unwrap(),
        )
    }
    pub fn build(&self) -> Value {
        let (authority, revision, destination) = self.authority();
        let cancel = AtomicBool::new(false);
        let source = self
            .jobs
            .inspect_registered(
                &self.project.id,
                authority.clone(),
                revision,
                &cancel,
                &mut || Ok(()),
            )
            .unwrap();
        let built = self
            .jobs
            .build_registered(
                &self.project.id,
                BuildRequest {
                    snapshot_id: source.snapshot_id.clone(),
                    expected_revision: revision,
                    description: "Build registered fixture".into(),
                },
                incant_module_build::Actor {
                    origin: "user".into(),
                    actor: "fixture".into(),
                    model: None,
                    conversation_id: None,
                },
                authority.clone(),
                &cancel,
                &mut || Ok(()),
            )
            .unwrap();
        json!({"module_id":self.module.id,"snapshot_id":source.snapshot_id,"candidate_id":built.candidate_id,"expected_revision":revision,"expected_source":authority.version,"expected_destination":destination,"description":"Install approved candidate"})
    }
    pub fn run(
        &self,
        args: &Value,
        mode: ApprovalMode,
        cancel: &AtomicBool,
        approve: &mut dyn FnMut(&str, &Value) -> bool,
    ) -> Result<incant_agent::TurnReport, AgentError> {
        let mut provider = Call {
            next: Some(
                json!({"type":"function_call","name":"module_install_registered","call_id":"install","arguments":args.to_string()}),
            ),
        };
        Agent {
            budget: Budget {
                max_tokens: 1_000_000,
                used_tokens: 0,
            },
            approval: mode,
            max_steps: 3,
            max_output_tokens: 64,
        }
        .run_shared(
            &mut provider,
            self.bus.as_ref(),
            &mut self.host(),
            "Install",
            "turn",
            cancel,
            approve,
            &mut |_| {},
        )
    }
    pub fn reopen(&self) {
        let mut bus = self
            .bus
            .try_lock()
            .expect("no guard during approval/backend");
        let before = bus.project().clone();
        let revision = bus.revision();
        let session = bus.session_id().to_owned();
        let old = std::mem::replace(&mut *bus, CommandBus::new(self.project.clone()).unwrap());
        drop(old);
        *bus = CommandBus::open_project_file(self.root.path().join("game.incant")).unwrap();
        assert_eq!(bus.project(), &before);
        assert_eq!(bus.revision(), revision);
        assert_ne!(bus.session_id(), session);
    }
    pub fn unchanged(&self, len: u64) {
        let bus = self.bus.lock().unwrap();
        assert_eq!(bus.revision(), 0);
        assert!(bus.project().assets.is_empty());
        assert!(bus.history().is_empty());
        assert!(!self.root.path().join("logic/main.wasm").exists());
        assert_eq!(
            fs::metadata(self.root.path().join("game.journal"))
                .unwrap()
                .len(),
            len
        );
    }
    pub fn journal_len(&self) -> u64 {
        fs::metadata(self.root.path().join("game.journal"))
            .unwrap()
            .len()
    }
}
struct Call {
    next: Option<Value>,
}
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
            output: self.next.take().into_iter().collect(),
            usage: Usage::default(),
        })
    }
}
pub fn write(f: &Fixture, text: &str) {
    let mut bus = f.bus.lock().unwrap();
    let revision = bus.revision();
    let source = bus.read_module_source(&f.module.id).unwrap().version;
    bus.execute(
        vec![incant_cmd::Command::WriteModuleSource {
            module_id: f.module.id.clone(),
            source: text.into(),
            expected_source: source,
        }],
        Actor::user("author"),
        "Edit",
        Some(revision),
    )
    .unwrap();
}
