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
const POLICY: &str = "incant-wasm-capture-v1-wabt-1.0.42-fixed-memory-meter-state-v2";
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
        // Interrupted writes can leave only an unreferenced private sibling,
        // never a partial immutable fingerprint entry. Publication cannot replace
        // an existing file or symlink, including a concurrently published entry.
        let mut file = stage(cache)?;
        file.write_all(&bytes).map_err(error)?;
        file.as_file().sync_all().map_err(error)?;
        Self::publish(cache, &key, file)
    }
    fn publish(cache: &Path, key: &str, file: tempfile::NamedTempFile) -> Result<bool, CookError> {
        match file.persist_noclobber(cache.join(format!("{key}.wasm.json"))) {
            Ok(_) => Ok(false),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                // A winning concurrent writer is a hit only after the ordinary
                // bounded identity/metering checks. Corrupt entries stay refused.
                Self::load(cache, key)?;
                Ok(true)
            }
            Err(e) => Err(error(e.error)),
        }
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

fn stage(cache: &Path) -> Result<tempfile::NamedTempFile, CookError> {
    tempfile::Builder::new()
        .prefix(".incant-wasm-")
        .tempfile_in(cache)
        .map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn module() -> CapturedModule {
        CapturedModule::capture(
            &wat::parse_str(
                r#"(module
          (memory (export "memory") 1)
          (func (export "incant_abi") (result i32) i32.const 1)
          (func (export "update") (param i64 f64)))"#,
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn interrupted_partial_staging_cannot_poison_the_fingerprint() {
        let root = tempfile::tempdir().unwrap();
        let cache = project_cache(root.path(), true).unwrap();
        let captured = module();
        let key = captured.fingerprint();
        let mut partial = stage(&cache).unwrap();
        partial.write_all(b"{\"policy\":").unwrap();
        partial.as_file().sync_all().unwrap();
        // Keep exactly the incomplete sibling a terminated process might leave.
        let abandoned = partial.into_temp_path().keep().unwrap();
        assert!(CapturedModule::load(&cache, &key).is_err());
        assert!(!cache.join(format!("{key}.wasm.json")).exists());
        assert!(!captured.store(&cache).unwrap());
        assert_eq!(
            CapturedModule::load(&cache, &key).unwrap().original(),
            captured.original()
        );
        assert!(captured.store(&cache).unwrap());
        assert_eq!(fs::read(abandoned).unwrap(), b"{\"policy\":");
    }

    #[test]
    fn concurrent_identical_publication_is_one_creation_then_validated_hits() {
        let root = tempfile::tempdir().unwrap();
        let cache = project_cache(root.path(), true).unwrap();
        let captured = module();
        let key = captured.fingerprint();
        let bytes = serde_json::to_vec(&captured).unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let mut writers = Vec::new();
        // All writers finish their private file before any can publish. This
        // deterministically exercises the racing destination path, not just the
        // existing-file shortcut at the beginning of store().
        for _ in 0..8 {
            let (cache, key, bytes, barrier) =
                (cache.clone(), key.clone(), bytes.clone(), barrier.clone());
            writers.push(std::thread::spawn(move || {
                let mut staged = stage(&cache).unwrap();
                staged.write_all(&bytes).unwrap();
                staged.as_file().sync_all().unwrap();
                barrier.wait();
                CapturedModule::publish(&cache, &key, staged).unwrap()
            }));
        }
        let hits: Vec<_> = writers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(hits.iter().filter(|hit| !**hit).count(), 1);
        assert_eq!(hits.iter().filter(|hit| **hit).count(), 7);
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
        assert_eq!(
            CapturedModule::load(&cache, &key).unwrap().prepared(),
            captured.prepared()
        );
    }

    #[test]
    fn racing_corrupt_destination_is_refused_without_overwrite_or_temp_leak() {
        let root = tempfile::tempdir().unwrap();
        let cache = project_cache(root.path(), true).unwrap();
        let captured = module();
        let key = captured.fingerprint();
        let mut staged = stage(&cache).unwrap();
        staged
            .write_all(&serde_json::to_vec(&captured).unwrap())
            .unwrap();
        staged.as_file().sync_all().unwrap();
        let path = cache.join(format!("{key}.wasm.json"));
        fs::write(&path, b"incomplete existing entry").unwrap();
        assert!(CapturedModule::publish(&cache, &key, staged).is_err());
        assert_eq!(fs::read(path).unwrap(), b"incomplete existing entry");
        assert_eq!(fs::read_dir(cache).unwrap().count(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn racing_symlink_is_refused_without_touching_its_target() {
        let root = tempfile::tempdir().unwrap();
        let cache = project_cache(root.path(), true).unwrap();
        let captured = module();
        let key = captured.fingerprint();
        let bytes = serde_json::to_vec(&captured).unwrap();
        let mut staged = stage(&cache).unwrap();
        staged.write_all(&bytes).unwrap();
        staged.as_file().sync_all().unwrap();
        let outside = root.path().join("outside");
        fs::write(&outside, &bytes).unwrap();
        let path = cache.join(format!("{key}.wasm.json"));
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(CapturedModule::publish(&cache, &key, staged).is_err());
        assert!(captured.store(&cache).is_err());
        assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read(outside).unwrap(), bytes);
        assert_eq!(fs::read_dir(cache).unwrap().count(), 1);
    }
}
