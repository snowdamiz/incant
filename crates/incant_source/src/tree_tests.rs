use super::*;
use std::fs;

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const CREATE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const UNDO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
const PATH: &str = "modules/movement/module.rs";
const TEXT: &str = "pub fn update() {}\n";

fn store(root: &Path) -> SourceStore {
    let journal = root.join("journal");
    if !journal.exists() {
        fs::write(&journal, "").unwrap();
    }
    SourceStore::open(root, &journal, ID).unwrap()
}
fn prepared(store: &SourceStore) -> SourceEdit {
    SourceEdit {
        script_id: ID.into(),
        kind: SourceKind::Rust,
        path: PATH.into(),
        before: None,
        after: Some(TEXT.into()),
        created_tree: store.prepare_creation(CREATE, PATH, TEXT).unwrap(),
    }
}

#[test]
fn private_stage_never_publishes_before_install_and_reopen_restores_owned_tree() {
    let root = tempfile::tempdir().unwrap();
    let edit = {
        let store = store(root.path());
        let edit = prepared(&store);
        assert!(edit.created_tree.is_some());
        assert!(!root.path().join("modules").exists());
        assert_eq!(store.read(PATH).unwrap(), None);
        store.begin(CREATE, &edit).unwrap();
        assert!(!root.path().join("modules").exists());
        store.install(CREATE, &edit).unwrap();
        assert_eq!(store.read(PATH).unwrap().as_deref(), Some(TEXT));
        edit
    };
    let store = store(root.path());
    assert_eq!(store.pending().unwrap().unwrap().edit, edit);
    store.rollback(CREATE, &edit).unwrap();
    store.finish(CREATE).unwrap();
    assert!(!root.path().join("modules").exists());
    // The original identities remain in retained storage and can be republished.
    store.begin(UNDO, &edit).unwrap();
    store.install(UNDO, &edit).unwrap();
    store.finish(UNDO).unwrap();
    assert_eq!(store.read(PATH).unwrap().as_deref(), Some(TEXT));
}

#[test]
fn racing_destination_and_foreign_entries_are_preserved_not_merged_or_deleted() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path());
    let edit = prepared(&store);
    store.begin(CREATE, &edit).unwrap();
    fs::create_dir(root.path().join("modules")).unwrap();
    fs::write(root.path().join("modules/user.txt"), "foreign").unwrap();
    assert!(store.install(CREATE, &edit).is_err());
    assert!(store.rollback(CREATE, &edit).is_err());
    assert!(store.ensure_ready().is_err());
    assert_eq!(
        fs::read(root.path().join("modules/user.txt")).unwrap(),
        b"foreign"
    );
    assert!(!root.path().join(PATH).exists());
    assert!(store.pending().unwrap().is_some());
}

#[test]
fn ownership_receipt_cannot_be_omitted_or_rebound_to_another_root() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path());
    let edit = prepared(&store);
    let tree = edit.created_tree.as_ref().unwrap();
    let value = serde_json::to_value(tree).unwrap();
    let receipt = root
        .path()
        .join(STATE)
        .join(format!("{}.ownership", value["token"].as_str().unwrap()));
    let original = fs::read(&receipt).unwrap();
    fs::remove_file(&receipt).unwrap();
    assert!(matches!(
        store.begin(CREATE, &edit),
        Err(SourceError::RecoveryRequired)
    ));
    fs::write(&receipt, &original).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    changed["binding"] = serde_json::json!("foreign root/journal");
    fs::write(&receipt, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(matches!(
        store.begin(CREATE, &edit),
        Err(SourceError::RecoveryRequired)
    ));
    assert!(!root.path().join("modules").exists());
}

#[test]
fn changed_owned_tree_blocks_undo_and_recovery_preserves_added_content() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path());
    let edit = prepared(&store);
    store.begin(CREATE, &edit).unwrap();
    store.install(CREATE, &edit).unwrap();
    store.finish(CREATE).unwrap();
    let undo = edit.reversed();
    store.begin(UNDO, &undo).unwrap();
    fs::write(root.path().join("modules/movement/extra.rs"), "// user").unwrap();
    assert!(store.install(UNDO, &undo).is_err());
    assert!(store.rollback(UNDO, &undo).is_err());
    assert_eq!(store.read(PATH).unwrap().as_deref(), Some(TEXT));
    assert_eq!(
        fs::read(root.path().join("modules/movement/extra.rs")).unwrap(),
        b"// user"
    );
    assert!(store.pending().unwrap().is_some());
}

#[test]
fn replaced_same_content_directory_does_not_inherit_ownership() {
    let root = tempfile::tempdir().unwrap();
    let edit = {
        let store = store(root.path());
        let edit = prepared(&store);
        store.begin(CREATE, &edit).unwrap();
        store.install(CREATE, &edit).unwrap();
        store.finish(CREATE).unwrap();
        edit
    };
    fs::rename(root.path().join("modules"), root.path().join("old-modules")).unwrap();
    fs::create_dir_all(root.path().join("modules/movement")).unwrap();
    fs::write(root.path().join(PATH), TEXT).unwrap();
    let store = store(root.path());
    assert!(store.begin(UNDO, &edit.reversed()).is_err());
    assert_eq!(fs::read(root.path().join(PATH)).unwrap(), TEXT.as_bytes());
    assert_eq!(
        fs::read(root.path().join("old-modules/movement/module.rs")).unwrap(),
        TEXT.as_bytes()
    );
    assert!(store.pending().unwrap().is_none());
}

#[test]
fn platform_subtree_move_never_replaces_an_existing_destination() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("staged")).unwrap();
    fs::create_dir(root.path().join("foreign")).unwrap();
    fs::write(root.path().join("staged/module.rs"), TEXT).unwrap();
    fs::write(root.path().join("foreign/user.txt"), "user").unwrap();
    let dir = Dir::root(root.path()).unwrap();
    assert!(dir.move_to("staged", &dir, "foreign").is_err());
    assert_eq!(
        fs::read(root.path().join("staged/module.rs")).unwrap(),
        TEXT.as_bytes()
    );
    assert_eq!(
        fs::read(root.path().join("foreign/user.txt")).unwrap(),
        b"user"
    );
    assert!(!root.path().join("foreign/module.rs").exists());
}

#[test]
fn subtree_metadata_is_bounded_and_cannot_authorize_a_typescript_or_unrelated_edit() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path());
    let edit = prepared(&store);
    for (field, value) in [
        ("root", serde_json::json!("elsewhere")),
        ("token", serde_json::json!("../outside")),
        ("directories", serde_json::json!([])),
        (
            "directories",
            serde_json::to_value(vec![[1_u64, 2]; 8]).unwrap(),
        ),
    ] {
        let mut raw = serde_json::to_value(&edit).unwrap();
        raw["created_tree"][field] = value;
        assert!(
            serde_json::from_value::<SourceEdit>(raw)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    let mut changed = edit.clone();
    changed.kind = SourceKind::TypeScript;
    changed.path = "modules/movement/module.ts".into();
    assert!(changed.validate().is_err());
    changed = edit.clone();
    changed.before = Some(TEXT.into());
    assert!(changed.validate().is_err());
    assert_eq!(edit.reversed().reversed(), edit);
    assert!(!root.path().join("modules").exists());
}

#[cfg(unix)]
#[test]
fn staged_symlink_or_hardlinked_leaf_cannot_publish() {
    use std::os::unix::fs::symlink;
    for hardlink in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("module.rs"), TEXT).unwrap();
        let store = store(root.path());
        let edit = prepared(&store);
        let value = serde_json::to_value(&edit.created_tree).unwrap();
        let leaf = root.path().join(STATE).join(format!(
            "{}.tree/movement/module.rs",
            value["token"].as_str().unwrap()
        ));
        fs::remove_file(&leaf).unwrap();
        if hardlink {
            fs::hard_link(outside.path().join("module.rs"), &leaf).unwrap();
        } else {
            symlink(outside.path().join("module.rs"), &leaf).unwrap();
        }
        assert!(store.begin(CREATE, &edit).is_err());
        assert!(!root.path().join("modules").exists());
        assert_eq!(
            fs::read(outside.path().join("module.rs")).unwrap(),
            TEXT.as_bytes()
        );
    }
}
