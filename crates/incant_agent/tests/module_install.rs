//! Real importer/journal/approval, synthesized compiler/backend. The separate
//! opt-in Local test is the only actual compiler-to-install claim in this suite.
#[path = "module_install/support.rs"]
mod support;
use incant_agent::{ApprovalMode, NoViewport, dispatch, tools};
use incant_cmd::{Actor, BinaryVersion, SourceVersion};
use incant_import::{CompiledWasmRequest, ImportRequest, ImportSnapshot, PreparedCompiledWasm};
use incant_module_build::InstallRequest;
use serde_json::json;
use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};
use support::*;

#[test]
fn installation_approval_is_one_durable_transaction_and_survives_private_candidate_removal() {
    for mode in [
        ApprovalMode::Always,
        ApprovalMode::Destructive,
        ApprovalMode::Auto,
    ] {
        for allow in [false, true] {
            let f = Fixture::new();
            let args = f.build();
            let len = f.journal_len();
            let mut approvals = 0;
            let report = f
                .run(&args, mode, &AtomicBool::new(false), &mut |name, a| {
                    assert!(f.bus.try_lock().is_ok());
                    assert_eq!(name, "module_install_registered");
                    assert_eq!(a, &args);
                    approvals += 1;
                    allow
                })
                .unwrap();
            let accepted = allow || matches!(mode, ApprovalMode::Auto);
            assert_eq!(approvals, usize::from(!matches!(mode, ApprovalMode::Auto)));
            assert_eq!(report.tool_calls[0].succeeded, accepted);
            if !accepted {
                f.unchanged(len);
                continue;
            }
            let (asset, raw) = {
                let mut bus = f.bus.lock().unwrap();
                assert_eq!(bus.revision(), 1);
                assert_eq!(bus.history().len(), 1);
                assert_eq!(report.transaction_ids, [bus.history()[0].id.clone()]);
                assert_eq!(
                    bus.history()[0].actor,
                    Actor::agent("mock-provider", "turn")
                );
                assert!(bus.history()[0].binary_edit.is_some());
                assert_eq!(bus.project().scenes, f.project.scenes);
                let asset = bus.project().assets.values().next().unwrap().clone();
                let ordinary = ImportSnapshot::capture(&bus)
                    .prepare(
                        f.root.path(),
                        None,
                        &[ImportRequest {
                            source: asset.path.clone(),
                            texture_usage: None,
                        }],
                    )
                    .unwrap();
                assert_eq!(ordinary.outcomes()[0].asset, asset);
                assert!(!ordinary.outcomes()[0].changed);
                let revision = bus.revision();
                bus.save_project_file(revision, Actor::user("save"))
                    .unwrap();
                let raw = fs::read(f.root.path().join(&asset.path)).unwrap();
                (asset, raw)
            };
            for entry in fs::read_dir(f.private.path()).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
            f.reopen();
            let mut bus = f.bus.lock().unwrap();
            bus.undo().unwrap();
            assert!(bus.project().assets.is_empty());
            assert!(!f.root.path().join(&asset.path).exists());
            bus.redo().unwrap();
            assert_eq!(bus.project().assets[&asset.id], asset);
            assert_eq!(fs::read(f.root.path().join(&asset.path)).unwrap(), raw);
        }
    }
}
#[test]
fn approval_fences_sibling_edit_registration_session_rebind_and_cancellation() {
    for case in [
        "sibling",
        "entry",
        "reopen",
        "rebind",
        "close",
        "cancel",
        "destination",
    ] {
        let f = Fixture::new();
        let args = f.build();
        let len = f.journal_len();
        let cancel = AtomicBool::new(false);
        let result = f.run(&args, ApprovalMode::Always, &cancel, &mut |_, _| {
            match case {
                "sibling" => {
                    fs::write(f.root.path().join("logic/helper.rs"), "pub const X:u8=2;").unwrap()
                }
                "entry" => fs::write(f.root.path().join("logic/main.rs"), NEXT).unwrap(),
                "reopen" => f.reopen(),
                "rebind" => f.jobs.bind(&f.project.id, f.root.path()).unwrap(),
                "close" => f.jobs.close(),
                "cancel" => cancel.store(true, Ordering::Release),
                _ => fs::write(
                    f.root.path().join("logic/main.wasm"),
                    b"external destination",
                )
                .unwrap(),
            }
            true
        });
        assert!(
            result.is_err() || !result.unwrap().tool_calls[0].succeeded,
            "{case}"
        );
        if case == "destination" {
            assert_eq!(
                fs::read(f.root.path().join("logic/main.wasm")).unwrap(),
                b"external destination"
            );
            fs::remove_file(f.root.path().join("logic/main.wasm")).unwrap();
        }
        f.unchanged(len);
    }
}
#[test]
fn installation_rejects_stale_ids_untrusted_fields_candidate_corruption_and_validator_mismatch() {
    for case in [
        "snapshot",
        "candidate",
        "revision",
        "module",
        "source",
        "destination",
        "path",
        "compiler",
        "raw_bytes",
        "corrupt",
        "prepared",
    ] {
        let f = Fixture::new();
        if case == "prepared" {
            f.fake.wrong_prepared.store(true, Ordering::Release);
        }
        let mut args = f.build();
        let len = f.journal_len();
        match case {
            "snapshot" => args["snapshot_id"] = json!(incant_doc::new_id()),
            "candidate" => args["candidate_id"] = json!(incant_doc::new_id()),
            "revision" => args["expected_revision"] = json!(1),
            "module" => args["module_id"] = json!(incant_doc::new_id()),
            "source" => args["expected_source"] = json!(SourceVersion::Missing),
            "destination" => {
                args["expected_destination"] = json!(BinaryVersion::of(Some(b"other")))
            }
            "path" | "compiler" | "raw_bytes" => args[case] = json!("untrusted"),
            "corrupt" => {
                let path = fs::read_dir(f.private.path())
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                fs::write(path, b"corrupt candidate").unwrap();
            }
            _ => {}
        }
        let result = f.run(
            &args,
            ApprovalMode::Always,
            &AtomicBool::new(false),
            &mut |_, _| true,
        );
        assert!(
            result.is_err() || !result.unwrap().tool_calls[0].succeeded,
            "{case}"
        );
        f.unchanged(len);
    }
    let tool = tools()
        .into_iter()
        .find(|t| t["name"] == "module_install_registered")
        .unwrap();
    assert_eq!(tool["parameters"]["additionalProperties"], false);
}
#[test]
fn package_revalidation_runs_without_bus_and_final_fence_rejects_sibling_after_prepare() {
    let f = Fixture::new();
    let args = f.build();
    let len = f.journal_len();
    let cancel = AtomicBool::new(false);
    let (authority, revision, destination) = f.authority();
    let installation = f
        .jobs
        .prepare_install(
            &f.project.id,
            InstallRequest {
                snapshot_id: args["snapshot_id"].as_str().unwrap().into(),
                candidate_id: args["candidate_id"].as_str().unwrap().into(),
                expected_revision: revision,
            },
            authority.clone(),
            &cancel,
            |root, bytes, _| {
                let bus = f.bus.try_lock().expect("no guard during prepare callback");
                PreparedCompiledWasm::prepare(
                    &bus,
                    root,
                    CompiledWasmRequest {
                        module_id: f.module.id.clone(),
                        expected_revision: revision,
                        expected_source: authority.version.clone(),
                        expected_destination: destination,
                    },
                    bytes,
                )
                .map_err(|e| incant_module_build::Error::Worker(e.to_string()))
            },
        )
        .unwrap();
    fs::write(f.root.path().join("logic/helper.rs"), "pub const X:u8=3;").unwrap();
    let mut bus = f.bus.lock().unwrap();
    assert!(
        installation
            .publish(&cancel, |prepared, _| prepared.commit(
                &mut bus,
                Actor::user("install"),
                "Install"
            ))
            .is_err()
    );
    drop(bus);
    f.unchanged(len);
}
#[test]
fn installed_output_does_not_contaminate_registered_rebuild_and_keeps_asset_identity() {
    let f = Fixture::new();
    let args = f.build();
    assert!(
        f.run(
            &args,
            ApprovalMode::Auto,
            &AtomicBool::new(false),
            &mut |_, _| panic!("auto")
        )
        .unwrap()
        .tool_calls[0]
            .succeeded
    );
    let first = f
        .bus
        .lock()
        .unwrap()
        .project()
        .assets
        .values()
        .next()
        .unwrap()
        .clone();
    let old = fs::read(f.root.path().join(&first.path)).unwrap();
    write(&f, NEXT);
    let next = f.build();
    assert!(
        f.run(
            &next,
            ApprovalMode::Auto,
            &AtomicBool::new(false),
            &mut |_, _| panic!("auto")
        )
        .unwrap()
        .tool_calls[0]
            .succeeded
    );
    let mut bus = f.bus.lock().unwrap();
    let second = bus.project().assets.values().next().unwrap().clone();
    assert_eq!(first.id, second.id);
    assert_ne!(first.sha256, second.sha256);
    bus.undo().unwrap();
    assert_eq!(fs::read(f.root.path().join(&first.path)).unwrap(), old);
    bus.redo().unwrap();
    assert_eq!(bus.project().assets[&first.id], second);
}
#[test]
fn source_read_remains_available_when_compiled_destination_is_invalid() {
    let f = Fixture::new();
    fs::write(
        f.root.path().join("logic/main.wasm"),
        vec![0; incant_source::MAX_WASM_BYTES + 1],
    )
    .unwrap();
    let value = dispatch(
        &mut f.bus.lock().unwrap(),
        &mut NoViewport,
        "module_read",
        json!({"module_id":f.module.id}),
        Actor::user("read"),
    )
    .unwrap();
    assert_eq!(value["untrusted_source_data"]["source"], TEXT);
    assert!(value["compiled_destination"].is_null());
    assert!(value["compiled_destination_error"].is_string());
}

#[cfg(all(target_os = "macos", feature = "host-qualification"))]
#[path = "module_install/actual.rs"]
mod actual;
