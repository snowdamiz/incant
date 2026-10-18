//! Real process-death controls reuse the existing source-journal fault points.
use super::*;
use std::{fs, path::Path, process::Command as Process};
const MODULE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const PATH: &str = "movement/main.rs";
const BEFORE: &str = "pub fn update() -> u32 { 1 }";
const AFTER: &str = "pub fn update() -> u32 { 2 }";
fn project() -> Project {
    let mut project = Project::empty("Rust recovery");
    project.id = "01ARZ3NDEKTSV4RRFFQ69G5FAX".into();
    project.module_sources.insert(
        MODULE.into(),
        RustModuleSource {
            id: MODULE.into(),
            name: "Movement".into(),
            path: PATH.into(),
        },
    );
    project
}
fn open(root: &Path) -> CommandBus {
    CommandBus::persistent_with_sources(root.join("journal"), project(), root).unwrap()
}
fn apply(bus: &mut CommandBus) -> Result<(), CommandError> {
    bus.execute(
        vec![
            Command::WriteModuleSource {
                module_id: MODULE.into(),
                expected_source: SourceVersion::of(Some(BEFORE)),
                source: AFTER.into(),
            },
            Command::SetMemory {
                section: "module".into(),
                text: "authored".into(),
            },
        ],
        Actor::agent("model", "rust-recovery"),
        "Rust write",
        Some(bus.revision()),
    )?;
    Ok(())
}
#[test]
fn crash_child() {
    let Some(root) = std::env::var_os("INCANT_CMD_RUST_CRASH_ROOT") else {
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
        _ => panic!("invalid operation"),
    }
    panic!("crash point was not reached");
}
#[test]
fn rust_apply_undo_and_redo_recover_every_durable_boundary() {
    if crate::test_isolation::isolated(
        "module_source_protocol_tests::rust_apply_undo_and_redo_recover_every_durable_boundary",
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
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir(dir.path().join("movement")).unwrap();
            fs::write(dir.path().join(PATH), BEFORE).unwrap();
            let revision = {
                let mut bus = open(dir.path());
                if operation != "apply" {
                    apply(&mut bus).unwrap();
                }
                if operation == "redo" {
                    bus.undo().unwrap();
                }
                bus.revision()
            };
            let status = Process::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "module_source_protocol_tests::crash_child",
                    "--nocapture",
                ])
                .env("INCANT_CMD_RUST_CRASH_ROOT", dir.path())
                .env("INCANT_CMD_OPERATION", operation)
                .env("INCANT_CMD_CRASH_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(73), "{operation}/{phase}");
            let committed = matches!(phase, "commit" | "finish");
            let after = if operation == "undo" {
                !committed
            } else {
                committed
            };
            let mut bus = open(dir.path());
            assert_eq!(
                bus.revision(),
                revision + u64::from(committed),
                "{operation}/{phase}"
            );
            assert_eq!(
                bus.read_module_source(MODULE).unwrap().source.as_deref(),
                Some(if after { AFTER } else { BEFORE }),
                "{operation}/{phase}"
            );
            assert_eq!(bus.project().memory.contains_key("module"), after);
            assert_eq!(bus.history().len(), usize::from(after));
            if after {
                assert_eq!(
                    bus.history()[0].source_edit.as_ref().unwrap().kind,
                    incant_source::SourceKind::Rust
                );
                assert_eq!(
                    bus.history()[0].actor,
                    Actor::agent("model", "rust-recovery")
                );
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
