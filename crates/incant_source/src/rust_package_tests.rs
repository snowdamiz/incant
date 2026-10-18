use super::*;
use std::fs;
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("logic/sub")).unwrap();
    fs::write(root.path().join("logic/main.rs"), "mod sub;").unwrap();
    fs::write(root.path().join("logic/sub/helper.rs"), "const X:u8=1;").unwrap();
    root
}
#[test]
fn registered_package_excludes_only_exact_bounded_derived_output() {
    let root = fixture();
    let before = RustPackageSnapshot::capture(root.path(), "logic/main.rs").unwrap();
    let expected = before.files().to_vec();
    drop(before);
    fs::write(
        root.path().join("logic/main.wasm"),
        b"private binary output",
    )
    .unwrap();
    let after = RustPackageSnapshot::capture(root.path(), "logic/main.rs").unwrap();
    assert_eq!(after.files(), expected);
    after.validate().unwrap();
    drop(after);
    for file in [
        "extra.wasm",
        "Main.wasm",
        "build.rs",
        "Cargo.toml",
        ".hidden",
    ] {
        let p = root.path().join("logic").join(file);
        // Do not clobber the existing case-insensitive output on Windows/macOS.
        if p.exists() {
            continue;
        }
        fs::write(&p, b"untrusted").unwrap();
        assert!(
            RustPackageSnapshot::capture(root.path(), "logic/main.rs").is_err(),
            "{file}"
        );
        fs::remove_file(p).unwrap();
    }
    fs::write(
        root.path().join("logic/main.wasm"),
        vec![0; MAX_WASM_BYTES + 1],
    )
    .unwrap();
    assert!(RustPackageSnapshot::capture(root.path(), "logic/main.rs").is_err());
}
#[test]
fn held_package_rejects_sibling_changes_additions_removals_and_same_byte_replacement() {
    for case in ["edit", "add", "remove", "replace"] {
        let root = fixture();
        let snapshot = RustPackageSnapshot::capture(root.path(), "logic/main.rs").unwrap();
        let path = root.path().join("logic/sub/helper.rs");
        match case {
            "edit" => fs::write(&path, "const X:u8=2;").unwrap(),
            "add" => fs::write(root.path().join("logic/new.rs"), "new").unwrap(),
            "remove" => fs::remove_file(&path).unwrap(),
            _ => {
                fs::remove_file(&path).unwrap();
                fs::write(&path, "const X:u8=1;").unwrap();
            }
        }
        assert!(snapshot.validate().is_err(), "{case}");
    }
}
#[test]
fn package_links_aliases_and_all_resource_bounds_fail_closed() {
    for case in [
        "hardlink",
        "output-hardlink",
        "file",
        "total",
        "count",
        "inventory",
        "depth",
        "alias",
    ] {
        let root = fixture();
        let package = root.path().join("logic");
        match case {
            "hardlink" => fs::hard_link(package.join("main.rs"), package.join("alias.rs")).unwrap(),
            "output-hardlink" => {
                fs::hard_link(package.join("main.rs"), package.join("main.wasm")).unwrap()
            }
            "file" => fs::write(
                package.join("main.rs"),
                vec![b'x'; MAX_RUST_SOURCE_BYTES + 1],
            )
            .unwrap(),
            "total" => {
                for i in 0..5 {
                    fs::write(
                        package.join(format!("f{i}.rs")),
                        vec![b'x'; MAX_RUST_SOURCE_BYTES],
                    )
                    .unwrap();
                }
            }
            "count" => {
                for i in 0..32 {
                    fs::write(package.join(format!("f{i}.rs")), b"x").unwrap();
                }
            }
            "inventory" => {
                for i in 0..129 {
                    fs::create_dir(package.join(format!("d{i}"))).unwrap();
                }
            }
            "depth" => fs::create_dir_all(package.join("a/b/c/d/e/f/g/h")).unwrap(),
            _ => {
                let p = package.join("MAIN.rs");
                if p.exists() {
                    continue;
                }
                fs::write(p, b"x").unwrap();
            }
        }
        assert!(
            RustPackageSnapshot::capture(root.path(), "logic/main.rs").is_err(),
            "{case}"
        );
    }
    #[cfg(unix)]
    {
        for output in [false, true] {
            let root = fixture();
            let path = root.path().join(if output {
                "logic/main.wasm"
            } else {
                "logic/link"
            });
            std::os::unix::fs::symlink(root.path(), path).unwrap();
            assert!(RustPackageSnapshot::capture(root.path(), "logic/main.rs").is_err());
        }
    }
}
