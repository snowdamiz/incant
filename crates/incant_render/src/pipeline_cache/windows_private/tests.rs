//! These controls execute Windows filesystem/ACL APIs; Mac compile/check is not
//! execution evidence. Fixtures are owned temporary leaves only.
use super::*;

#[test]
fn canonical_private_leaf_roundtrips_atomically_and_keeps_ancestors_pinned() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    let path = canonical.join("library");
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("current.library", b"one").unwrap();
    assert_eq!(directory.read("current.library", 3).unwrap(), b"one");
    directory
        .publish("current.library", b"replacement")
        .unwrap();
    assert_eq!(
        directory.read("current.library", 11).unwrap(),
        b"replacement"
    );
    assert!(directory.read("current.library", 10).is_err());
    assert!(std::fs::rename(&path, canonical.join("renamed")).is_err());
    assert!(!path.join("pending.library").exists());
    for name in ["", ".", "..", "../outside", "other:stream", "a\\b"] {
        assert!(directory.publish(name, b"x").is_err());
    }
    drop(directory);
    root.close().unwrap();
}

#[test]
fn hardlinks_unsafe_destination_and_existing_staging_never_replace_prior_bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("library");
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("current.library", b"old").unwrap();
    std::fs::hard_link(path.join("current.library"), root.path().join("alias")).unwrap();
    assert!(directory.read("current.library", 10).is_err());
    assert!(directory.publish("current.library", b"bad").is_err());
    assert_eq!(std::fs::read(path.join("current.library")).unwrap(), b"old");
    assert!(!path.join("pending.library").exists());
    std::fs::remove_file(root.path().join("alias")).unwrap();
    std::fs::write(path.join("pending.library"), b"another writer").unwrap();
    assert!(directory.publish("current.library", b"bad").is_err());
    assert_eq!(
        std::fs::read(path.join("pending.library")).unwrap(),
        b"another writer"
    );
    assert_eq!(directory.read("current.library", 10).unwrap(), b"old");
    drop(directory);
    root.close().unwrap();
}

#[test]
fn broad_unprotected_and_null_dacl_are_refused_without_repair() {
    let root = tempfile::tempdir().unwrap();
    for (name, sddl, protected) in [
        ("broad", "D:P(A;;FA;;;WD)", true),
        ("inherited", "D:(A;;FA;;;WD)", false),
        ("null", "D:NO_ACCESS_CONTROL", true),
    ] {
        let path = root.path().join(name);
        drop(Directory::open(&path, true).unwrap());
        security::replace_test_dacl(&path, sddl, protected).unwrap();
        assert!(Directory::open(&path, false).is_err(), "admitted {name}");
    }
}

#[test]
fn held_private_leaf_rechecks_new_file_acl_before_native_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("library");
    let directory = Directory::open(&path, true).unwrap();
    directory
        .publish("current.library", b"owned bytes")
        .unwrap();
    security::replace_test_dacl(&path.join("current.library"), "D:P(A;;FA;;;WD)", true).unwrap();
    assert!(directory.read("current.library", 100).is_err());
    assert!(directory.publish("current.library", b"bad").is_err());
    assert_eq!(
        std::fs::read(path.join("current.library")).unwrap(),
        b"owned bytes"
    );
}

#[test]
fn reparse_leaf_and_file_never_redirect_trusted_native_input() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("library");
    drop(Directory::open(&path, true).unwrap());
    let alias = root.path().join("alias");
    std::os::windows::fs::symlink_dir(&path, &alias)
        .expect("Windows reparse control requires symlink privilege or Developer Mode");
    assert!(Directory::open(&alias, false).is_err());
    let directory = Directory::open(&path, false).unwrap();
    directory.publish("owned.library", b"owned").unwrap();
    std::os::windows::fs::symlink_file(path.join("owned.library"), path.join("current.library"))
        .unwrap();
    assert!(directory.read("current.library", 100).is_err());
    assert!(directory.publish("current.library", b"bad").is_err());
    assert_eq!(directory.read("owned.library", 10).unwrap(), b"owned");
}
