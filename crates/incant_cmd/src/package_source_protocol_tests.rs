//! Actual process death at each journal boundary for a newly owned subtree.
use super::*;
use std::{fs, path::Path, process::Command as Process};
const MODULE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const PATH: &str = "modules/movement/module.rs";
const TEXT: &str = "pub fn update() {}";

fn project() -> Project {
    let mut project = Project::empty("Package recovery");
    project.id = "01ARZ3NDEKTSV4RRFFQ69G5FAX".into();
    project
}
fn open(root: &Path) -> CommandBus {
    CommandBus::persistent_with_sources(root.join("journal"), project(), root).unwrap()
}
fn create(bus: &mut CommandBus) {
    bus.execute(
        vec![
            Command::UpsertModuleSource {
                module: RustModuleSource {
                    id: MODULE.into(),
                    name: "Movement".into(),
                    path: PATH.into(),
                },
            },
            Command::WriteModuleSource {
                module_id: MODULE.into(),
                expected_source: SourceVersion::Missing,
                source: TEXT.into(),
            },
            Command::SetMemory {
                section: "package".into(),
                text: "created".into(),
            },
        ],
        Actor::agent("model", "package-recovery"),
        "Create first package",
        Some(bus.revision()),
    )
    .unwrap();
}
#[test]
fn crash_child() {
    let Some(root) = std::env::var_os("INCANT_CMD_PACKAGE_CRASH_ROOT") else {
        return;
    };
    let mut bus = open(Path::new(&root));
    match std::env::var("INCANT_CMD_OPERATION").unwrap().as_str() {
        "apply" => create(&mut bus),
        "undo" => {
            bus.undo().unwrap();
        }
        "redo" => {
            bus.redo().unwrap();
        }
        _ => panic!("unknown package operation"),
    }
    panic!("crash point was not reached");
}
#[test]
fn package_apply_undo_redo_recover_every_durable_boundary_without_partial_directories() {
    if crate::test_isolation::isolated(
        "package_source_protocol_tests::package_apply_undo_redo_recover_every_durable_boundary_without_partial_directories",
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
            let revision = {
                let mut bus = open(root.path());
                if operation != "apply" {
                    create(&mut bus);
                }
                if operation == "redo" {
                    bus.undo().unwrap();
                }
                bus.revision()
            };
            let result = Process::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "package_source_protocol_tests::crash_child",
                    "--nocapture",
                ])
                .env("INCANT_CMD_PACKAGE_CRASH_ROOT", root.path())
                .env("INCANT_CMD_OPERATION", operation)
                .env("INCANT_CMD_CRASH_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(result.code(), Some(73), "{operation}/{phase}");
            let committed = matches!(phase, "commit" | "finish");
            let present = if operation == "undo" {
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
            assert_eq!(bus.project().module_sources.contains_key(MODULE), present);
            assert_eq!(bus.project().memory.contains_key("package"), present);
            assert_eq!(
                root.path().join("modules").exists(),
                present,
                "{operation}/{phase}"
            );
            if present {
                assert_eq!(fs::read(root.path().join(PATH)).unwrap(), TEXT.as_bytes());
                assert!(
                    bus.history()[0]
                        .source_edit
                        .as_ref()
                        .unwrap()
                        .created_tree
                        .is_some()
                );
                assert_eq!(
                    bus.history()[0].actor,
                    Actor::agent("model", "package-recovery")
                );
                bus.undo().unwrap();
            } else if bus.can_redo() {
                bus.redo().unwrap();
            } else {
                create(&mut bus);
            }
            bus.ensure_sources_ready().unwrap();
        }
    }
}
