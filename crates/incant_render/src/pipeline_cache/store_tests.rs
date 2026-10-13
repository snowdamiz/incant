use super::*;
use std::fs;

fn adapter() -> wgpu::AdapterInfo {
    wgpu::AdapterInfo {
        name: "test adapter".into(),
        vendor: 123,
        device: 456,
        device_type: wgpu::DeviceType::DiscreteGpu,
        device_pci_bus_id: String::new(),
        driver: "test driver".into(),
        driver_info: "1.0".into(),
        backend: wgpu::Backend::Vulkan,
        subgroup_min_size: 32,
        subgroup_max_size: 32,
        transient_saves_memory: false,
    }
}
fn store(root: &Path) -> Store {
    Store::new(root.join("pipelines"), &adapter())
}

#[test]
fn cold_cache_reopens_and_atomically_replaces_complete_driver_bytes() {
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    assert_eq!(s.load(), (None, CacheRead::Missing));
    s.save(b"opaque driver data").unwrap();
    let reopened = store(root.path());
    assert_eq!(
        reopened.load(),
        (
            Some(b"opaque driver data".to_vec()),
            CacheRead::Supplied { bytes: 18 }
        )
    );
    reopened.save(b"replacement").unwrap();
    assert_eq!(s.load().0.unwrap(), b"replacement");
    assert_eq!(fs::read_dir(&s.directory).unwrap().count(), 1);
}

#[test]
fn corrupt_truncated_extended_and_foreign_envelopes_never_supply_driver_bytes() {
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    s.save(b"real driver output fixture").unwrap();
    let valid = fs::read(s.path()).unwrap();
    for length in [0, 7, 40, 79, valid.len() - 1] {
        fs::write(s.path(), &valid[..length]).unwrap();
        assert_eq!(s.load(), (None, CacheRead::Rejected));
    }
    for offset in [0, 8, 40, 48, HEADER] {
        let mut corrupt = valid.clone();
        corrupt[offset] ^= 1;
        fs::write(s.path(), corrupt).unwrap();
        assert_eq!(s.load(), (None, CacheRead::Rejected));
    }
    let mut extra = valid.clone();
    extra.push(0);
    fs::write(s.path(), extra).unwrap();
    assert_eq!(s.load(), (None, CacheRead::Rejected));
    let mut huge = valid;
    huge[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
    fs::write(s.path(), huge).unwrap();
    assert_eq!(s.load(), (None, CacheRead::Rejected));
}

#[test]
fn adapter_and_driver_upgrades_get_separate_cold_entries() {
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    s.save(b"driver data").unwrap();
    let mut variants = Vec::new();
    let mut a = adapter();
    a.driver_info = "2.0".into();
    variants.push(a);
    let mut a = adapter();
    a.vendor += 1;
    variants.push(a);
    let mut a = adapter();
    a.device += 1;
    variants.push(a);
    let mut a = adapter();
    a.driver = "other driver".into();
    variants.push(a);
    let mut a = adapter();
    a.backend = wgpu::Backend::Dx12;
    variants.push(a);
    for a in variants {
        let other = Store::new(s.directory.clone(), &a);
        assert_ne!(other.path(), s.path());
        assert_eq!(other.load(), (None, CacheRead::Missing));
        fs::copy(s.path(), other.path()).unwrap();
        assert_eq!(other.load(), (None, CacheRead::Rejected));
    }
}

#[test]
fn rejected_writes_preserve_the_last_complete_cache() {
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    s.save(b"keep me").unwrap();
    assert_eq!(s.save(b"").unwrap_err().kind(), io::ErrorKind::InvalidData);
    assert_eq!(s.load().0.unwrap(), b"keep me");
    assert_eq!(fs::read_dir(&s.directory).unwrap().count(), 1);
}

#[test]
fn unavailable_directory_is_an_io_result_not_a_renderer_error() {
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    fs::write(&s.directory, b"not a directory").unwrap();
    assert!(matches!(s.load().1, CacheRead::Io(_)));
    assert!(s.save(b"driver data").is_err());
    assert_eq!(fs::read(&s.directory).unwrap(), b"not a directory");
}

#[test]
fn concurrent_publishers_leave_one_complete_entry_and_no_temporaries() {
    let root = tempfile::tempdir().unwrap();
    std::thread::scope(|scope| {
        for byte in 1..=8 {
            let root = root.path();
            scope.spawn(move || store(root).save(&vec![byte; 8192]).unwrap());
        }
    });
    let s = store(root.path());
    let bytes = s.load().0.unwrap();
    assert_eq!(bytes.len(), 8192);
    assert!((1..=8).contains(&bytes[0]));
    assert!(bytes.iter().all(|b| *b == bytes[0]));
    assert_eq!(fs::read_dir(&s.directory).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn symlink_leaf_is_not_read_or_used_as_a_write_destination() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let root = tempfile::tempdir().unwrap();
    let s = store(root.path());
    s.save(b"initial").unwrap();
    let victim = root.path().join("unrelated");
    fs::rename(s.path(), &victim).unwrap();
    let original = fs::read(&victim).unwrap();
    symlink(&victim, s.path()).unwrap();
    assert!(matches!(s.load(), (None, CacheRead::Io(_))));
    s.save(b"new cache").unwrap();
    assert_eq!(fs::read(&victim).unwrap(), original);
    assert_eq!(s.load().0.unwrap(), b"new cache");
    assert_eq!(
        fs::metadata(s.path()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&s.directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

use std::path::Path;
