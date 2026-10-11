//! Only this engine-vetted build script runs. Project input is source data;
//! supplied Rust, build scripts and bytecode caches are never included.
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

fn bounded(path: &Path, limit: u64) -> Result<String, Box<dyn std::error::Error>> {
    let display = path.to_str().ok_or("source path must be UTF-8")?;
    if display.chars().any(char::is_control) {
        return Err("control character in source path".into());
    }
    println!("cargo:rerun-if-changed={display}");
    let mut text = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err("script cook input exceeds limit".into());
    }
    Ok(text)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for key in [
        "INCANT_NATIVE_SCRIPT_SOURCE",
        "INCANT_NATIVE_SCRIPT_MAP",
        "INCANT_NATIVE_SCRIPT_NAME",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let host = env::var("HOST")?;
    let target = env::var("TARGET")?;
    if host != target {
        return Err("embedded game player currently requires native host == target; cross-target export is unverified".into());
    }
    let path = env::var_os("INCANT_NATIVE_SCRIPT_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fixtures/game.js"));
    let source = bounded(&path, 1_000_000)?;
    let map = env::var_os("INCANT_NATIVE_SCRIPT_MAP")
        .map(|path| bounded(Path::new(&path), 4 * 1024 * 1024))
        .transpose()?;
    let name =
        env::var("INCANT_NATIVE_SCRIPT_NAME").unwrap_or_else(|_| "embedded-behavior.js".into());
    let artifact = incant_cook::cook_script(&name, &source, map.as_deref(), &target)?;
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR missing")?);
    // Always generate from this invocation's source compilation. Existing
    // executable caches in either the project or OUT_DIR are never trusted.
    fs::write(out.join("behavior.rs"), artifact.rust_module)?;
    fs::write(out.join("script-symbols.json"), artifact.symbol_manifest)?;
    if let Some(map) = artifact.source_map {
        fs::write(out.join("behavior.js.map"), map)?;
    } else if out.join("behavior.js.map").exists() {
        fs::remove_file(out.join("behavior.js.map"))?;
    }
    Ok(())
}
