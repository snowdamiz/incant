use super::*;

fn junction(target: &Path, link: &Path) {
    // Unlike a symlink, a directory junction needs no Developer Mode or
    // SeCreateSymbolicLinkPrivilege. Failure to create it is a test failure,
    // never evidence that the resolver refused a reparse point.
    let output = std::process::Command::new("cmd")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction setup failed: {:?}",
        output
    );
}

#[test]
fn known_folder_resolves_one_stable_dependency_root_outside_projects() {
    let base = tempfile::tempdir().unwrap();
    let host = base.path().join("Local data Ω");
    let project = base.path().join("project");
    std::fs::create_dir(&host).unwrap();
    std::fs::create_dir(&project).unwrap();
    let first = resolve(&host, &project).unwrap();
    let second = resolve(&host, &project).unwrap();
    assert_eq!(first.path(), host.canonicalize().unwrap().join("swc"));
    assert_eq!(first.path(), second.path());
    assert!(
        !first.path().exists(),
        "resolution must not materialize an addon"
    );
    assert!(
        std::fs::rename(&host, base.path().join("moved")).is_err(),
        "parent must stay pinned"
    );
    drop((first, second));
    let moved = base.path().join("moved");
    std::fs::rename(&host, &moved).unwrap();
    std::fs::create_dir(moved.join("swc")).unwrap();
    let existing = resolve(&moved, &project).unwrap();
    assert!(std::fs::rename(moved.join("swc"), moved.join("renamed")).is_err());
    drop(existing);
    std::fs::rename(moved.join("swc"), moved.join("renamed")).unwrap();
}

#[test]
fn cache_inside_project_and_case_aliases_are_refused_before_creation() {
    let base = tempfile::tempdir().unwrap();
    let host = base.path().join("Host Ärea");
    std::fs::create_dir(&host).unwrap();
    assert!(resolve(&host, &host).is_err());
    assert!(!host.join("swc").exists());
    let canonical = host.canonicalize().unwrap();
    let alternate = PathBuf::from(canonical.as_os_str().to_string_lossy().to_uppercase());
    assert!(resolve(&host, &alternate).is_err());
    assert!(overlaps(&canonical.join("swc"), &alternate));
    let project = host.join("swc/project");
    std::fs::create_dir_all(&project).unwrap();
    assert!(resolve(&host, &project).is_err());
    assert!(!overlaps(
        &canonical.join("swc"),
        &canonical.join("swc-other")
    ));
}

#[test]
fn invalid_or_reparse_cache_paths_refuse_without_repair() {
    let base = tempfile::tempdir().unwrap();
    let project = base.path().join("project");
    let host = base.path().join("host");
    let outside = base.path().join("outside");
    for p in [&project, &host, &outside] {
        std::fs::create_dir(p).unwrap();
    }
    for p in [
        Path::new("relative"),
        Path::new(r"C:relative"),
        Path::new(r"\\.\pipe\host"),
    ] {
        assert!(resolve(p, &project).is_err());
    }
    std::fs::write(host.join("swc"), b"do not replace").unwrap();
    assert!(resolve(&host, &project).is_err());
    assert_eq!(std::fs::read(host.join("swc")).unwrap(), b"do not replace");
    std::fs::remove_file(host.join("swc")).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside remains untouched").unwrap();
    junction(&outside, &host.join("swc"));
    assert!(resolve(&host, &project).is_err());
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside remains untouched"
    );
    let alias = base.path().join("alias");
    junction(&host, &alias);
    assert!(resolve(&alias, &project).is_err());
    std::fs::remove_dir(alias).unwrap();
    std::fs::remove_dir(host.join("swc")).unwrap();
}

#[test]
fn actual_known_folder_comes_from_os_and_is_absolute() {
    let known = local_app_data().unwrap();
    assert!(known.is_absolute());
    let root = tempfile::tempdir().unwrap();
    let cache = dependency_cache(root.path()).unwrap();
    assert_eq!(cache.path(), known.canonicalize().unwrap().join("swc"));
}
