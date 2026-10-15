//! Portable logical catalog identity shared by GPU-free recordings and renderers.
//! Native cache/device identities are separate. Directory watches include future
//! source additions; no git command, generated output, or project data is read.
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn inventory(root: &Path, path: &Path, paths: &mut Vec<PathBuf>) {
    let metadata = fs::symlink_metadata(path).expect("catalog source metadata");
    assert!(!metadata.file_type().is_symlink(), "catalog source symlink");
    println!("cargo:rerun-if-changed={}", path.display());
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("catalog source directory") {
            inventory(root, &entry.expect("catalog source entry").path(), paths);
        }
    } else {
        assert!(metadata.is_file(), "catalog source type");
        paths.push(
            path.strip_prefix(root)
                .expect("catalog source root")
                .to_owned(),
        );
    }
}
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let mut paths = Vec::new();
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/incant_render/build.rs",
        "crates/incant_render/build",
        "crates/incant_render/Cargo.toml",
        "crates/incant_render/src",
        "crates/incant_render_data/build.rs",
        "crates/incant_render_data/Cargo.toml",
        "crates/incant_render_data/src",
        "crates/incant_assets_runtime/Cargo.toml",
        "crates/incant_assets_runtime/src",
        "crates/incant_shader_cook/Cargo.toml",
        "crates/incant_shader_cook/src",
    ] {
        inventory(&root, &root.join(path), &mut paths);
    }
    paths.sort();
    let mut hash = Sha256::new();
    hash.update(b"Incant ModelMaterialV1 logical source catalog\0");
    for path in paths {
        let name = path
            .to_str()
            .expect("UTF-8 catalog path")
            .replace('\\', "/");
        let bytes = fs::read(root.join(&path)).expect("catalog source bytes");
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    let digest: [u8; 32] = hash.finalize().into();
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("material_catalog.rs");
    fs::write(
        output,
        format!("pub const MATERIAL_CATALOG_SHA256: [u8; 32] = {digest:?};\n"),
    )
    .expect("catalog identity output");
}
