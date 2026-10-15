use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};

fn main() {
    let root = Path::new("../..");
    let mut identity = Sha256::new();
    // The temporary probe admission is part of executable compatibility. Both
    // the trusted host producer and target consumer must explicitly opt in.
    identity.update(b"probe-android-bytecode");
    identity.update([u8::from(
        env::var_os("CARGO_FEATURE_PROBE_ANDROID_BYTECODE").is_some(),
    )]);
    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("pinned Cargo.lock");
    let wrapper = lock
        .split("[[package]]")
        .find(|p| p.contains("name = \"rquickjs\"\n"))
        .expect("pinned QuickJS wrapper");
    assert!(
        wrapper.contains("version = \"0.14.0\"")
            && wrapper
                .contains("source = \"registry+https://github.com/rust-lang/crates.io-index\"")
            && wrapper.contains(
                "checksum = \"a0a22bf72f515cc4189dd7c63d016f6de92a11eee98ddf7dd64794198f6eb34e\""
            ),
        "Re-audit the cooked script ABI before changing the pinned wrapper"
    );
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("workspace patches");
    for package in ["rquickjs-core", "rquickjs-sys"] {
        let entries: Vec<_> = lock
            .split("[[package]]")
            .filter(|p| p.contains(&format!("name = \"{package}\"\n")))
            .collect();
        assert!(
            entries.len() == 1
                && entries[0].contains("version = \"0.14.0\"")
                && !entries[0].contains("source = ")
                && !entries[0].contains("checksum = ")
                && manifest.contains(&format!(
                    "{package} = {{ path = \"third_party/{package}\" }}"
                )),
            "Re-audit the maintained-counter patch before changing the native engine"
        );
        // Include C, headers, bundled ABI declarations, build support and Rust,
        // with logical names. No machine-dependent checkout path enters identity.
        hash_vendor_tree(root, &root.join("third_party").join(package), &mut identity);
    }
    // cc accepts these environment overrides for the linked QuickJS build.
    // Cross-target admission uses the audited default C configuration only;
    // native cooks preserve their existing support for trusted custom flags.
    let target = env::var("TARGET").unwrap();
    let flags = [
        "CFLAGS".to_owned(),
        "HOST_CFLAGS".into(),
        "TARGET_CFLAGS".into(),
        format!("CFLAGS_{target}"),
        format!("CFLAGS_{}", target.replace('-', "_")),
    ];
    let mut default_flags = true;
    for key in flags {
        println!("cargo:rerun-if-env-changed={key}");
        default_flags &= env::var_os(key).is_none_or(|v| v.is_empty());
    }
    println!(
        "cargo:rustc-env=INCANT_SCRIPT_DEFAULT_C_FLAGS={}",
        u8::from(default_flags)
    );
    // Conservative compatibility identity, not authentication. Changes to the
    // pinned graph, wrapper, hardening or loader require a fresh trusted cook.
    for path in [
        root.join("Cargo.lock"),
        root.join("Cargo.toml"),
        Path::new("Cargo.toml").into(),
        Path::new("src/compiler.rs").into(),
        Path::new("src/compiled.rs").into(),
        Path::new("src/bindings.rs").into(),
        Path::new("src/bindings_embed.rs").into(),
        Path::new("src/lib.rs").into(),
        Path::new("src/budget.rs").into(),
        Path::new("src/diagnostics.rs").into(),
        Path::new("src/loan.rs").into(),
        Path::new("src/loan/ids.rs").into(),
        Path::new("src/loan/scratch.rs").into(),
        Path::new("src/heap_profile.rs").into(),
        Path::new("src/input.rs").into(),
        Path::new("src/hud.rs").into(),
        Path::new("src/flipbooks.rs").into(),
        Path::new("src/animations.rs").into(),
        Path::new("src/animation_ik.rs").into(),
        Path::new("src/animation_commands.rs").into(),
        Path::new("src/commands.rs").into(),
        Path::new("src/audio_commands.rs").into(),
        Path::new("src/light_commands.rs").into(),
        Path::new("src/queries.rs").into(),
        Path::new("src/query_values.rs").into(),
        Path::new("src/query_values_2d.rs").into(),
        Path::new("src/navigation_values.rs").into(),
        Path::new("src/steering_values.rs").into(),
        Path::new("src/localization_values.rs").into(),
        Path::new("src/schedule.rs").into(),
        Path::new("src/timers.rs").into(),
        Path::new("src/coroutines/mod.rs").into(),
        Path::new("src/coroutines/api.rs").into(),
        Path::new("src/coroutines/catalog.js").into(),
        Path::new("src/logging.rs").into(),
        Path::new("src/session.rs").into(),
        Path::new("src/program.rs").into(),
        Path::new("src/persistence.rs").into(),
        Path::new("src/checkpoint/mod.rs").into(),
        Path::new("src/checkpoint/pending.rs").into(),
        Path::new("src/checkpoint/native.rs").into(),
        Path::new("src/checkpoint/native/hosted.rs").into(),
        Path::new("src/checkpoint/native/prepare.rs").into(),
        Path::new("src/checkpoint/pending/native.rs").into(),
        Path::new("src/checkpoint/codec.rs").into(),
        Path::new("src/persistence/value.rs").into(),
        Path::new("src/artifact.rs").into(),
        Path::new("src/artifact/compatibility.rs").into(),
        Path::new("build.rs").into(),
        Path::new("src/harden.js").into(),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(&path).expect("script compatibility input");
        identity.update((bytes.len() as u64).to_le_bytes());
        identity.update(bytes);
    }
    println!(
        "cargo:rustc-env=INCANT_SCRIPT_BUILD_ID={:x}",
        identity.finalize()
    );
    // Ordinary-load compatibility covers the actual local native data/runtime
    // sources, not just the bytecode engine. This is conservative build identity,
    // not authentication, a VM snapshot or a complete system-toolchain receipt.
    let mut runtime = Sha256::new();
    for name in [
        "incant_runtime",
        "incant_profile",
        "incant_physics_runtime",
        "incant_physics_2d_runtime",
        "incant_input",
        "incant_types",
        "incant_2d",
        "incant_nav",
        "incant_localization",
        "incant_ui",
        "incant_text",
        "incant_audio_runtime",
        "incant_assets_runtime",
    ] {
        let dir = root.join("crates").join(name);
        hash_tree(&dir.join("src"), &mut runtime);
        hash_file(&dir.join("Cargo.toml"), &mut runtime);
    }
    // Path-patched runtime dependencies have no registry checksum in Cargo.lock.
    // Include their actual sources so a later patch cannot reuse stale saves.
    for name in ["kira", "parry2d", "parry3d", "rapier2d", "rapier3d"] {
        let dir = root.join("third_party").join(name);
        hash_tree(&dir.join("src"), &mut runtime);
        hash_file(&dir.join("Cargo.toml"), &mut runtime);
    }
    println!(
        "cargo:rustc-env=INCANT_CHECKPOINT_RUNTIME_ID={:x}",
        runtime.finalize()
    );
    println!(
        "cargo:rustc-env=INCANT_SCRIPT_TARGET={}",
        env::var("TARGET").unwrap()
    );
}

fn hash_file(path: &Path, hash: &mut Sha256) {
    println!("cargo:rerun-if-changed={}", path.display());
    let bytes = fs::read(path).expect("checkpoint runtime identity input");
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}
fn hash_tree(path: &Path, hash: &mut Sha256) {
    println!("cargo:rerun-if-changed={}", path.display());
    let mut paths: Vec<_> = fs::read_dir(path)
        .expect("runtime source directory")
        .map(|p| p.expect("source entry").path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            hash_tree(&path, hash);
        } else if path.extension().is_some_and(|s| s == "rs") {
            hash_file(&path, hash);
        }
    }
}

fn hash_vendor_tree(root: &Path, path: &Path, hash: &mut Sha256) {
    println!("cargo:rerun-if-changed={}", path.display());
    let mut paths: Vec<_> = fs::read_dir(path)
        .expect("vendored QuickJS directory")
        .map(|p| p.expect("vendor entry").path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            hash_vendor_tree(root, &path, hash);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("vendor below root")
                .to_str()
                .expect("UTF-8 vendor path")
                .replace('\\', "/");
            hash.update((relative.len() as u64).to_le_bytes());
            hash.update(relative.as_bytes());
            hash_file(&path, hash);
        }
    }
}
