//! Real process death; preparation and private copies never stand in for WAL durability.
use super::*;
use std::{fs, path::Path, process::Command as Process};
const MODULE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const ASSET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const TEXT: &str = "pub fn update() {}";
const RAW: &[u8] = b"\0asm\x01\0\0\0new";
const OLD: &[u8] = b"\0asm\x01\0\0\0old";
fn project() -> Project {
    let mut p = Project::empty("Binary recovery");
    p.id = "01ARZ3NDEKTSV4RRFFQ69G5FAX".into();
    p.module_sources.insert(
        MODULE.into(),
        RustModuleSource {
            id: MODULE.into(),
            name: "Game".into(),
            path: "modules/game/module.rs".into(),
        },
    );
    p.assets.insert(
        ASSET.into(),
        Asset {
            id: ASSET.into(),
            name: "Game".into(),
            path: "modules/game/module.wasm".into(),
            kind: "wasm".into(),
            sha256: "a".repeat(64),
            import_settings: None,
        },
    );
    p
}
fn open(root: &Path) -> CommandBus {
    CommandBus::persistent_with_sources(root.join("journal"), project(), root).unwrap()
}
fn apply(bus: &mut CommandBus) -> Result<(), CommandError> {
    let mut asset = project().assets[ASSET].clone();
    asset.sha256 = "b".repeat(64);
    let prepared = bus.prepare_wasm_asset(
        MODULE,
        SourceVersion::of(Some(TEXT)),
        BinaryVersion::of(Some(OLD)),
        asset,
        RAW,
    )?;
    bus.publish_wasm_asset(
        prepared,
        bus.revision(),
        Actor::agent("model", "binary"),
        "Replace compiled module",
    )?;
    Ok(())
}
#[test]
fn crash_child() {
    let Some(root) = std::env::var_os("INCANT_BINARY_CRASH_ROOT") else {
        return;
    };
    let mut bus = open(Path::new(&root));
    match std::env::var("INCANT_CMD_OPERATION").unwrap().as_str() {
        "apply" => apply(&mut bus).unwrap(),
        "undo" => {
            bus.undo().unwrap();
        }
        "redo" => {
            bus.redo().unwrap();
        }
        _ => panic!("operation"),
    }
    panic!("crash phase not reached");
}
#[test]
fn binary_replacement_apply_undo_redo_recover_every_durable_boundary() {
    if crate::test_isolation::isolated(
        "binary_protocol_tests::binary_replacement_apply_undo_redo_recover_every_durable_boundary",
    ) {
        return;
    }
    for operation in ["apply", "undo", "redo"] {
        for phase in [
            "marker",
            "partial_prepare",
            "prepare",
            "installed",
            "partial_commit",
            "commit",
            "finish",
        ] {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir_all(root.path().join("modules/game")).unwrap();
            fs::write(root.path().join("modules/game/module.rs"), TEXT).unwrap();
            fs::write(root.path().join("modules/game/module.wasm"), OLD).unwrap();
            let revision = {
                let mut bus = open(root.path());
                if operation != "apply" {
                    apply(&mut bus).unwrap();
                }
                if operation == "redo" {
                    bus.undo().unwrap();
                }
                bus.revision()
            };
            let output = Process::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "binary_protocol_tests::crash_child",
                    "--nocapture",
                ])
                .env("INCANT_BINARY_CRASH_ROOT", root.path())
                .env("INCANT_CMD_OPERATION", operation)
                .env("INCANT_CMD_CRASH_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(output.code(), Some(73), "{operation}/{phase}");
            let committed = matches!(phase, "commit" | "finish");
            let new = if operation == "undo" {
                !committed
            } else {
                committed
            };
            let mut bus = open(root.path());
            assert_eq!(
                bus.revision(),
                revision + u64::from(committed),
                "{operation}/{phase}"
            );
            assert_eq!(
                fs::read(root.path().join("modules/game/module.wasm")).unwrap(),
                if new { RAW } else { OLD },
                "{operation}/{phase}"
            );
            assert_eq!(
                bus.project().assets[ASSET].sha256,
                if new { "b".repeat(64) } else { "a".repeat(64) }
            );
            assert_eq!(
                fs::read(root.path().join("modules/game/module.rs")).unwrap(),
                TEXT.as_bytes()
            );
            if new {
                bus.undo().unwrap();
            } else if bus.can_redo() {
                bus.redo().unwrap();
            } else {
                apply(&mut bus).unwrap();
            }
            bus.ensure_sources_ready().unwrap();
        }
    }
}
#[test]
fn binary_journal_reserves_terminal_space_before_marker_or_publication() {
    if crate::test_isolation::isolated(
        "binary_protocol_tests::binary_journal_reserves_terminal_space_before_marker_or_publication",
    ) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("modules/game")).unwrap();
    fs::write(root.path().join("modules/game/module.rs"), TEXT).unwrap();
    fs::write(root.path().join("modules/game/module.wasm"), OLD).unwrap();
    let mut bus = open(root.path());
    let file = bus.journal.as_mut().unwrap().test_file();
    let original = file.metadata().unwrap().len();
    file.set_len(journal::MAX_PROJECT_JOURNAL_BYTES as u64 - 1)
        .unwrap();
    assert!(
        apply(&mut bus)
            .unwrap_err()
            .to_string()
            .contains("terminal record")
    );
    assert_eq!(bus.revision(), 0);
    assert_eq!(
        fs::read(root.path().join("modules/game/module.wasm")).unwrap(),
        OLD
    );
    assert!(
        !root
            .path()
            .join(".incant-source-transactions/binary.pending")
            .exists()
    );
    bus.journal
        .as_mut()
        .unwrap()
        .test_file()
        .set_len(original)
        .unwrap();
}
