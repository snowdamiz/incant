//! Conservative engine-source identity for host-owned Metal archives.
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn emit() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let root =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory")).join("../..");
    let mut paths = Vec::new();
    // Include the complete workspace engine and cook source sets. This is
    // deliberately conservative: an unrelated engine source edit may go cold,
    // but new shaders/catalog constructors cannot silently retain an old key.
    collect(&root.join("crates"), &mut paths);
    collect(&root.join("third_party/wgpu-hal/src"), &mut paths);
    if root.join(".cargo").is_dir() {
        collect(&root.join(".cargo"), &mut paths);
    }
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "third_party/wgpu-hal/Cargo.toml",
        "third_party/wgpu-hal/build.rs",
    ] {
        paths.push(root.join(path));
    }
    paths.sort();
    let mut digest = Sha256::new();
    field(&mut digest, b"incant-metal-source-v1");
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path.strip_prefix(&root).expect("source path");
        field(
            &mut digest,
            relative.to_str().expect("UTF-8 engine path").as_bytes(),
        );
        field(
            &mut digest,
            &fs::read(&path).expect("engine cache identity source"),
        );
    }
    // Custom --cfg flags can select different shader/catalog construction even
    // with identical source and package features. Bind all Cargo cfg values and
    // encoded flags, not merely the profile label. Cargo's own rerun rules cover
    // target/features; explicit environment inputs also get rerun directives.
    for key in [
        "RUSTC",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "MACOSX_DEPLOYMENT_TARGET",
        "SDKROOT",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let mut environment: Vec<_> = env::vars()
        .filter(|(key, _)| {
            key.starts_with("CARGO_FEATURE_")
                || key.starts_with("CARGO_CFG_")
                || key.starts_with("CARGO_PROFILE_")
                || matches!(
                    key.as_str(),
                    "TARGET"
                        | "HOST"
                        | "PROFILE"
                        | "OPT_LEVEL"
                        | "DEBUG"
                        | "RUSTC"
                        | "RUSTFLAGS"
                        | "CARGO_ENCODED_RUSTFLAGS"
                        | "RUSTC_WRAPPER"
                        | "RUSTC_WORKSPACE_WRAPPER"
                        | "MACOSX_DEPLOYMENT_TARGET"
                        | "SDKROOT"
                )
        })
        .collect();
    environment.sort();
    for (key, value) in environment {
        if key.starts_with("CARGO_PROFILE_") {
            println!("cargo:rerun-if-env-changed={key}");
        }
        field(&mut digest, key.as_bytes());
        field(&mut digest, value.as_bytes());
    }
    // A local compiler identity query only: no shell, build, network or install.
    // Preserve failure rather than silently sharing a key across unknown tools.
    let compiler = std::process::Command::new(env::var_os("RUSTC").expect("Cargo compiler"))
        .arg("-vV")
        .output()
        .expect("compiler identity query");
    assert!(compiler.status.success(), "compiler identity query failed");
    field(&mut digest, &compiler.stdout);
    println!(
        "cargo:rustc-env=INCANT_METAL_ARCHIVE_SOURCE={:x}",
        digest.finalize()
    );
}
fn field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}
fn collect(directory: &Path, paths: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    for entry in fs::read_dir(directory).expect("engine source directory") {
        let entry = entry.expect("engine source entry");
        let kind = entry.file_type().expect("engine source type");
        let path = entry.path();
        if kind.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("target" | ".git" | "node_modules")
            ) {
                collect(&path, paths);
            }
        } else if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("rs" | "wgsl" | "metal" | "toml" | "h" | "c" | "cpp")
        ) {
            assert!(
                kind.is_file(),
                "engine source must be a regular file: {}",
                path.display()
            );
            paths.push(path);
        }
    }
}
