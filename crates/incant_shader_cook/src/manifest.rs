use crate::{Bundle, CONTRACT, CookError, Result};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};

/// Write deterministic build artifacts and a private Rust manifest, never an
/// externally supplied artifact loader. Publication completes with the manifest.
pub fn write_bundle(bundle: &Bundle, directory: &Path) -> Result<()> {
    fs::create_dir_all(directory)?;
    if bundle.artifacts.len() != 6 {
        return Err(CookError::Contract("artifact count"));
    }
    let mut generated = format!(
        "const MANIFEST_CONTRACT: &str = {CONTRACT:?};\nconst SOURCE_HASH: [u8; 32] = {:?};\nconst PRODUCER_HASH: [u8; 32] = {:?};\nconst ARTIFACTS: [Artifact; 6] = [\n",
        bundle.source_hash, bundle.producer_hash
    );
    for artifact in &bundle.artifacts {
        let file = format!("{:?}-{:?}.shader", artifact.backend, artifact.stage);
        fs::write(directory.join(&file), &artifact.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&artifact.bytes).into();
        writeln!(generated, "Artifact {{ backend: wgpu::Backend::{:?}, stage: Stage::{:?}, entry: {:?}, hash: {:?}, bytes: include_bytes!({:?}) }},", artifact.backend, artifact.stage, artifact.entry, hash, file).expect("writing String");
    }
    generated.push_str("];\n");
    fs::write(directory.join("manifest.rs"), generated)?;
    Ok(())
}
