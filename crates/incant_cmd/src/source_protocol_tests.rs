//! Failure controls exist only in this unit-test binary, never engine builds.
use super::*;
use std::{fs, io::Write, path::Path, process::Command as Process};
const SCRIPT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
pub(super) fn crash_point(phase: &str) {
    if std::env::var("INCANT_CMD_CRASH_PHASE").as_deref() == Ok(phase) {
        std::process::exit(73);
    }
}
pub(super) fn journal_failure(
    action: &journal::Action,
    file: &mut fs::File,
    bytes: &[u8],
) -> std::io::Result<()> {
    let phase = match action {
        journal::Action::Prepare { .. } => "prepare",
        journal::Action::Commit { .. } => "commit",
        _ => return Ok(()),
    };
    let crash = std::env::var("INCANT_CMD_CRASH_PHASE").ok();
    let fail = std::env::var("INCANT_CMD_FAIL_PHASE").ok();
    if crash.as_deref() == Some(&format!("partial_{phase}")) || fail.as_deref() == Some(phase) {
        file.write_all(&bytes[..bytes.len() / 2])?;
        file.sync_all()?;
        if fail.as_deref() == Some(phase) {
            // Produce an actual OS write error on a read-only journal handle
            // after a real partial write, then exercise production truncation.
            let root = std::env::var_os("INCANT_CMD_CRASH_ROOT").expect("test root");
            return fs::File::open(Path::new(&root).join("journal"))?.write_all(bytes);
        }
        std::process::exit(73);
    }
    Ok(())
}
fn project() -> Project {
    let mut project = Project::empty("Crash recovery");
    project.id = "01ARZ3NDEKTSV4RRFFQ69G5FAX".into();
    project.scripts.insert(
        SCRIPT.into(),
        ScriptSource {
            id: SCRIPT.into(),
            name: "Main".into(),
            path: "main.ts".into(),
        },
    );
    project
}
fn open(root: &Path) -> Result<CommandBus, CommandError> {
    CommandBus::persistent_with_sources(root.join("journal"), project(), root)
}
fn apply(bus: &mut CommandBus) -> Result<(), CommandError> {
    bus.execute(
        vec![
            Command::WriteScript {
                script_id: SCRIPT.into(),
                expected_source: SourceVersion::of(Some("before")),
                source: "after".into(),
            },
            Command::SetMemory {
                section: "source".into(),
                text: "authored".into(),
            },
        ],
        Actor::agent("model", "crash-turn"),
        "Write source",
        Some(bus.revision()),
    )?;
    Ok(())
}
#[test]
fn crash_child() {
    let Some(root) = std::env::var_os("INCANT_CMD_CRASH_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let mut bus = open(root).unwrap();
    let revision = bus.revision();
    let initial = bus.project().clone();
    let source = fs::read(root.join("main.ts")).unwrap();
    let operation = std::env::var("INCANT_CMD_OPERATION").unwrap();
    let result = match operation.as_str() {
        "apply" => apply(&mut bus),
        "undo" => bus.undo().map(|_| ()),
        "redo" => bus.redo().map(|_| ()),
        _ => panic!("invalid test operation"),
    };
    if std::env::var_os("INCANT_CMD_FAIL_PHASE").is_some() {
        assert!(matches!(result, Err(CommandError::Io(_))));
        assert_eq!(bus.project(), &initial);
        assert_eq!(bus.revision(), revision);
        assert_eq!(fs::read(root.join("main.ts")).unwrap(), source);
        bus.ensure_sources_ready().unwrap();
        std::process::exit(74);
    }
    panic!("crash point not reached: {result:?}");
}
fn setup(root: &Path, operation: &str) -> u64 {
    fs::write(root.join("main.ts"), "before").unwrap();
    let mut bus = open(root).unwrap();
    if operation != "apply" {
        apply(&mut bus).unwrap();
    }
    if operation == "redo" {
        bus.undo().unwrap();
    }
    bus.revision()
}
fn child(root: &Path, operation: &str) -> Process {
    let mut child = Process::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "source_protocol_tests::crash_child",
            "--nocapture",
        ])
        .env("INCANT_CMD_CRASH_ROOT", root)
        .env("INCANT_CMD_OPERATION", operation);
    child
}
#[test]
fn actual_process_death_at_every_journal_boundary_recovers_apply_undo_and_redo() {
    if crate::test_isolation::isolated(
        "source_protocol_tests::actual_process_death_at_every_journal_boundary_recovers_apply_undo_and_redo",
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
            let revision = setup(dir.path(), operation);
            let result = child(dir.path(), operation)
                .env("INCANT_CMD_CRASH_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(result.code(), Some(73), "{operation}/{phase}");
            let committed = matches!(phase, "commit" | "finish");
            let mut bus = open(dir.path()).unwrap();
            assert_eq!(
                bus.revision(),
                revision + u64::from(committed),
                "{operation}/{phase}"
            );
            let after = if operation == "undo" {
                !committed
            } else {
                committed
            };
            assert_eq!(
                fs::read_to_string(dir.path().join("main.ts")).unwrap(),
                if after { "after" } else { "before" },
                "{operation}/{phase}"
            );
            assert_eq!(bus.project().memory.contains_key("source"), after);
            assert_eq!(bus.history().len(), usize::from(after));
            if after {
                assert_eq!(bus.history()[0].actor, Actor::agent("model", "crash-turn"));
            }
            bus.ensure_sources_ready().unwrap();
            if after {
                bus.undo().unwrap();
            } else if bus.can_redo() {
                bus.redo().unwrap();
            } else {
                apply(&mut bus).unwrap();
            }
        }
    }
}
#[test]
fn actual_partial_write_errors_abort_without_document_or_disk_prefix_changes() {
    if crate::test_isolation::isolated(
        "source_protocol_tests::actual_partial_write_errors_abort_without_document_or_disk_prefix_changes",
    ) {
        return;
    }
    for operation in ["apply", "undo", "redo"] {
        for phase in ["prepare", "commit"] {
            let dir = tempfile::tempdir().unwrap();
            let revision = setup(dir.path(), operation);
            let original = fs::read(dir.path().join("main.ts")).unwrap();
            let status = child(dir.path(), operation)
                .env("INCANT_CMD_FAIL_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(74));
            let bus = open(dir.path()).unwrap();
            assert_eq!(bus.revision(), revision);
            assert_eq!(fs::read(dir.path().join("main.ts")).unwrap(), original);
            bus.ensure_sources_ready().unwrap();
        }
    }
}

#[test]
fn unresolved_file_effect_requires_its_bound_store_and_preserves_foreign_changes() {
    if crate::test_isolation::isolated(
        "source_protocol_tests::unresolved_file_effect_requires_its_bound_store_and_preserves_foreign_changes",
    ) {
        return;
    }
    for foreign in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        setup(dir.path(), "apply");
        assert_eq!(
            child(dir.path(), "apply")
                .env("INCANT_CMD_CRASH_PHASE", "installed")
                .status()
                .unwrap()
                .code(),
            Some(73)
        );
        assert!(matches!(
            CommandBus::persistent(dir.path().join("journal"), project()),
            Err(CommandError::Source(SourceError::RecoveryRequired))
        ));
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(matches!(
            CommandBus::persistent_with_sources(
                dir.path().join("journal"),
                project(),
                elsewhere.path()
            ),
            Err(CommandError::Source(SourceError::RecoveryRequired))
        ));
        assert!(!elsewhere.path().join("main.ts").exists());
        if foreign {
            fs::write(dir.path().join("main.ts"), "external change after crash").unwrap();
            let journal = fs::read(dir.path().join("journal")).unwrap();
            assert!(matches!(
                open(dir.path()),
                Err(CommandError::Source(SourceError::RecoveryRequired))
            ));
            assert_eq!(fs::read(dir.path().join("journal")).unwrap(), journal);
            assert_eq!(
                fs::read_to_string(dir.path().join("main.ts")).unwrap(),
                "external change after crash"
            );
            assert!(matches!(
                incant_source::check_recovery(dir.path()),
                Err(SourceError::RecoveryRequired)
            ));
        } else {
            let bus = open(dir.path()).unwrap();
            assert_eq!(bus.revision(), 0);
            assert_eq!(
                fs::read_to_string(dir.path().join("main.ts")).unwrap(),
                "before"
            );
        }
    }
}
