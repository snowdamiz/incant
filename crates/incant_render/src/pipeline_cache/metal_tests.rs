use super::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    process::Command,
};
fn fixture() -> (tempfile::TempDir, Store) {
    let base = std::env::var_os("INCANT_CACHE_TEST_ROOT")
        .or_else(|| std::env::var_os("HOME"))
        .expect("private test root");
    let root = tempfile::Builder::new()
        .prefix("incant-metal-cache-")
        .tempdir_in(base)
        .unwrap();
    assert!(root.path().is_dir(), "test root was not created");
    let cache = root.path().join("cache");
    let store = Store::open(&cache, [7; 32])
        .unwrap_or_else(|error| panic!("Store admission at {}: {error:?}", cache.display()));
    (root, store)
}
fn write(store: &Store, bytes: &[u8]) -> io::Result<usize> {
    store.save(|path| fs::write(path, bytes))
}
fn raw(store: &Store) -> PathBuf {
    store.directory.path(&store.raw).unwrap()
}
fn sidecar(store: &Store) -> PathBuf {
    store.directory.path(&store.metadata).unwrap()
}

#[test]
fn owned_round_trip_rejects_changed_missing_and_mismatched_bytes() {
    let (_root, store) = fixture();
    assert_eq!(store.load(), (None, CacheRead::Missing));
    assert_eq!(write(&store, b"native-only-test-bytes").unwrap(), 22);
    assert!(matches!(store.load().1, CacheRead::Supplied { bytes: 22 }));
    fs::write(raw(&store), b"different-native-data").unwrap();
    assert_eq!(store.load(), (None, CacheRead::Rejected));
    write(&store, b"native-only-test-bytes").unwrap();
    let mut header = fs::read(sidecar(&store)).unwrap();
    header[8] ^= 1;
    fs::write(sidecar(&store), header).unwrap();
    assert_eq!(store.load(), (None, CacheRead::Rejected));
    fs::remove_file(sidecar(&store)).unwrap();
    assert_eq!(store.load(), (None, CacheRead::Missing));
}
#[test]
fn bad_native_export_preserves_the_pair_and_explicitly_cleans_temporary() {
    let (root, store) = fixture();
    write(&store, b"first").unwrap();
    let result = store.save(|path| {
        fs::write(path, b"partial")?;
        Err(io::Error::other("native serialization refused"))
    });
    assert!(result.is_err());
    assert_eq!(fs::read(raw(&store)).unwrap(), b"first");
    assert!(matches!(store.load().1, CacheRead::Supplied { bytes: 5 }));
    assert_eq!(fs::read_dir(root.path().join("cache")).unwrap().count(), 2);
    assert!(store.save(|_| Ok(())).is_err());
    assert_eq!(fs::read_dir(root.path().join("cache")).unwrap().count(), 2);
}
#[test]
fn symlinks_hardlinks_and_permissive_files_never_reach_native_input() {
    let (root, store) = fixture();
    write(&store, b"first").unwrap();
    fs::set_permissions(raw(&store), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    fs::set_permissions(raw(&store), fs::Permissions::from_mode(0o600)).unwrap();
    let alias = root.path().join("alias");
    fs::hard_link(raw(&store), &alias).unwrap();
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    fs::remove_file(&alias).unwrap();
    fs::rename(raw(&store), &alias).unwrap();
    symlink(&alias, raw(&store)).unwrap();
    assert!(matches!(store.load().1, CacheRead::Io(_)));
}
#[test]
fn url_ancestors_and_replaced_directory_are_revalidated() {
    let (root, store) = fixture();
    write(&store, b"first").unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::rename(root.path().join("cache"), root.path().join("old")).unwrap();
    fs::create_dir(root.path().join("cache")).unwrap();
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    let link = root.path().join("link");
    symlink(root.path().join("old"), &link).unwrap();
    assert!(Store::open(&link, [7; 32]).is_err());
}
#[test]
fn oversized_sparse_native_file_is_rejected_before_reading() {
    let (_root, store) = fixture();
    write(&store, b"first").unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(raw(&store))
        .unwrap()
        .set_len(MAX_BYTES as u64 + 1)
        .unwrap();
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::InvalidData)
    ));
}
fn acl(path: &Path, action: &str, entry: &str) {
    assert!(
        Command::new("/bin/chmod")
            .args([action, entry])
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn darwin_deny_acl_is_admitted_and_file_or_ancestor_grant_is_refused() {
    let (root, store) = fixture();
    write(&store, b"first").unwrap();
    let path = raw(&store);
    acl(&path, "+a", "everyone deny execute");
    assert!(matches!(store.load().1, CacheRead::Supplied { bytes: 5 }));
    acl(&path, "-a", "everyone deny execute");
    acl(&path, "+a", "everyone allow read");
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    acl(&path, "-a", "everyone allow read");
    acl(root.path(), "+a", "everyone allow read");
    assert!(matches!(
        store.load().1,
        CacheRead::Io(io::ErrorKind::PermissionDenied)
    ));
    acl(root.path(), "-a", "everyone allow read");
}
#[test]
fn drop_has_no_publication_or_cleanup_side_effects() {
    let (root, store) = fixture();
    write(&store, b"first").unwrap();
    let archive = raw(&store);
    let metadata = sidecar(&store);
    let before = (fs::read(&archive).unwrap(), fs::read(&metadata).unwrap());
    drop(store);
    assert_eq!(
        (fs::read(archive).unwrap(), fs::read(metadata).unwrap()),
        before
    );
    assert_eq!(fs::read_dir(root.path().join("cache")).unwrap().count(), 2);
}
#[test]
fn malformed_sidecars_and_empty_archive_are_not_native_inputs() {
    let (_root, store) = fixture();
    write(&store, b"first").unwrap();
    fs::write(sidecar(&store), b"short").unwrap();
    assert_eq!(store.load(), (None, CacheRead::Rejected));
    fs::write(raw(&store), []).unwrap();
    fs::write(sidecar(&store), metadata(&store.identity, &[])).unwrap();
    assert_eq!(store.load(), (None, CacheRead::Rejected));
}
