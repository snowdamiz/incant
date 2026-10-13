use super::*;
use std::{
    fs as disk,
    os::unix::fs::{PermissionsExt, symlink},
    sync::Arc,
};

fn path(root: &tempfile::TempDir) -> std::path::PathBuf {
    root.path().join("cache")
}
fn mode(path: &Path, value: u32) {
    disk::set_permissions(path, disk::Permissions::from_mode(value)).unwrap();
}

#[test]
fn private_directory_reopens_and_reads_only_within_the_bound() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    assert!(Directory::open(&path, false).is_err());
    let directory = Directory::open(&path, true).unwrap();
    directory
        .publish("entry", &[b"header", b"payload"])
        .unwrap();
    assert_eq!(directory.read("entry", 13).unwrap(), b"headerpayload");
    assert_eq!(
        directory.read("entry", 12).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        directory.read("entry", usize::MAX).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        disk::metadata(&path).unwrap().permissions().mode() & 0o7777,
        0o700
    );
    assert_eq!(
        disk::metadata(path.join("entry"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        Directory::open(&path, false)
            .unwrap()
            .read("entry", 13)
            .unwrap(),
        b"headerpayload"
    );
}

#[test]
fn existing_and_later_widened_permissions_are_never_trusted_or_repaired() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("entry", &[b"private"]).unwrap();
    for permissions in [0o755, 0o750, 0o777, 0o1700] {
        mode(&path, permissions);
        assert!(Directory::open(&path, false).is_err());
        assert!(Directory::open(&path, true).is_err());
        assert!(directory.read("entry", 7).is_err());
        assert!(directory.publish("entry", &[b"rejected"]).is_err());
        assert_eq!(
            disk::metadata(&path).unwrap().permissions().mode() & 0o7777,
            permissions
        );
    }
    mode(&path, 0o700);
    assert_eq!(directory.read("entry", 7).unwrap(), b"private");
    for permissions in [0o644, 0o640, 0o606, 0o660, 0o4600] {
        mode(&path.join("entry"), permissions);
        assert!(directory.read("entry", 7).is_err());
    }
    mode(&path.join("entry"), 0o400);
    assert_eq!(directory.read("entry", 7).unwrap(), b"private");
}

#[test]
fn symlinks_hardlinks_and_fifo_are_rejected_before_reading() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("entry", &[b"private"]).unwrap();
    let alias = root.path().join("alias");
    symlink(&path, &alias).unwrap();
    assert!(Directory::open(&alias, false).is_err());
    symlink(path.join("entry"), path.join("symlink")).unwrap();
    assert!(directory.read("symlink", 100).is_err());
    disk::hard_link(path.join("entry"), root.path().join("hardlink")).unwrap();
    assert!(directory.read("entry", 100).is_err());
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fs::mkfifoat(&directory.fd, "fifo", Mode::RUSR | Mode::WUSR).unwrap();
    // rustix intentionally omits mkfifoat on Darwin. Only the test fixture
    // uses the platform utility; production never launches a process.
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    assert!(
        std::process::Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(path.join("fifo"))
            .status()
            .unwrap()
            .success()
    );
    // No writer is opened: this would block indefinitely without NONBLOCK.
    assert!(directory.read("fifo", 100).is_err());
    disk::create_dir(path.join("subdirectory")).unwrap();
    assert!(directory.read("subdirectory", 100).is_err());
}

#[test]
fn held_directory_is_not_rebound_by_path_or_ancestor_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("parent/cache");
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("entry", &[b"original"]).unwrap();
    disk::rename(root.path().join("parent"), root.path().join("retained")).unwrap();
    let replacement = root.path().join("replacement");
    let other = Directory::open(&replacement.join("cache"), true).unwrap();
    other.publish("entry", &[b"replacement"]).unwrap();
    symlink(&replacement, root.path().join("parent")).unwrap();
    assert_eq!(directory.read("entry", 32).unwrap(), b"original");
    directory.publish("entry", &[b"updated original"]).unwrap();
    assert_eq!(other.read("entry", 32).unwrap(), b"replacement");
    assert_eq!(
        disk::read(root.path().join("retained/cache/entry")).unwrap(),
        b"updated original"
    );
}

#[test]
fn publication_never_follows_a_destination_symlink() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    let directory = Directory::open(&path, true).unwrap();
    let victim = root.path().join("unrelated");
    disk::write(&victim, b"keep").unwrap();
    symlink(&victim, path.join("entry")).unwrap();
    directory.publish("entry", &[b"owned cache"]).unwrap();
    assert_eq!(disk::read(victim).unwrap(), b"keep");
    assert_eq!(directory.read("entry", 100).unwrap(), b"owned cache");
    assert_eq!(disk::read_dir(&path).unwrap().count(), 1);
}

#[test]
fn failed_rename_cleans_the_temporary_without_clobbering_other_data() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    let directory = Directory::open(&path, true).unwrap();
    directory.publish("existing", &[b"keep"]).unwrap();
    disk::create_dir(path.join("blocked")).unwrap();
    disk::write(path.join("blocked/child"), b"keep child").unwrap();
    assert!(directory.publish("blocked", &[b"replacement"]).is_err());
    assert_eq!(disk::read_dir(&path).unwrap().count(), 2);
    assert_eq!(directory.read("existing", 100).unwrap(), b"keep");
    assert_eq!(
        disk::read(path.join("blocked/child")).unwrap(),
        b"keep child"
    );
}

#[test]
fn invalid_names_cannot_escape_the_directory() {
    let root = tempfile::tempdir().unwrap();
    assert!(Directory::open(Path::new("relative"), true).is_err());
    let directory = Directory::open(&path(&root), true).unwrap();
    for name in [
        "",
        ".",
        "..",
        "../escape",
        "/absolute",
        "part/entry",
        "part\\entry",
        "nul\0entry",
    ] {
        assert_eq!(
            directory.read(name, 100).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            directory.publish(name, &[b"x"]).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(disk::read_dir(path(&root)).unwrap().count(), 0);
}

#[test]
fn concurrent_replacement_reads_only_complete_entries_and_removes_temps() {
    let root = tempfile::tempdir().unwrap();
    let path = path(&root);
    let directory = Arc::new(Directory::open(&path, true).unwrap());
    directory.publish("entry", &[&[9; 4096]]).unwrap();
    std::thread::scope(|scope| {
        for byte in 1..=4 {
            let directory = Arc::clone(&directory);
            scope.spawn(move || {
                for _ in 0..8 {
                    directory
                        .publish("entry", &[&[byte; 2048], &[byte; 2048]])
                        .unwrap();
                }
            });
        }
        for _ in 0..64 {
            match directory.read("entry", 4096) {
                Ok(bytes) => {
                    assert_eq!(bytes.len(), 4096);
                    assert!(bytes.iter().all(|byte| *byte == bytes[0]));
                }
                // A valid held old file can be unlinked by publication after
                // open; refusing that now-zero-link snapshot is a safe miss.
                Err(error) => assert_eq!(error.kind(), io::ErrorKind::PermissionDenied),
            }
        }
    });
    let bytes = directory.read("entry", 4096).unwrap();
    assert!((1..=4).contains(&bytes[0]));
    assert!(bytes.iter().all(|byte| *byte == bytes[0]));
    assert_eq!(disk::read_dir(path).unwrap().count(), 1);
}
