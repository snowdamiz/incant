use crate::{SourceEdit, SourceError, SourceStore, check_recovery, validate_path};
use std::{fs, path::Path, process::Command};
const SCRIPT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
fn store(root: &Path) -> SourceStore {
    let journal = root.join("journal");
    if !journal.exists() {
        fs::write(&journal, "").unwrap();
    }
    SourceStore::open(root, &journal, SCRIPT).unwrap()
}
fn edit(before: Option<&str>, after: Option<&str>) -> SourceEdit {
    SourceEdit {
        script_id: SCRIPT.into(),
        path: "main.ts".into(),
        before: before.map(str::to_owned),
        after: after.map(str::to_owned),
    }
}
#[test]
fn read_only_guard_creates_nothing_and_epoch_detects_a_completed_write() {
    let dir = tempfile::tempdir().unwrap();
    let first = check_recovery(dir.path()).unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    let store = store(dir.path());
    let change = edit(None, Some("export const answer = 42;"));
    store.begin(OP, &change).unwrap();
    assert!(matches!(
        check_recovery(dir.path()),
        Err(SourceError::RecoveryRequired)
    ));
    store.install(OP, &change).unwrap();
    store.finish(OP).unwrap();
    assert_ne!(first, check_recovery(dir.path()).unwrap());
    assert_eq!(store.read("main.ts").unwrap(), change.after);
}
#[test]
fn replacement_retains_actual_old_file_and_rollback_restores_it() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.ts"), "before").unwrap();
    let store = store(dir.path());
    let change = edit(Some("before"), Some("after"));
    store.begin(OP, &change).unwrap();
    store.install(OP, &change).unwrap();
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(format!(".incant-source-transactions/{OP}.previous"))
        )
        .unwrap(),
        "before"
    );
    store.rollback(OP, &change).unwrap();
    store.finish(OP).unwrap();
    assert_eq!(store.read("main.ts").unwrap(), change.before);
}
#[test]
fn external_race_is_retained_and_blocks_recovery_instead_of_clobbering() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.ts"), "before").unwrap();
    let store = store(dir.path());
    let change = edit(Some("before"), Some("after"));
    store.begin(OP, &change).unwrap();
    fs::write(dir.path().join("main.ts"), "external").unwrap();
    assert!(matches!(
        store.install(OP, &change),
        Err(SourceError::Conflict)
    ));
    assert!(matches!(
        store.rollback(OP, &change),
        Err(SourceError::RecoveryRequired)
    ));
    assert!(matches!(
        check_recovery(dir.path()),
        Err(SourceError::RecoveryRequired)
    ));
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(format!(".incant-source-transactions/{OP}.previous"))
        )
        .unwrap(),
        "external"
    );
}
#[test]
fn missing_source_race_never_clobbers_the_new_external_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    let change = edit(None, Some("ours"));
    store.begin(OP, &change).unwrap();
    fs::write(dir.path().join("main.ts"), "external").unwrap();
    assert!(store.install(OP, &change).is_err());
    assert_eq!(store.read("main.ts").unwrap().as_deref(), Some("external"));
    assert!(store.rollback(OP, &change).is_err());
}
#[test]
fn byte_limit_paths_links_and_missing_parent_refuse_without_effects() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    for path in [
        "../out.ts",
        "/out.ts",
        "a//b.ts",
        "a\\b.ts",
        "CON.ts",
        "nul/a.ts",
        "a.ts:other",
        "a.d.ts",
        ".incant-source-transactions/a.ts",
        "a./b.ts",
        "a /b.ts",
    ] {
        assert!(validate_path(path).is_err(), "{path}");
    }
    assert!(store.read("missing/a.ts").is_err());
    let mut change = edit(None, Some("ok"));
    change.after = Some("x".repeat(crate::MAX_SOURCE_BYTES + 1));
    assert!(store.begin(OP, &change).is_err());
    fs::write(dir.path().join("main.ts"), "linked").unwrap();
    fs::hard_link(dir.path().join("main.ts"), dir.path().join("alias.ts")).unwrap();
    assert!(store.read("main.ts").is_err());
    assert!(store.pending().unwrap().is_none());
}
#[test]
fn root_lock_is_exclusive_across_different_journals() {
    let dir = tempfile::tempdir().unwrap();
    let first = store(dir.path());
    fs::write(dir.path().join("other-journal"), "").unwrap();
    assert!(SourceStore::open(dir.path(), &dir.path().join("other-journal"), SCRIPT).is_err());
    drop(first);
    assert!(SourceStore::open(dir.path(), &dir.path().join("other-journal"), SCRIPT).is_ok());
}
#[test]
fn process_crash_child() {
    let Some(root) = std::env::var_os("INCANT_SOURCE_CRASH_TEST_ROOT") else {
        return;
    };
    let phase = std::env::var("INCANT_SOURCE_CRASH_TEST_PHASE").unwrap();
    let root = Path::new(&root);
    let store = store(root);
    let change = edit(Some("before"), Some("after"));
    store.begin(OP, &change).unwrap();
    if phase != "prepared" {
        store.install(OP, &change).unwrap();
    }
    if matches!(
        phase.as_str(),
        "receipt_retired" | "receipt_published" | "finished"
    ) {
        store.finish(OP).unwrap();
    }
    // A real process exits without Rust destructors, releasing only OS handles.
    std::process::exit(73);
}
#[test]
fn actual_process_crashes_recover_prepared_and_installed_files() {
    for phase in [
        "staged",
        "prepared",
        "displaced",
        "installed",
        "receipt_retired",
        "receipt_published",
        "finished",
    ] {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.ts"), "before").unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::process_crash_child", "--nocapture"])
            .env("INCANT_SOURCE_CRASH_TEST_ROOT", dir.path())
            .env("INCANT_SOURCE_CRASH_TEST_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(73));
        if matches!(phase, "staged" | "finished") {
            check_recovery(dir.path()).unwrap();
        } else {
            assert!(matches!(
                check_recovery(dir.path()),
                Err(SourceError::RecoveryRequired)
            ));
        }
        let store = store(dir.path());
        if phase == "staged" {
            assert_eq!(store.read("main.ts").unwrap().as_deref(), Some("before"));
            continue;
        }
        if phase == "finished" {
            assert_eq!(store.read("main.ts").unwrap().as_deref(), Some("after"));
            continue;
        }
        let pending = store.pending().unwrap().unwrap();
        if matches!(phase, "receipt_retired" | "receipt_published") {
            store.finish(&pending.operation).unwrap();
            assert_eq!(store.read("main.ts").unwrap().as_deref(), Some("after"));
            continue;
        }
        store.rollback(&pending.operation, &pending.edit).unwrap();
        store.finish(&pending.operation).unwrap();
        assert_eq!(store.read("main.ts").unwrap().as_deref(), Some("before"));
        check_recovery(dir.path()).unwrap();
    }
}
#[cfg(unix)]
#[test]
fn symlink_files_directories_and_state_are_not_followed() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("main.ts"), "outside").unwrap();
    symlink(outside.path().join("main.ts"), dir.path().join("main.ts")).unwrap();
    symlink(outside.path(), dir.path().join("scripts")).unwrap();
    let store = store(dir.path());
    assert!(store.read("main.ts").is_err());
    assert!(store.read("scripts/main.ts").is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("main.ts")).unwrap(),
        "outside"
    );
    drop(store);
    fs::remove_dir_all(dir.path().join(".incant-source-transactions")).unwrap();
    symlink(
        outside.path(),
        dir.path().join(".incant-source-transactions"),
    )
    .unwrap();
    assert!(matches!(
        check_recovery(dir.path()),
        Err(SourceError::RecoveryRequired)
    ));
}
#[cfg(windows)]
#[test]
fn junction_directory_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("main.ts"), "outside").unwrap();
    // Junction creation requires no symlink privilege on Windows CI.
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dir.path().join("scripts"))
        .arg(outside.path())
        .status()
        .unwrap();
    assert!(status.success());
    let store = store(dir.path());
    assert!(store.read("scripts/main.ts").is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("main.ts")).unwrap(),
        "outside"
    );
}

#[cfg(unix)]
#[test]
fn raced_leaf_symlink_is_preserved_without_following_its_target() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.ts"), "before").unwrap();
    fs::write(outside.path().join("other.ts"), "outside").unwrap();
    let store = store(dir.path());
    let change = edit(Some("before"), Some("ours"));
    store.begin(OP, &change).unwrap();
    fs::remove_file(dir.path().join("main.ts")).unwrap();
    symlink(outside.path().join("other.ts"), dir.path().join("main.ts")).unwrap();
    assert!(store.install(OP, &change).is_err());
    assert!(matches!(
        store.rollback(OP, &change),
        Err(SourceError::RecoveryRequired)
    ));
    assert_eq!(
        fs::read_to_string(outside.path().join("other.ts")).unwrap(),
        "outside"
    );
    assert!(
        fs::symlink_metadata(
            dir.path()
                .join(format!(".incant-source-transactions/{OP}.previous"))
        )
        .unwrap()
        .is_symlink()
    );
}
#[cfg(windows)]
#[test]
fn pinned_windows_directory_cannot_be_renamed_during_source_access() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("scripts")).unwrap();
    let root = super::platform::Dir::root(dir.path()).unwrap();
    let pinned = root.child("scripts").unwrap();
    assert!(fs::rename(dir.path().join("scripts"), dir.path().join("moved")).is_err());
    drop(pinned);
    fs::rename(dir.path().join("scripts"), dir.path().join("moved")).unwrap();
}
