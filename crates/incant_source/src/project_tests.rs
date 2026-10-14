use super::*;
use std::{fs, path::Path, process::Command};
const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
const OP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
fn opened(root: &Path) -> (OpenedProject, SourceStore, File) {
    let (mut opened, journal) = OpenedProject::lock(&root.join("game.incant")).unwrap();
    let store = opened.source_store(ID).unwrap();
    let snapshot = opened.read().unwrap();
    if let Some(snapshot) = snapshot {
        opened.admit_snapshot(&snapshot);
    }
    (opened, store, journal)
}
fn edit() -> ProjectEdit {
    ProjectEdit {
        project_id: ID.into(),
        before: "before".into(),
        after: "after".into(),
    }
}
#[test]
fn project_crash_child() {
    let Some(root) = std::env::var_os("INCANT_PROJECT_SOURCE_ROOT") else {
        return;
    };
    let (opened, store, _journal) = opened(Path::new(&root));
    let snapshot = opened.read_admitted().unwrap();
    opened.begin(&store, OP, &edit(), &snapshot).unwrap();
    opened.displace(&store, OP, &edit()).unwrap();
    opened.publish(&store, OP, &edit()).unwrap();
    opened.finish(&store, OP, &edit(), true).unwrap();
    panic!("crash was not reached");
}
#[test]
fn project_exact_backup_and_missing_node_survive_real_process_exit() {
    for phase in ["backup", "marker", "displaced", "published", "finish"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("game.incant"), "before").unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "project::tests::project_crash_child",
                "--nocapture",
            ])
            .env("INCANT_PROJECT_SOURCE_ROOT", root)
            .env("INCANT_PROJECT_CRASH_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        assert_eq!(
            fs::read_to_string(root.join(STATE).join(format!("{OP}.project.backup"))).unwrap(),
            "before"
        );
        let (opened, store, _journal) = opened(root);
        if phase == "backup" {
            assert!(opened.pending(&store).unwrap().is_none());
            assert_eq!(opened.read().unwrap().unwrap().text, "before");
        } else if phase == "finish" {
            assert!(opened.pending(&store).unwrap().is_none());
            assert_eq!(opened.read().unwrap().unwrap().text, "after");
        } else {
            opened.rollback(&store, OP, &edit()).unwrap();
            opened.finish(&store, OP, &edit(), false).unwrap();
            assert_eq!(opened.read().unwrap().unwrap().text, "before");
        }
    }
}
#[test]
fn project_backup_collision_and_foreign_displaced_node_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("game.incant"), "before").unwrap();
    let (opened, store, _journal) = opened(root);
    let backup = root.join(STATE).join(format!("{OP}.project.backup"));
    fs::write(&backup, "foreign backup").unwrap();
    assert!(
        opened
            .begin(&store, OP, &edit(), &opened.read_admitted().unwrap())
            .is_err()
    );
    assert_eq!(fs::read_to_string(backup).unwrap(), "foreign backup");
    assert!(opened.pending(&store).unwrap().is_none());
    let other = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    opened
        .begin(&store, other, &edit(), &opened.read_admitted().unwrap())
        .unwrap();
    fs::write(root.join("game.incant"), "external in-place edit").unwrap();
    assert!(opened.displace(&store, other, &edit()).is_err());
    assert_eq!(
        fs::read_to_string(root.join("game.incant")).unwrap(),
        "external in-place edit"
    );
    assert!(opened.rollback(&store, other, &edit()).is_err());
}
#[test]
fn bounded_encoding_counts_escaped_bytes_and_names_are_not_data_authority() {
    assert!(bounded_json(&"\u{0000}".repeat(10), 20).is_err());
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("game.incant"), "before").unwrap();
    let (opened, store, _journal) = opened(root);
    assert!(
        opened
            .begin(
                &store,
                "../foreign",
                &edit(),
                &opened.read_admitted().unwrap()
            )
            .is_err()
    );
    assert!(opened.pending(&store).unwrap().is_none());
    assert!(!root.join("foreign").exists());
}

#[cfg(unix)]
#[test]
fn opened_and_reviewed_nodes_stay_alive_after_unlink_until_authority_retires() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("game.incant"), "before").unwrap();
    let (opened, _store, journal_writer) = opened(root);
    let snapshot = opened.read_admitted().unwrap();
    let original = snapshot.identity;
    fs::remove_file(root.join("game.incant")).unwrap();
    assert_eq!(snapshot.node.metadata().unwrap().nlink(), 0);
    assert_eq!(
        platform::identity(&opened.expected.as_ref().unwrap().node).unwrap(),
        original
    );
    fs::write(root.join("game.incant"), "before").unwrap();
    assert_ne!(opened.read().unwrap().unwrap().identity, original);
    assert!(opened.read_admitted().is_err());
    drop(snapshot);
    assert_eq!(
        opened
            .expected
            .as_ref()
            .unwrap()
            .node
            .metadata()
            .unwrap()
            .nlink(),
        0
    );
    // The capability also pins the journal node independently of its writer.
    drop(journal_writer);
    fs::remove_file(root.join("game.journal")).unwrap();
    assert_eq!(opened._journal_node.metadata().unwrap().nlink(), 0);
    fs::write(root.join("game.journal"), "").unwrap();
    assert!(opened.verify_journal().is_err());
}
