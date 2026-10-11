use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};

fn main() {
    let root = Path::new("../..");
    let mut identity = Sha256::new();
    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("pinned Cargo.lock");
    for package in ["rquickjs", "rquickjs-core", "rquickjs-sys"] {
        assert!(
            lock.contains(&format!("name = \"{package}\"\nversion = \"0.14.0\"")),
            "Re-audit the cooked script ABI before changing the pinned engine"
        );
    }
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
        Path::new("src/input.rs").into(),
        Path::new("src/flipbooks.rs").into(),
        Path::new("src/commands.rs").into(),
        Path::new("src/audio_commands.rs").into(),
        Path::new("src/queries.rs").into(),
        Path::new("src/query_values.rs").into(),
        Path::new("src/query_values_2d.rs").into(),
        Path::new("src/navigation_values.rs").into(),
        Path::new("src/localization_values.rs").into(),
        Path::new("src/schedule.rs").into(),
        Path::new("src/timers.rs").into(),
        Path::new("src/logging.rs").into(),
        Path::new("src/session.rs").into(),
        Path::new("src/program.rs").into(),
        Path::new("src/persistence.rs").into(),
        Path::new("src/checkpoint/mod.rs").into(),
        Path::new("src/checkpoint/codec.rs").into(),
        Path::new("src/persistence/value.rs").into(),
        Path::new("src/artifact.rs").into(),
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
        "incant_input",
        "incant_types",
        "incant_2d",
        "incant_nav",
        "incant_localization",
    ] {
        let dir = root.join("crates").join(name);
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
