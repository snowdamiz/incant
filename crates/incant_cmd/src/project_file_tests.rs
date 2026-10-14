//! Actual files and fresh-process recovery. Fault hooks exist only in this unit
//! test executable and never in the engine/application build.
use super::*;
use base64::Engine as _;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command as Process,
};
const PROJECT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
fn initial() -> Project {
    let mut p = Project::empty("Legacy file");
    p.id = PROJECT.into();
    p.engine_version = None;
    p
}
fn path(root: &Path) -> PathBuf {
    root.join("game.incant")
}
fn setup(root: &Path, operation: &str) -> (u64, Vec<u8>) {
    // Deliberately noncanonical input; undo must restore every original byte.
    let source = format!("\n{}\n  ", serde_json::to_string(&initial()).unwrap());
    fs::write(path(root), source).unwrap();
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    if operation != "apply" {
        pin(&mut bus).unwrap();
    }
    if operation == "redo" {
        bus.undo().unwrap();
    }
    (bus.revision(), fs::read(path(root)).unwrap())
}
// Observe bytes through the handle that owns the Windows byte-range lock.
// Restoring its cursor keeps later journal operations unchanged.
fn journal_bytes(bus: &mut CommandBus) -> Vec<u8> {
    let file = bus.journal.as_mut().unwrap().test_file();
    let position = file.stream_position().unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    let result = file.read_to_end(&mut bytes);
    file.seek(SeekFrom::Start(position)).unwrap();
    result.unwrap();
    bytes
}
fn pin(bus: &mut CommandBus) -> Result<(), CommandError> {
    let review = bus.review_project_file_pin()?;
    bus.apply_project_file_pin(&review.review_id, Actor::agent("astra", "pin-test"))?;
    Ok(())
}
fn child(root: &Path, operation: &str) -> Process {
    let mut p = Process::new(std::env::current_exe().unwrap());
    p.args(["--exact", "project_file_tests::crash_child", "--nocapture"])
        .env("INCANT_PROJECT_TEST_ROOT", root)
        .env("INCANT_PROJECT_OPERATION", operation);
    p
}
pub(super) fn journal_failure(
    action: &journal::Action,
    file: &mut fs::File,
    bytes: &[u8],
) -> std::io::Result<()> {
    let phase = match action {
        journal::Action::ProjectPrepare { .. } => "prepare",
        journal::Action::ProjectCommit { .. } => "commit",
        _ => return Ok(()),
    };
    let crash = std::env::var("INCANT_PROJECT_PROTOCOL_CRASH").ok();
    let fail = std::env::var("INCANT_PROJECT_FAIL").ok();
    if crash.as_deref() == Some(&format!("partial_{phase}"))
        || fail.as_deref() == Some(phase)
        || fail.as_deref() == Some(&format!("poison_{phase}"))
    {
        file.write_all(&bytes[..bytes.len() / 2])?;
        file.sync_all()?;
        if crash.as_deref() == Some(&format!("partial_{phase}")) {
            std::process::exit(73);
        }
        let root = std::env::var_os("INCANT_PROJECT_TEST_ROOT").expect("test root");
        let mut read_only = fs::File::open(Path::new(&root).join("game.journal"))?;
        if fail.as_deref() == Some(&format!("poison_{phase}")) {
            *file = read_only;
            return Err(std::io::Error::other("injected ambiguous durability"));
        }
        return read_only.write_all(bytes);
    }
    Ok(())
}
#[test]
fn crash_child() {
    let Some(root) = std::env::var_os("INCANT_PROJECT_TEST_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    let revision = bus.revision();
    let before = bus.project().clone();
    let bytes = fs::read(path(root)).unwrap();
    let result = match std::env::var("INCANT_PROJECT_OPERATION").unwrap().as_str() {
        "apply" => pin(&mut bus),
        "undo" => bus.undo().map(|_| ()),
        "redo" => bus.redo().map(|_| ()),
        _ => panic!("operation"),
    };
    if let Ok(fail) = std::env::var("INCANT_PROJECT_FAIL") {
        assert!(result.is_err());
        assert_eq!(bus.revision(), revision);
        assert_eq!(bus.project(), &before);
        if fail.starts_with("poison_") {
            assert!(bus.ensure_sources_ready().is_err());
        } else {
            assert_eq!(fs::read(path(root)).unwrap(), bytes);
            bus.ensure_sources_ready().unwrap();
        }
        std::process::exit(74);
    }
    panic!("boundary was not reached: {result:?}");
}
#[test]
fn pin_undo_redo_preserve_exact_file_and_reopen_history() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_, original) = setup(root, "apply");
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    let first = bus.review_project_file_pin().unwrap();
    let fresh = bus.review_project_file_pin().unwrap();
    assert!(
        bus.apply_project_file_pin(&first.review_id, Actor::user("u"))
            .is_err()
    );
    bus.apply_project_file_pin(&fresh.review_id, Actor::agent("astra", "turn"))
        .unwrap();
    assert_eq!(
        fs::read_to_string(path(root)).unwrap(),
        fresh.candidate_text
    );
    assert!(bus.history()[0].source_edit.is_none());
    assert!(bus.history()[0].project_edit.is_some());
    assert_eq!(bus.history()[0].actor, Actor::agent("astra", "turn"));
    drop(bus);
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    assert_eq!(bus.revision(), 1);
    bus.undo().unwrap();
    assert_eq!(fs::read(path(root)).unwrap(), original);
    drop(bus);
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    assert!(bus.can_redo());
    bus.redo().unwrap();
    assert_eq!(
        fs::read_to_string(path(root)).unwrap(),
        fresh.candidate_text
    );
    let backups: Vec<_> = fs::read_dir(root.join(".incant-source-transactions"))
        .unwrap()
        .map(Result::unwrap)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".project.backup"))
        .map(|e| fs::read(e.path()).unwrap())
        .collect();
    assert_eq!(backups.len(), 3);
    assert_eq!(backups.iter().filter(|b| **b == original).count(), 2);
}
#[test]
fn stale_revision_bytes_and_same_byte_replacement_refuse_without_file_or_history_effects() {
    for change in ["revision", "bytes", "identity"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        setup(root, "apply");
        let mut bus = CommandBus::open_project_file(path(root)).unwrap();
        let review = bus.review_project_file_pin().unwrap();
        match change {
            "revision" => {
                bus.execute(
                    vec![Command::SetMemory {
                        section: "x".into(),
                        text: "new".into(),
                    }],
                    Actor::user("u"),
                    "edit",
                    Some(0),
                )
                .unwrap();
            }
            "bytes" => {
                fs::write(path(root), "foreign bytes").unwrap();
            }
            _ => {
                let old = fs::read(path(root)).unwrap();
                fs::rename(path(root), root.join("saved-foreign-node")).unwrap();
                fs::write(path(root), old).unwrap();
            }
        }
        let before = fs::read(path(root)).unwrap();
        let journal = journal_bytes(&mut bus);
        let revision = bus.revision();
        assert!(
            bus.apply_project_file_pin(&review.review_id, Actor::user("u"))
                .is_err()
        );
        assert_eq!(bus.revision(), revision);
        assert_eq!(fs::read(path(root)).unwrap(), before);
        assert_eq!(journal_bytes(&mut bus), journal);
        assert!(
            !root
                .join(".incant-source-transactions/project.pending")
                .exists()
        );
    }
}
#[test]
fn process_restart_at_every_project_boundary_reconciles_apply_undo_redo() {
    for operation in ["apply", "undo", "redo"] {
        for phase in [
            "marker",
            "partial_prepare",
            "prepare",
            "displaced",
            "installed",
            "partial_commit",
            "commit",
            "finish",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            let (revision, original) = setup(root, operation);
            let result = child(root, operation)
                .env("INCANT_PROJECT_PROTOCOL_CRASH", phase)
                .status()
                .unwrap();
            assert_eq!(result.code(), Some(73), "{operation}/{phase}");
            if phase == "displaced" {
                assert!(!path(root).exists());
            }
            let committed = matches!(phase, "commit" | "finish");
            let bus = CommandBus::open_project_file(path(root)).unwrap();
            assert_eq!(
                bus.revision(),
                revision + u64::from(committed),
                "{operation}/{phase}"
            );
            let expected_pin = if operation == "undo" {
                !committed
            } else {
                committed
            };
            assert_eq!(bus.project().engine_version.is_some(), expected_pin);
            assert_eq!(
                Project::from_text(&fs::read_to_string(path(root)).unwrap()).unwrap(),
                *bus.project()
            );
            if !committed {
                assert_eq!(fs::read(path(root)).unwrap(), original);
            }
            bus.ensure_sources_ready().unwrap();
        }
    }
}
#[test]
fn real_partial_write_error_rolls_back_and_ambiguous_durability_blocks_until_reopen() {
    for operation in ["apply", "undo", "redo"] {
        for phase in ["prepare", "commit", "poison_prepare", "poison_commit"] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            let (revision, original) = setup(root, operation);
            let status = child(root, operation)
                .env("INCANT_PROJECT_FAIL", phase)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(74), "{operation}/{phase}");
            // A known Commit failure emits Abort before this reopen. Its bytes
            // must be the durable checkpoint, even when import re-exports differ.
            if phase == "commit" {
                assert_abort_checkpoint(root);
            }
            let mut bus = CommandBus::open_project_file(path(root))
                .unwrap_or_else(|e| panic!("{operation}/{phase}: {e}"));
            if phase.ends_with("commit") {
                assert_abort_bytes(&journal_bytes(&mut bus));
            }
            assert_eq!(bus.revision(), revision);
            assert_eq!(fs::read(path(root)).unwrap(), original);
            bus.ensure_sources_ready().unwrap();
        }
    }
}
#[test]
fn absent_document_recovers_but_foreign_nodes_are_never_overwritten() {
    for phase in ["displaced", "installed", "commit"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        setup(root, "apply");
        assert_eq!(
            child(root, "apply")
                .env("INCANT_PROJECT_PROTOCOL_CRASH", phase)
                .status()
                .unwrap()
                .code(),
            Some(73)
        );
        if path(root).exists() {
            fs::rename(path(root), root.join("foreign-retired")).unwrap();
        }
        fs::write(path(root), "external replacement").unwrap();
        let journal = fs::read(root.join("game.journal")).unwrap();
        assert!(CommandBus::open_project_file(path(root)).is_err());
        assert_eq!(
            fs::read_to_string(path(root)).unwrap(),
            "external replacement"
        );
        assert_eq!(fs::read(root.join("game.journal")).unwrap(), journal);
        assert!(incant_source::check_recovery(root).is_err());
    }
}
#[test]
fn journal_and_marker_corruption_do_not_grant_recovery_authority() {
    for kind in [
        "future",
        "identity",
        "digest",
        "marker_version",
        "marker_path",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        setup(root, "apply");
        assert_eq!(
            child(root, "apply")
                .env("INCANT_PROJECT_PROTOCOL_CRASH", "prepare")
                .status()
                .unwrap()
                .code(),
            Some(73)
        );
        let journal_path = root.join("game.journal");
        let marker_path = root.join(".incant-source-transactions/project.pending");
        if kind.starts_with("marker") {
            let mut marker: serde_json::Value =
                serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
            if kind == "marker_version" {
                marker["version"] = 999.into();
            } else {
                marker["backup_path"] = "../foreign.json".into();
            }
            fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        } else {
            let text = fs::read_to_string(&journal_path).unwrap();
            let mut rows: Vec<serde_json::Value> = text
                .lines()
                .map(|s| serde_json::from_str(s).unwrap())
                .collect();
            if kind == "future" {
                rows[0]["version"] = 99.into();
            } else if kind == "identity" {
                rows[0]["action"]["project"]["id"] = "01ARZ3NDEKTSV4RRFFQ69G5FAV".into();
            } else {
                rows[1]["digest"] = "forged".into();
            }
            fs::write(
                &journal_path,
                rows.iter()
                    .map(|r| serde_json::to_string(r).unwrap() + "\n")
                    .collect::<String>(),
            )
            .unwrap();
        }
        let before = fs::read(path(root)).unwrap();
        let journal = fs::read(&journal_path).unwrap();
        let marker = fs::read(&marker_path).unwrap();
        assert!(CommandBus::open_project_file(path(root)).is_err());
        assert_eq!(fs::read(path(root)).unwrap(), before);
        assert_eq!(fs::read(journal_path).unwrap(), journal);
        assert_eq!(fs::read(marker_path).unwrap(), marker);
    }
}
#[test]
fn journal_limit_and_unavailable_capability_refuse_before_backup() {
    let mut memory = CommandBus::new(initial()).unwrap();
    assert!(memory.review_project_file_pin().is_err());
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    setup(root, "apply");
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    let review = bus.review_project_file_pin().unwrap();
    let file = bus.journal.as_mut().unwrap().test_file();
    let original_len = file.metadata().unwrap().len();
    // Use the lock-owning handle for this sparse capacity control; no 256 MiB
    // allocation/read or second Windows writer is required.
    file.set_len(journal::MAX_PROJECT_JOURNAL_BYTES as u64)
        .unwrap();
    assert!(
        bus.apply_project_file_pin(&review.review_id, Actor::user("u"))
            .is_err()
    );
    assert_eq!(bus.revision(), 0);
    assert!(
        !root
            .join(".incant-source-transactions/project.pending")
            .exists()
    );
    assert!(
        !fs::read_dir(root.join(".incant-source-transactions"))
            .unwrap()
            .map(Result::unwrap)
            .any(|e| e.file_name().to_string_lossy().ends_with(".backup"))
    );
    bus.journal
        .as_mut()
        .unwrap()
        .test_file()
        .set_len(original_len)
        .unwrap();
}
#[cfg(unix)]
#[test]
fn symlink_hardlink_and_journal_alias_refuse_without_touching_targets() {
    use std::os::unix::fs::symlink;
    for mode in [
        "project_symlink",
        "project_hardlink",
        "journal_symlink",
        "journal_hardlink",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let (_, original) = setup(root, "apply");
        let target = if mode.starts_with("project") {
            path(root)
        } else {
            root.join("game.journal")
        };
        fs::rename(&target, root.join("foreign")).unwrap();
        let foreign = fs::read(root.join("foreign")).unwrap();
        if mode.ends_with("symlink") {
            symlink(root.join("foreign"), &target).unwrap();
        } else {
            fs::hard_link(root.join("foreign"), &target).unwrap();
        }
        assert!(CommandBus::open_project_file(path(root)).is_err());
        assert_eq!(fs::read(root.join("foreign")).unwrap(), foreign);
        if mode.starts_with("journal") {
            assert_eq!(fs::read(path(root)).unwrap(), original);
        }
    }
}

#[test]
fn create_only_backup_failure_never_advances_project_history_or_journal() {
    let operation_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    for operation in ["apply", "undo", "redo"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let (revision, original) = setup(root, operation);
        let backup = root
            .join(".incant-source-transactions")
            .join(format!("{operation_id}.project.backup"));
        fs::write(&backup, "unrelated artifact").unwrap();
        let journal = fs::read(root.join("game.journal")).unwrap();
        assert_eq!(
            child(root, operation)
                .env("INCANT_PROJECT_FAIL", "backup")
                .env("INCANT_PROJECT_TEST_OPERATION_ID", operation_id)
                .status()
                .unwrap()
                .code(),
            Some(74)
        );
        assert_eq!(fs::read(&backup).unwrap(), b"unrelated artifact");
        assert_eq!(fs::read(path(root)).unwrap(), original);
        assert_eq!(fs::read(root.join("game.journal")).unwrap(), journal);
        let bus = CommandBus::open_project_file(path(root)).unwrap();
        assert_eq!(bus.revision(), revision);
    }
}
#[test]
fn project_transaction_cannot_smuggle_unrelated_edits_or_explicit_repins() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path(), "apply");
    let mut bus = CommandBus::open_project_file(path(dir.path())).unwrap();
    pin(&mut bus).unwrap();
    let tx = bus.history()[0].clone();
    super::project_file::validate_transaction(&tx).unwrap();
    let mut bad = tx.clone();
    bad.commands.push(Command::SetMemory {
        section: "x".into(),
        text: "x".into(),
    });
    assert!(super::project_file::validate_transaction(&bad).is_err());
    let mut bad = tx.clone();
    bad.after.name = "unrelated".into();
    bad.project_edit.as_mut().unwrap().after = bad.after.canonical_text().unwrap();
    assert!(super::project_file::validate_transaction(&bad).is_err());
    let mut bad = tx;
    bad.before.engine_version = Some(incant_doc::EngineRequirement::exact_current());
    bad.project_edit.as_mut().unwrap().before = bad.before.canonical_text().unwrap();
    assert!(super::project_file::validate_transaction(&bad).is_err());
}

fn artifact_inventory(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap().map(Result::unwrap) {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}
#[test]
fn unmatched_prepared_operation_never_retires_a_structurally_valid_foreign_marker() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    setup(root, "apply");
    assert_eq!(
        child(root, "apply")
            .env("INCANT_PROJECT_PROTOCOL_CRASH", "prepare")
            .status()
            .unwrap()
            .code(),
        Some(73)
    );
    let state = root.join(".incant-source-transactions");
    let marker = state.join("project.pending");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    let old = value["operation"].as_str().unwrap().to_owned();
    let foreign = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    for suffix in ["backup", "next"] {
        fs::rename(
            state.join(format!("{old}.project.{suffix}")),
            state.join(format!("{foreign}.project.{suffix}")),
        )
        .unwrap();
    }
    value["operation"] = foreign.into();
    fs::write(&marker, serde_json::to_vec(&value).unwrap()).unwrap();
    // Fixed names now match the foreign ULID and preserve node IDs/text. The
    // marker passes its filesystem validation; only the journal join rejects it.
    {
        let (mut opened, _journal) = incant_source::OpenedProject::lock(&path(root)).unwrap();
        let store = opened.source_store(PROJECT).unwrap();
        assert_eq!(opened.pending(&store).unwrap().unwrap().operation, foreign);
    }
    let before = artifact_inventory(root);
    assert!(CommandBus::open_project_file(path(root)).is_err());
    assert_eq!(artifact_inventory(root), before);
    assert!(CommandBus::open_project_file(path(root)).is_err());
    assert_eq!(artifact_inventory(root), before);
}
#[test]
fn incomplete_initial_journal_is_refused_without_truncation_or_backup() {
    for partial in [
        b"{".as_slice(),
        b"{\"version\":999",
        b"unrecognized bytes",
        b"\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        setup(root, "apply");
        fs::write(root.join("game.journal"), partial).unwrap();
        let before = artifact_inventory(root);
        assert!(CommandBus::open_project_file(path(root)).is_err());
        assert_eq!(artifact_inventory(root), before);
    }
}
#[test]
fn unlink_then_recreate_same_bytes_retires_a_review_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_, original) = setup(root, "apply");
    let before = artifact_inventory(root);
    let mut bus = CommandBus::open_project_file(path(root)).unwrap();
    let review = bus.review_project_file_pin().unwrap();
    fs::remove_file(path(root)).unwrap();
    fs::write(path(root), &original).unwrap();
    assert!(
        bus.apply_project_file_pin(&review.review_id, Actor::user("u"))
            .is_err()
    );
    assert_eq!(bus.revision(), 0);
    drop(bus);
    assert_eq!(artifact_inventory(root), before);
}

fn assert_abort_checkpoint(root: &Path) {
    assert_abort_bytes(&fs::read(root.join("game.journal")).unwrap());
}
fn assert_abort_bytes(bytes: &[u8]) {
    let entries: Vec<journal::Entry> = std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let [.., before, prepare, abort] = entries.as_slice() else {
        panic!("missing abort history")
    };
    assert!(matches!(
        prepare.action,
        journal::Action::ProjectPrepare { .. }
    ));
    assert!(matches!(abort.action, journal::Action::ProjectAbort { .. }));
    assert_eq!(abort.crdt, before.crdt);
}
#[test]
fn same_projection_different_causal_snapshot_cannot_replace_abort_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    setup(root, "apply");
    assert_eq!(
        child(root, "apply")
            .env("INCANT_PROJECT_FAIL", "commit")
            .status()
            .unwrap()
            .code(),
        Some(74)
    );
    assert_abort_checkpoint(root);
    let mut entries: Vec<journal::Entry> = fs::read_to_string(root.join("game.journal"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let fresh = CollaborativeDocument::new(&initial()).unwrap();
    let original = base64::engine::general_purpose::STANDARD
        .decode(&entries.last().unwrap().crdt)
        .unwrap();
    assert_eq!(
        CollaborativeDocument::from_export(&original)
            .unwrap()
            .project()
            .unwrap(),
        fresh.project().unwrap()
    );
    let last = entries.last_mut().unwrap();
    last.crdt = base64::engine::general_purpose::STANDARD.encode(fresh.export().unwrap());
    assert_ne!(
        base64::engine::general_purpose::STANDARD
            .decode(&last.crdt)
            .unwrap(),
        original
    );
    last.digest = journal::digest(
        last.version,
        last.sequence,
        &last.previous,
        &last.action,
        &last.crdt,
    )
    .unwrap();
    let forged = entries
        .iter()
        .map(|e| serde_json::to_string(e).unwrap() + "\n")
        .collect::<String>();
    fs::write(root.join("game.journal"), forged).unwrap();
    let before = artifact_inventory(root);
    assert!(CommandBus::open_project_file(path(root)).is_err());
    assert_eq!(artifact_inventory(root), before);
}
