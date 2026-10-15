//! Deterministic build evidence only. Never loaded by the native renderer.
use crate::{Backend, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(result, "{byte:02x}").expect("writing String");
    }
    result
}

pub(crate) fn write<'a>(
    directory: &Path,
    contract: &str,
    source: &[u8; 32],
    producer: &[u8; 32],
    artifacts: impl IntoIterator<Item = (Backend, &'static str, &'a str, &'a [u8])>,
) -> Result<()> {
    let artifacts = artifacts
        .into_iter()
        .map(|(backend, stage, entry, bytes)| {
            json!({
                "backend": format!("{backend:?}"), "stage": stage, "entry": entry,
                "file": format!("{backend:?}-{stage}.shader"),
                "bytes": bytes.len(), "sha256": hex(&Sha256::digest(bytes)),
            })
        })
        .collect::<Vec<_>>();
    let manifest = fs::read(directory.join("manifest.rs"))?;
    let receipt = json!({
        "format": "incant-fixed-shader-build", "version": 1, "contract": contract,
        "source_sha256": hex(source), "producer_sha256": hex(producer),
        "manifest_sha256": hex(&Sha256::digest(&manifest)), "artifacts": artifacts,
    });
    let bytes = serde_json::to_vec(&receipt).map_err(std::io::Error::other)?;
    fs::write(directory.join("audit.json"), bytes)?;
    Ok(())
}
