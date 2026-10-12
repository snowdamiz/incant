//! Data-only captured inputs. Every load revalidates the original and regenerates
//! metering, so a cache never supplies trusted native code or a metering bypass.
use crate::{CookError, error, prepare};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const MAX_MODULE_BYTES: usize = 1024 * 1024;
pub const MAX_CAPTURE_BYTES: usize = 64 * 1024 * 1024;
const POLICY: &str = "incant-wasm-capture-v1-wabt-1.0.42-fixed-memory-meter-v1";
const REGISTRY: &[u8] = include_bytes!("../../../sdk/abi/native-v1.json");
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedModule {
    policy: String,
    registry_sha256: String,
    input_sha256: String,
    prepared_sha256: String,
    original: Vec<u8>,
    prepared: Vec<u8>,
}
impl CapturedModule {
    pub fn capture(original: &[u8]) -> Result<Self, CookError> {
        let prepared = prepare(original)?;
        Ok(Self {
            policy: POLICY.into(),
            registry_sha256: sha(REGISTRY),
            input_sha256: sha(original),
            prepared_sha256: sha(&prepared),
            original: original.to_vec(),
            prepared,
        })
    }
    pub fn original(&self) -> &[u8] {
        &self.original
    }
    pub fn prepared(&self) -> &[u8] {
        &self.prepared
    }
    pub fn input_sha256(&self) -> &str {
        &self.input_sha256
    }
    pub fn prepared_sha256(&self) -> &str {
        &self.prepared_sha256
    }
    pub fn registry_sha256(&self) -> &str {
        &self.registry_sha256
    }
    pub fn fingerprint(&self) -> String {
        sha(format!(
            "{}\0{}\0{}\0{}",
            self.policy, self.registry_sha256, self.input_sha256, self.prepared_sha256
        )
        .as_bytes())
    }
    pub fn store(&self, cache: &Path) -> Result<bool, CookError> {
        let key = self.fingerprint();
        let path = cache.join(format!("{key}.wasm.json"));
        if path.symlink_metadata().is_ok() {
            Self::load(cache, &key)?;
            return Ok(true);
        }
        let bytes = serde_json::to_vec(self).map_err(error)?;
        if bytes.len() > MAX_CAPTURE_BYTES {
            return Err(error("captured module exceeds size limit"));
        }
        // Cache writes never overwrite existing content, including symlinks.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(error)?;
        file.write_all(&bytes).map_err(error)?;
        file.sync_all().map_err(error)?;
        Ok(false)
    }
    pub fn load(cache: &Path, fingerprint: &str) -> Result<Self, CookError> {
        if fingerprint.len() != 64
            || !fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(error("invalid captured module fingerprint"));
        }
        let path = cache.join(format!("{fingerprint}.wasm.json"));
        let metadata = path.symlink_metadata().map_err(error)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > MAX_CAPTURE_BYTES as u64
        {
            return Err(error("captured module must be a bounded regular file"));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(error)?
            .take(MAX_CAPTURE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(error)?;
        if bytes.len() > MAX_CAPTURE_BYTES {
            return Err(error("captured module exceeds size limit"));
        }
        let captured: Self = serde_json::from_slice(&bytes).map_err(error)?;
        if captured.policy != POLICY
            || captured.registry_sha256 != sha(REGISTRY)
            || captured.input_sha256 != sha(&captured.original)
            || captured.prepared_sha256 != sha(&captured.prepared)
            || captured.fingerprint() != fingerprint
        {
            return Err(error("captured module identity mismatch"));
        }
        if prepare(&captured.original)? != captured.prepared {
            return Err(error("captured module prepared input mismatch"));
        }
        Ok(captured)
    }
}
/// Resolve only real project-owned cache directories. Source deletion does not
/// invalidate a captured module; reimport explicitly replaces its fingerprint.
pub fn project_cache(root: &Path, create: bool) -> Result<PathBuf, CookError> {
    let root = root.canonicalize().map_err(error)?;
    let mut path = root.clone();
    for part in [".incant", "cache", "wasm"] {
        path.push(part);
        if create {
            match fs::create_dir(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(error(e)),
            }
        }
        let metadata = path.symlink_metadata().map_err(error)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || !path.canonicalize().map_err(error)?.starts_with(&root)
        {
            return Err(error("Wasm cache must use real project directories"));
        }
    }
    Ok(path)
}
