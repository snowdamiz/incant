// Incant default-off checked-state identity. No runtime compilation or tool calls.
#[cfg(not(feature = "incant-checked-restore"))]
fn main() {}
#[cfg(feature = "incant-checked-restore")]
fn main() {
    use sha2::{Digest, Sha256};
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::Command,
    };
    fn item(h: &mut Sha256, name: &str, bytes: &[u8]) {
        h.update((name.len() as u64).to_le_bytes());
        h.update(name.as_bytes());
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    fn file(h: &mut Sha256, root: &Path, path: &Path) {
        println!("cargo:rerun-if-changed={}", path.display());
        assert!(
            !fs::symlink_metadata(path)
                .expect("source metadata")
                .file_type()
                .is_symlink(),
            "identity source symlink"
        );
        let relative = path
            .strip_prefix(root)
            .expect("relative identity input")
            .to_str()
            .expect("UTF-8 source name");
        item(
            h,
            &relative.replace('\\', "/"),
            &fs::read(path).expect("identity input"),
        );
    }
    fn tree(h: &mut Sha256, root: &Path, path: &Path) {
        println!("cargo:rerun-if-changed={}", path.display());
        let mut paths: Vec<PathBuf> = fs::read_dir(path)
            .expect("source directory")
            .map(|p| p.expect("source entry").path())
            .collect();
        paths.sort();
        for p in paths {
            assert!(
                !fs::symlink_metadata(&p)
                    .expect("source metadata")
                    .file_type()
                    .is_symlink(),
                "identity source symlink"
            );
            if p.is_dir() {
                tree(h, root, &p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                file(h, root, &p);
            }
        }
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let mut h = Sha256::new();
    item(&mut h, "domain", b"Incant native checkpoint build v1");
    // The only relocatable dependency path is replaced by a logical label;
    // its actual source tree is independently hashed below. Prepared harnesses
    // rewrite that path to absolute, without changing compatibility inputs.
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("native manifest");
    let mut in_snapshot = false;
    let mut snapshot = None;
    let mut logical = String::new();
    for line in manifest.lines() {
        if line.starts_with('[') {
            in_snapshot = line == "[dependencies.incant_physics_snapshot]";
        }
        if in_snapshot && line.trim_start().starts_with("path = ") {
            let value = line.trim_start().strip_prefix("path = ").unwrap();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .expect("literal dependency path");
            assert!(!value.contains('\\'), "escaped dependency path unsupported");
            assert!(
                snapshot.replace(root.join(value)).is_none(),
                "duplicate snapshot path"
            );
            logical.push_str("path = \"<incant_physics_snapshot>\"\n");
        } else {
            logical.push_str(line);
            logical.push('\n');
        }
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join("Cargo.toml").display()
    );
    item(&mut h, "Cargo.toml", logical.as_bytes());
    file(&mut h, &root, &root.join("build.rs"));
    tree(&mut h, &root, &root.join("src"));
    let snapshot = snapshot.expect("exact optional inert dependency source");
    item(&mut h, "dependency", b"incant_physics_snapshot");
    file(&mut h, &snapshot, &snapshot.join("Cargo.toml"));
    tree(&mut h, &snapshot, &snapshot.join("src"));
    // Vendor directories retain ignored upstream locks. The supported producer
    // is an explicit enclosing workspace (repository or prepared native harness),
    // not an arbitrary standalone vendor invocation. Select its actual lock.
    let workspace = root
        .ancestors()
        .find(|p| {
            std::fs::read_to_string(p.join("Cargo.toml"))
                .is_ok_and(|s| s.lines().any(|l| l.trim() == "[workspace]"))
        })
        .expect("checked native build needs explicit enclosing workspace");
    let lock = workspace.join("Cargo.lock");
    assert!(lock.is_file(), "checked workspace lock missing");
    println!("cargo:rerun-if-changed={}", lock.display());
    item(
        &mut h,
        "Cargo.lock",
        &fs::read(lock).expect("resolved dependency identities"),
    );
    let mut features: Vec<_> = env::vars()
        .filter(|(k, _)| k.starts_with("CARGO_FEATURE_"))
        .collect();
    features.sort();
    for (k, v) in features {
        item(&mut h, &k, v.as_bytes());
    }
    for key in [
        "TARGET",
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_CFG_TARGET_ENDIAN",
        "CARGO_CFG_TARGET_POINTER_WIDTH",
        "CARGO_CFG_TARGET_FEATURE",
        "CARGO_CFG_TARGET_OS",
        "CARGO_CFG_TARGET_ENV",
        "CARGO_CFG_TARGET_ABI",
        "CARGO_CFG_PANIC",
        "CARGO_ENCODED_RUSTFLAGS",
        "OPT_LEVEL",
        "DEBUG",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        item(&mut h, key, env::var(key).unwrap_or_default().as_bytes());
    }
    let compiler = Command::new(env::var_os("RUSTC").expect("Cargo compiler"))
        .arg("-vV")
        .output()
        .expect("compiler identity");
    assert!(compiler.status.success(), "compiler identity failed");
    item(&mut h, "rustc -vV", &compiler.stdout);
    let bytes: [u8; 32] = h.finalize().into();
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output"));
    fs::write(output.join("incant_identity.rs"),format!("/// Exact source, feature, target and compiler compatibility digest.\npub const SOURCE_ID: [u8; 32] = {bytes:?};\n")).expect("write identity");
}
