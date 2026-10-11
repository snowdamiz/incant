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
        Path::new("src/queries.rs").into(),
        Path::new("src/query_values.rs").into(),
        Path::new("src/query_values_2d.rs").into(),
        Path::new("src/schedule.rs").into(),
        Path::new("src/timers.rs").into(),
        Path::new("src/logging.rs").into(),
        Path::new("src/session.rs").into(),
        Path::new("src/program.rs").into(),
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
    println!(
        "cargo:rustc-env=INCANT_SCRIPT_TARGET={}",
        env::var("TARGET").unwrap()
    );
}
