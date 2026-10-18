use super::*;
use std::fs;
const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const UNDO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
const SOURCE: &str = "modules/game/module.rs";
const DEST: &str = "modules/game/module.wasm";
const RAW: &[u8] = b"\0asm\x01\0\0\0candidate";
fn open(root: &Path) -> SourceStore {
    fs::create_dir_all(root.join("modules/game")).unwrap();
    if !root.join(SOURCE).exists() {
        fs::write(root.join(SOURCE), "source").unwrap();
    }
    let journal = root.join("journal");
    if !journal.exists() {
        fs::write(&journal, "").unwrap();
    }
    SourceStore::open(root, &journal, ID).unwrap()
}
fn prepare(store: &SourceStore, expected: BinaryVersion) -> BinaryEdit {
    let snapshot = store.capture_binary(SOURCE, &expected).unwrap();
    store
        .prepare_binary(
            OP,
            BinaryPublication {
                module_id: ID,
                asset_id: UNDO,
                source_path: SOURCE,
                source_sha256: &format!("{:x}", Sha256::digest(b"source")),
                snapshot: &snapshot,
                bytes: RAW,
            },
        )
        .unwrap()
}
#[test]
fn binary_nodes_survive_reopen_and_repeated_reversible_publication() {
    let root = tempfile::tempdir().unwrap();
    let edit = {
        let store = open(root.path());
        let edit = prepare(&store, BinaryVersion::Missing);
        assert!(!root.path().join(DEST).exists());
        store.begin_binary(OP, &edit).unwrap();
        assert!(check_recovery(root.path()).is_err());
        assert!(!root.path().join(DEST).exists());
        store.install_binary(OP, &edit).unwrap();
        store.finish_binary(OP).unwrap();
        assert_eq!(store.read_binary(SOURCE).unwrap().unwrap(), RAW);
        edit
    };
    let store = open(root.path());
    store.begin_binary(UNDO, &edit.reversed()).unwrap();
    store.install_binary(UNDO, &edit.reversed()).unwrap();
    store.finish_binary(UNDO).unwrap();
    assert!(!root.path().join(DEST).exists());
    let redo = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
    store.begin_binary(redo, &edit).unwrap();
    store.install_binary(redo, &edit).unwrap();
    store.finish_binary(redo).unwrap();
    assert_eq!(store.read_binary(SOURCE).unwrap().unwrap(), RAW);
}
#[test]
fn mixed_pending_protocols_refuse_before_binary_recovery_moves_bytes() {
    for name in ["pending", "project.pending"] {
        let root = tempfile::tempdir().unwrap();
        let store = open(root.path());
        let edit = prepare(&store, BinaryVersion::Missing);
        store.begin_binary(OP, &edit).unwrap();
        let marker = root.path().join(STATE).join(name);
        fs::write(&marker, b"foreign pending protocol").unwrap();
        assert!(store.binary_pending().is_err());
        assert!(store.install_binary(OP, &edit).is_err());
        assert!(store.rollback_binary(OP, &edit).is_err());
        assert!(store.finish_binary(OP).is_err());
        assert!(!root.path().join(DEST).exists());
        assert_eq!(fs::read(&marker).unwrap(), b"foreign pending protocol");
        assert_eq!(
            fs::read(root.path().join(STATE).join(format!("{OP}.wasm-after"))).unwrap(),
            RAW
        );
        fs::remove_file(marker).unwrap();
        store.rollback_binary(OP, &edit).unwrap();
        store.finish_binary(OP).unwrap();
    }
}
#[test]
fn binary_replacement_retains_old_node_and_rollback_never_clobbers_racing_content() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    fs::write(root.path().join(DEST), b"old arbitrary imported bytes").unwrap();
    let edit = prepare(
        &store,
        BinaryVersion::of(Some(b"old arbitrary imported bytes")),
    );
    store.begin_binary(OP, &edit).unwrap();
    store.install_binary(OP, &edit).unwrap();
    store.rollback_binary(OP, &edit).unwrap();
    store.finish_binary(OP).unwrap();
    assert_eq!(
        store.read_binary(SOURCE).unwrap().unwrap(),
        b"old arbitrary imported bytes"
    );
    store.begin_binary(UNDO, &edit).unwrap();
    fs::write(root.path().join(DEST), b"user change").unwrap();
    assert!(store.install_binary(UNDO, &edit).is_err());
    assert!(store.rollback_binary(UNDO, &edit).is_err());
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), b"user change");
    assert!(store.binary_pending().unwrap().is_some());
}
#[test]
fn same_bytes_replacement_and_parent_replacement_do_not_inherit_binary_authority() {
    for replace_parent in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let store = open(root.path());
        let edit = prepare(&store, BinaryVersion::Missing);
        store.begin_binary(OP, &edit).unwrap();
        store.install_binary(OP, &edit).unwrap();
        store.finish_binary(OP).unwrap();
        if replace_parent {
            fs::rename(
                root.path().join("modules/game"),
                root.path().join("modules/old"),
            )
            .unwrap();
            fs::create_dir(root.path().join("modules/game")).unwrap();
        } else {
            fs::rename(
                root.path().join(DEST),
                root.path().join("modules/game/old.wasm"),
            )
            .unwrap();
        }
        fs::write(root.path().join(DEST), RAW).unwrap();
        assert!(store.begin_binary(UNDO, &edit.reversed()).is_err());
        assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
        assert!(store.binary_pending().unwrap().is_none());
    }
}
#[test]
fn binary_review_binds_named_node_and_both_byte_limits_precede_publication() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    fs::write(root.path().join(DEST), RAW).unwrap();
    let snapshot = store
        .capture_binary(SOURCE, &BinaryVersion::of(Some(RAW)))
        .unwrap();
    fs::rename(
        root.path().join(DEST),
        root.path().join("modules/game/old.wasm"),
    )
    .unwrap();
    fs::write(root.path().join(DEST), RAW).unwrap();
    assert!(store.check_binary_snapshot(&snapshot).is_err());
    let large = vec![0; MAX_WASM_BYTES + 1];
    let publication = BinaryPublication {
        module_id: ID,
        asset_id: UNDO,
        source_path: SOURCE,
        source_sha256: &format!("{:x}", Sha256::digest(b"source")),
        snapshot: &snapshot,
        bytes: &large,
    };
    assert!(store.prepare_binary(OP, publication).is_err());
    fs::write(root.path().join(DEST), &large).unwrap();
    assert!(store.read_binary(SOURCE).is_err());
    assert!(store.binary_pending().unwrap().is_none());
}
#[test]
fn binary_owned_slot_corruption_or_collision_cannot_become_publication_authority() {
    for collide in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let store = open(root.path());
        let edit = prepare(&store, BinaryVersion::Missing);
        if collide {
            fs::write(root.path().join(DEST), b"foreign").unwrap();
        } else {
            fs::write(
                root.path().join(STATE).join(format!("{OP}.wasm-after")),
                b"changed",
            )
            .unwrap();
        }
        assert!(store.begin_binary(OP, &edit).is_err());
        assert!(store.binary_pending().unwrap().is_none());
        if collide {
            assert_eq!(fs::read(root.path().join(DEST)).unwrap(), b"foreign");
        } else {
            assert!(!root.path().join(DEST).exists());
        }
    }
}
#[cfg(unix)]
#[test]
fn binary_source_destination_and_owned_slot_links_are_refused() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("raw"), RAW).unwrap();
    let store = open(root.path());
    symlink(outside.path().join("raw"), root.path().join(DEST)).unwrap();
    assert!(store.read_binary(SOURCE).is_err());
    fs::remove_file(root.path().join(DEST)).unwrap();
    fs::hard_link(outside.path().join("raw"), root.path().join(DEST)).unwrap();
    assert!(store.read_binary(SOURCE).is_err());
    fs::remove_file(root.path().join(DEST)).unwrap();
    let edit = prepare(&store, BinaryVersion::Missing);
    let slot = root.path().join(STATE).join(format!("{OP}.wasm-after"));
    fs::remove_file(&slot).unwrap();
    symlink(outside.path().join("raw"), &slot).unwrap();
    assert!(store.begin_binary(OP, &edit).is_err());
    assert_eq!(fs::read(outside.path().join("raw")).unwrap(), RAW);
}

#[test]
fn maximum_binary_is_admitted_and_moved_project_does_not_rebind_undo() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("root");
    let moved = outer.path().join("moved");
    fs::create_dir(&root).unwrap();
    let mut raw = vec![0; MAX_WASM_BYTES];
    raw[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    let edit = {
        let store = open(&root);
        let snapshot = store
            .capture_binary(SOURCE, &BinaryVersion::Missing)
            .unwrap();
        let edit = store
            .prepare_binary(
                OP,
                BinaryPublication {
                    module_id: ID,
                    asset_id: UNDO,
                    source_path: SOURCE,
                    source_sha256: &format!("{:x}", Sha256::digest(b"source")),
                    snapshot: &snapshot,
                    bytes: &raw,
                },
            )
            .unwrap();
        store.begin_binary(OP, &edit).unwrap();
        store.install_binary(OP, &edit).unwrap();
        store.finish_binary(OP).unwrap();
        edit
    };
    fs::rename(&root, &moved).unwrap();
    {
        let store = open(&moved);
        assert!(store.begin_binary(UNDO, &edit.reversed()).is_err());
        assert_eq!(fs::read(moved.join(DEST)).unwrap(), raw);
    }
    fs::rename(&moved, &root).unwrap();
    let store = open(&root);
    store.begin_binary(UNDO, &edit.reversed()).unwrap();
    store.install_binary(UNDO, &edit.reversed()).unwrap();
    store.finish_binary(UNDO).unwrap();
    assert!(!root.join(DEST).exists());
}

#[test]
fn binary_displacement_crash_child() {
    let Some(root) = std::env::var_os("INCANT_BINARY_SOURCE_CRASH_ROOT") else {
        return;
    };
    let store = open(Path::new(&root));
    let edit = prepare(&store, BinaryVersion::of(Some(b"prior")));
    store.begin_binary(OP, &edit).unwrap();
    store.install_binary(OP, &edit).unwrap();
    panic!("crash not reached");
}
#[test]
fn actual_process_death_between_binary_moves_recovers_exact_previous_node() {
    let root = tempfile::tempdir().unwrap();
    {
        let _store = open(root.path());
        fs::write(root.path().join(DEST), b"prior").unwrap();
    }
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "binary_tests::binary_displacement_crash_child",
            "--nocapture",
        ])
        .env("INCANT_BINARY_SOURCE_CRASH_ROOT", root.path())
        .env("INCANT_SOURCE_CRASH_TEST_PHASE", "binary_displaced")
        .status()
        .unwrap();
    assert_eq!(result.code(), Some(73));
    assert!(!root.path().join(DEST).exists());
    let store = open(root.path());
    let pending = store.binary_pending().unwrap().unwrap();
    store
        .rollback_binary(&pending.operation, &pending.edit)
        .unwrap();
    store.finish_binary(&pending.operation).unwrap();
    assert_eq!(store.read_binary(SOURCE).unwrap().unwrap(), b"prior");
}
