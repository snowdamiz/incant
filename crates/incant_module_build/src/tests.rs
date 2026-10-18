use super::*;
use std::sync::atomic::AtomicUsize;

struct Mock {
    source: Mutex<SourcePackage>,
    builds: AtomicUsize,
    action: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    fail: AtomicBool,
    unclosed: AtomicBool,
}
impl Mock {
    fn new() -> Self {
        Self {
            source: Mutex::new(SourcePackage {
                entry: "module.rs".into(),
                files: vec![SourceFile {
                    path: "module.rs".into(),
                    text: "fn entry(){}".into(),
                }],
            }),
            builds: AtomicUsize::new(0),
            action: Mutex::new(None),
            fail: AtomicBool::new(false),
            unclosed: AtomicBool::new(false),
        }
    }
}
impl Backend for Mock {
    fn inspect(
        &self,
        _: &Path,
        _: &str,
        _: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Result<SourcePackage> {
        if cancel() {
            return Err(Error::Cancelled);
        }
        Ok(self.source.lock().unwrap().clone())
    }
    fn build(&self, _: &SourcePackage, cancel: &dyn Fn() -> bool) -> Result<backend::Built> {
        self.builds.fetch_add(1, Ordering::Relaxed);
        if let Some(action) = self.action.lock().unwrap().take() {
            action();
        }
        if self.unclosed.load(Ordering::Relaxed) {
            return Err(Error::Cleanup("actual owner closure missing".into()));
        }
        if cancel() {
            return Err(Error::Cancelled);
        }
        if self.fail.load(Ordering::Relaxed) {
            return Err(Error::Worker("compiler error".into()));
        }
        Ok(backend::Built {
            diagnostics: "data only".into(),
            receipt: serde_json::json!({"raw_sha256":"mock-not-native"}),
            candidate: PathBuf::from("private-mock-candidate"),
        })
    }
}
struct Fixture {
    service: Arc<Service>,
    backend: Arc<Mock>,
    project: String,
    cancel: AtomicBool,
    // Drop the held service root before TempDir cleanup on Windows.
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let backend = Arc::new(Mock::new());
        let service = Arc::new(Service::new(backend.clone()));
        let project = ulid::Ulid::new().to_string();
        service.bind(&project, root.path()).unwrap();
        Self {
            root,
            service,
            backend,
            project,
            cancel: AtomicBool::new(false),
        }
    }
    fn inspect(&self) -> SourceView {
        self.service
            .inspect(
                &self.project,
                SourceQuery {
                    package: "modules/logic".into(),
                    entry: "module.rs".into(),
                    expected_revision: 3,
                },
                &self.cancel,
                &mut || Ok(()),
            )
            .unwrap()
    }
    fn request(&self, id: &str) -> BuildRequest {
        BuildRequest {
            snapshot_id: id.into(),
            expected_revision: 3,
            description: "Check code".into(),
        }
    }
    fn actor(&self) -> Actor {
        Actor {
            origin: "agent".into(),
            actor: "engine-agent".into(),
            model: Some("mock".into()),
            conversation_id: Some("turn".into()),
        }
    }
    fn build(&self, id: &str) -> Result<BuildResult> {
        self.service.build(
            &self.project,
            self.request(id),
            self.actor(),
            &self.cancel,
            &mut || Ok(()),
        )
    }
}
#[test]
fn manifest_order_is_stable_and_ambiguity_bounds_fail() {
    let mut package = Mock::new().source.into_inner().unwrap();
    let initial = package.digest().unwrap();
    package.files.push(SourceFile {
        path: "helper.rs".into(),
        text: "const X:u32=1;".into(),
    });
    let a = package.digest().unwrap();
    package.files.reverse();
    assert_eq!(a, package.digest().unwrap());
    assert_ne!(a, initial);
    for path in [
        "../bad.rs",
        "/bad.rs",
        "./bad.rs",
        "build.rs",
        "a\\b.rs",
        "bad.txt",
    ] {
        let mut p = package.clone();
        p.files[0].path = path.into();
        assert!(p.digest().is_err());
    }
    let mut alias = package.clone();
    alias.files.push(SourceFile {
        path: "MODULE.rs".into(),
        text: String::new(),
    });
    assert!(alias.digest().is_err());
    package.files[0].text = "x".repeat(65537);
    assert!(package.digest().is_err());
    assert!(serde_json::from_value::<SourceQuery>(serde_json::json!({"package":"a","entry":"m.rs","expected_revision":0,"rustc":"/bin/sh"})).is_err());
}
#[test]
fn job_status_opaque_ids_actor_and_no_project_writes() {
    let f = Fixture::new();
    let old = std::fs::read_dir(f.root.path()).unwrap().count();
    let view = f.inspect();
    let result = f.build(&view.snapshot_id).unwrap();
    for id in [&view.snapshot_id, &result.build_id, &result.candidate_id] {
        assert!(ulid::Ulid::from_string(id).is_ok());
    }
    assert_eq!(result.provenance["actor"]["model"], "mock");
    assert_eq!(result.provenance["not_installed"], true);
    let status = f
        .service
        .status(
            &f.project,
            &JobQuery {
                snapshot_id: view.snapshot_id,
            },
        )
        .unwrap();
    assert_eq!(status.state, "complete");
    assert_eq!(
        status.candidate_id.as_deref(),
        Some(result.candidate_id.as_str())
    );
    assert_eq!(old, std::fs::read_dir(f.root.path()).unwrap().count());
}
#[test]
fn stale_foreign_cancelled_and_unavailable_requests_start_no_build() {
    let f = Fixture::new();
    let view = f.inspect();
    assert!(f.build("foreign").is_err());
    let mut stale = f.request(&view.snapshot_id);
    stale.expected_revision = 4;
    assert!(
        f.service
            .build(&f.project, stale, f.actor(), &f.cancel, &mut || Ok(()))
            .is_err()
    );
    assert!(
        f.service
            .build(
                "other",
                f.request(&view.snapshot_id),
                f.actor(),
                &f.cancel,
                &mut || Ok(())
            )
            .is_err()
    );
    f.cancel.store(true, Ordering::Relaxed);
    assert!(f.build(&view.snapshot_id).is_err());
    assert_eq!(f.backend.builds.load(Ordering::Relaxed), 0);
    assert!(
        Service::unavailable("missing trusted backend")
            .inspect(
                &f.project,
                SourceQuery {
                    package: "m".into(),
                    entry: "m.rs".into(),
                    expected_revision: 0
                },
                &AtomicBool::new(false),
                &mut || Ok(())
            )
            .is_err()
    );
}
#[test]
fn changed_sources_and_old_generation_never_publish_candidates() {
    let f = Fixture::new();
    let view = f.inspect();
    let backend = f.backend.clone();
    *f.backend.action.lock().unwrap() = Some(Box::new(move || {
        backend.source.lock().unwrap().files[0].text = "changed during build".into()
    }));
    assert!(matches!(f.build(&view.snapshot_id), Err(Error::Stale)));
    assert!(f.service.lock().unwrap().candidate.is_none());
    let service = f.service.clone();
    let project = f.project.clone();
    let root = f.root.path().to_owned();
    let view = f.inspect();
    *f.backend.action.lock().unwrap() =
        Some(Box::new(move || service.bind(&project, &root).unwrap()));
    assert!(f.build(&view.snapshot_id).is_err());
    assert!(
        f.service
            .status(
                &f.project,
                &JobQuery {
                    snapshot_id: view.snapshot_id
                }
            )
            .is_err()
    );
}
#[test]
fn busy_cancel_close_and_poison_require_owner_terminal() {
    let f = Fixture::new();
    let view = f.inspect();
    let service = f.service.clone();
    let project = f.project.clone();
    let id = view.snapshot_id.clone();
    *f.backend.action.lock().unwrap() = Some(Box::new(move || {
        assert!(matches!(service.begin(&project), Err(Error::Busy)));
        service.cancel_build(&project, &id).unwrap();
        assert!(service.lock().unwrap().active.is_some());
    }));
    assert!(matches!(f.build(&view.snapshot_id), Err(Error::Cancelled)));
    assert!(f.service.lock().unwrap().active.is_none());
    let view = f.inspect();
    let service = f.service.clone();
    *f.backend.action.lock().unwrap() = Some(Box::new(move || service.close()));
    assert!(f.build(&view.snapshot_id).is_err());
    assert!(!f.service.available());
    let g = Fixture::new();
    let view = g.inspect();
    g.backend.unclosed.store(true, Ordering::Relaxed);
    assert!(matches!(g.build(&view.snapshot_id), Err(Error::Cleanup(_))));
    assert!(!g.service.available());
    assert!(g.service.lock().unwrap().candidate.is_none());
    assert_eq!(
        g.service
            .status(
                &g.project,
                &JobQuery {
                    snapshot_id: view.snapshot_id
                }
            )
            .unwrap()
            .state,
        "cleanup_unproven"
    );
}
#[test]
fn aggregate_deadline_and_cancellation_cannot_hide_cleanup_failure() {
    let f = Fixture::new();
    let (lease, _) = f.service.begin(&f.project).unwrap();
    let old = Instant::now() - Duration::from_secs(181);
    assert!(matches!(
        lease.bounded(Ok(()), old, &f.cancel),
        Err(Error::Deadline)
    ));
    assert!(matches!(
        lease.bounded::<()>(Err(Error::Cleanup("owned".into())), old, &f.cancel),
        Err(Error::Cleanup(_))
    ));
    assert!(f.service.lock().unwrap().poisoned);
}
#[test]
fn new_failure_retires_previous_candidate_and_root_replacement_refuses() {
    let f = Fixture::new();
    let view = f.inspect();
    f.build(&view.snapshot_id).unwrap();
    assert!(f.service.lock().unwrap().candidate.is_some());
    f.backend.fail.store(true, Ordering::Relaxed);
    assert!(f.build(&view.snapshot_id).is_err());
    assert!(f.service.lock().unwrap().candidate.is_none());
    assert_eq!(
        f.service
            .status(
                &f.project,
                &JobQuery {
                    snapshot_id: view.snapshot_id
                }
            )
            .unwrap()
            .state,
        "failed"
    );
    let parent = tempfile::tempdir().unwrap();
    let old = parent.path().join("old");
    #[cfg(unix)]
    {
        std::fs::rename(f.root.path(), &old).unwrap();
        std::fs::create_dir(f.root.path()).unwrap();
        assert!(f.service.begin(&f.project).is_err());
    }
    #[cfg(windows)]
    {
        // Held no-DELETE-sharing root prevents replacement itself on Windows.
        let error = std::fs::rename(f.root.path(), &old).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(32));
        assert!(f.service.bound_to(&f.project, f.root.path()));
        assert!(f.service.begin(&f.project).is_ok());
    }
}
#[test]
fn exact_ordered_recipes_refuse_native_or_unknown_control() {
    let config = backend::Config {
        python: "/trusted/python".into(),
        worker: "/engine/tools/rust_guest_job.py".into(),
        rustc: "/toolchain/bin/rustc".into(),
        validator: "/engine/cook".into(),
        storage: "/jobs".into(),
    };
    let argv = vec!["/toolchain/bin/rustc".into(), "-vV".into()];
    assert!(crate::recipe::admit(&argv, &config, Path::new("/jobs/one"), None).is_ok());
    for args in [
        vec!["/bin/sh", "-c", "true"],
        vec!["/toolchain/bin/rustc", "evil.rs", "--crate-type=bin"],
        vec!["/toolchain/bin/rustc", "-vV", "--cfg=project"],
        vec![
            "/usr/bin/sandbox-exec",
            "-f",
            "/jobs/one/work/policy.sb",
            "/bin/sh",
            "-c",
            "true",
        ],
    ] {
        assert!(
            crate::recipe::admit(
                &args.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>(),
                &config,
                Path::new("/jobs/one"),
                None
            )
            .is_err()
        );
    }
}
