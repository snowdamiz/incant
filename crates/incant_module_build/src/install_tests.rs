//! Synthesized backend/service controls; no compiler, sandbox or player claim.
use super::*;
use sha2::{Digest, Sha256};
use std::fs;
const TEXT: &str = "fn update() {}";
const RAW: &[u8] = b"\0asm\x01\0\0\0private mock bytes";
struct BackendFixture {
    private: PathBuf,
}
impl Backend for BackendFixture {
    fn inspect(
        &self,
        root: &Path,
        package: &str,
        entry: &str,
        _: &dyn Fn() -> bool,
    ) -> Result<SourcePackage> {
        Ok(SourcePackage {
            entry: entry.into(),
            files: [entry, "helper.rs"]
                .into_iter()
                .map(|path| {
                    Ok(SourceFile {
                        path: path.into(),
                        text: fs::read_to_string(root.join(package).join(path))?,
                    })
                })
                .collect::<Result<_>>()?,
        })
    }
    fn build(&self, _: &SourcePackage, _: &dyn Fn() -> bool) -> Result<backend::Built> {
        let candidate = self.private.join("module.wasm");
        fs::write(&candidate, RAW)?;
        Ok(backend::Built {
            candidate,
            diagnostics: "mock only".into(),
            receipt: serde_json::json!({
                "raw_sha256":format!("{:x}",Sha256::digest(RAW)),"raw_bytes":RAW.len(),
                "prepared_sha256":"a".repeat(64),"policy_sha256":"b".repeat(64),
                "host_tool_manifest_sha256":"c".repeat(64),"host_closeout_sha256":"d".repeat(64),
                "compiler_version":"mock","sdk_sources":{},"service_sources":{},"mock":true
            }),
        })
    }
}
struct Fixture {
    service: Service,
    registered: RegisteredSource,
    project: String,
    request: InstallRequest,
    root: tempfile::TempDir,
    private: tempfile::TempDir,
}
impl Fixture {
    fn new(registered: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("package")).unwrap();
        fs::write(root.path().join("package/module.rs"), TEXT).unwrap();
        fs::write(root.path().join("package/helper.rs"), "const X:u8=1;").unwrap();
        let service = Service::new(Arc::new(BackendFixture {
            private: private.path().canonicalize().unwrap(),
        }));
        let project = ulid::Ulid::new().to_string();
        service.bind(&project, root.path()).unwrap();
        let authority = RegisteredSource {
            bus_session_id: ulid::Ulid::new().to_string(),
            module_id: ulid::Ulid::new().to_string(),
            path: "package/module.rs".into(),
            version: incant_source::SourceVersion::of(Some(TEXT)),
        };
        let cancel = AtomicBool::new(false);
        let view = if registered {
            service.inspect_registered(&project, authority.clone(), 0, &cancel, &mut || Ok(()))
        } else {
            service.inspect(&project, authority.query(0).unwrap(), &cancel, &mut || {
                Ok(())
            })
        }
        .unwrap();
        let build = BuildRequest {
            snapshot_id: view.snapshot_id.clone(),
            expected_revision: 0,
            description: "mock".into(),
        };
        let actor = Actor {
            origin: "user".into(),
            actor: "fixture".into(),
            model: None,
            conversation_id: None,
        };
        let result = if registered {
            service.build_registered(
                &project,
                build,
                actor,
                authority.clone(),
                &cancel,
                &mut || Ok(()),
            )
        } else {
            service.build(&project, build, actor, &cancel, &mut || Ok(()))
        }
        .unwrap();
        Self {
            service,
            registered: authority,
            project,
            request: InstallRequest {
                snapshot_id: view.snapshot_id,
                candidate_id: result.candidate_id,
                expected_revision: 0,
            },
            root,
            private,
        }
    }
    fn prepare(&self, cancel: &AtomicBool) -> Result<Installation<'_, Vec<u8>>> {
        self.service.prepare_install(
            &self.project,
            self.request.clone(),
            self.registered.clone(),
            cancel,
            |_, bytes, _| Ok(bytes.to_vec()),
        )
    }
}
#[test]
fn install_requires_registered_complete_exact_ids_and_worker_identity() {
    for case in [
        "legacy",
        "snapshot",
        "candidate",
        "revision",
        "session",
        "bytes",
        "receipt",
        "oversize",
    ] {
        let mut f = Fixture::new(case != "legacy");
        match case {
            "snapshot" => f.request.snapshot_id = ulid::Ulid::new().to_string(),
            "candidate" => f.request.candidate_id = ulid::Ulid::new().to_string(),
            "revision" => f.request.expected_revision = 1,
            "session" => f.registered.bus_session_id = ulid::Ulid::new().to_string(),
            "bytes" => fs::write(
                f.private.path().join("module.wasm"),
                b"\0asm\x01\0\0\0altered",
            )
            .unwrap(),
            "oversize" => fs::write(
                f.private.path().join("module.wasm"),
                vec![0; incant_source::MAX_WASM_BYTES + 1],
            )
            .unwrap(),
            "receipt" => {
                f.service
                    .state
                    .lock()
                    .unwrap()
                    .candidate
                    .as_mut()
                    .unwrap()
                    .receipt["raw_bytes"] = serde_json::json!(1)
            }
            _ => {}
        }
        assert!(f.prepare(&AtomicBool::new(false)).is_err(), "{case}");
        assert!(!f.root.path().join("package/module.wasm").exists());
    }
}
#[test]
fn install_recaptures_siblings_before_and_after_import_preparation() {
    for after in [false, true] {
        let f = Fixture::new(true);
        let change =
            || fs::write(f.root.path().join("package/helper.rs"), "const X:u8=2;").unwrap();
        if !after {
            change();
        }
        let result = f.service.prepare_install(
            &f.project,
            f.request.clone(),
            f.registered.clone(),
            &AtomicBool::new(false),
            |_, _, _| {
                assert!(
                    f.service.state.try_lock().is_ok(),
                    "preparation must not hold service state"
                );
                if after {
                    change();
                }
                Ok(())
            },
        );
        assert!(result.is_err());
    }
}
#[test]
fn prepared_payload_cannot_publish_after_close_rebind_cancellation_or_candidate_change() {
    for case in ["close", "rebind", "cancel", "candidate", "epoch"] {
        let f = Fixture::new(true);
        let cancel = AtomicBool::new(false);
        let lease = f.prepare(&cancel).unwrap();
        match case {
            "close" => f.service.close(),
            "rebind" => f.service.bind(&f.project, f.root.path()).unwrap(),
            "cancel" => cancel.store(true, Ordering::Release),
            "candidate" => fs::write(
                f.private.path().join("module.wasm"),
                b"\0asm\x01\0\0\0different",
            )
            .unwrap(),
            "epoch" => {
                fs::create_dir(f.root.path().join(".incant-source-transactions")).unwrap();
                fs::write(
                    f.root.path().join(".incant-source-transactions/receipt"),
                    ulid::Ulid::new().to_string(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let mut called = false;
        let result = lease.publish(&cancel, |_, _| {
            called = true;
            Ok::<_, ()>(())
        });
        assert!(result.is_err(), "{case}");
        assert!(!called, "{case}");
    }
}
#[test]
fn publication_holds_service_generation_and_finishes_after_cancellation_boundary() {
    let f = Fixture::new(true);
    let cancel = AtomicBool::new(false);
    let lease = f.prepare(&cancel).unwrap();
    let result = lease
        .publish(&cancel, |bytes, receipt| {
            assert!(f.service.state.try_lock().is_err());
            assert_eq!(bytes, RAW);
            assert_eq!(receipt.candidate_id, f.request.candidate_id);
            cancel.store(true, Ordering::Release);
            Ok::<_, ()>(42)
        })
        .unwrap()
        .unwrap();
    assert_eq!(result, 42);
    assert!(f.service.state.lock().unwrap().active.is_none());
    fs::remove_file(f.private.path().join("module.wasm")).unwrap();
}

#[test]
fn final_publication_refuses_direct_sibling_edit_after_preparation() {
    let f = Fixture::new(true);
    let cancel = AtomicBool::new(false);
    let installation = f.prepare(&cancel).unwrap();
    fs::write(f.root.path().join("package/helper.rs"), "const X:u8=3;").unwrap();
    let result = installation.publish(&cancel, |_, _| -> std::result::Result<(), ()> {
        panic!("no publication callback after sibling edit")
    });
    assert!(result.is_err());
    assert!(!f.root.path().join("package/module.wasm").exists());
    assert!(f.service.state.lock().unwrap().active.is_none());
}
