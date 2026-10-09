use crate::{AssetError, MAX_SOURCE_BYTES, Result, invalid, sha256};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub path: String,
    pub sha256: String,
}

/// One bounded snapshot of a source file and its local dependencies. URI resolution
/// allows sibling directories inside the project, but never network or outside files.
pub struct SourceSet {
    root: PathBuf,
    files: BTreeMap<String, Vec<u8>>,
    total: usize,
}
impl SourceSet {
    pub fn new(root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(invalid("asset root is not a directory"));
        }
        Ok(Self {
            root,
            files: BTreeMap::new(),
            total: 0,
        })
    }
    pub fn read(&mut self, relative: &Path) -> Result<Vec<u8>> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, Component::Prefix(_)))
        {
            return Err(invalid("asset path must be relative"));
        }
        // Normalize lexical parents before following symlinks. Both the logical
        // dependency name and its resolved file must remain in the project.
        let mut normalized = PathBuf::new();
        for c in relative.components() {
            match c {
                Component::Normal(v) => normalized.push(v),
                Component::CurDir => {}
                Component::ParentDir if normalized.pop() => {}
                _ => return Err(invalid("asset path escapes the project")),
            }
        }
        let name = normalized
            .to_str()
            .ok_or_else(|| invalid("asset path is not UTF-8"))?
            .replace('\\', "/");
        if name.is_empty()
            || name.contains(':')
            || name.contains('\0')
            || normalized.to_string_lossy().contains('\\')
        {
            return Err(invalid("asset path is not portable"));
        }
        if let Some(bytes) = self.files.get(&name) {
            return Ok(bytes.clone());
        }
        let path = self.root.join(&normalized).canonicalize()?;
        if !path.starts_with(&self.root) {
            return Err(invalid("asset symlink escapes the project"));
        }
        if self.files.len() >= 1024 {
            return Err(AssetError::Limit("dependency count"));
        }
        let file = File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(invalid("asset dependency is not a regular file"));
        }
        let remaining = MAX_SOURCE_BYTES - self.total;
        if file.metadata()?.len() > remaining as u64 {
            return Err(AssetError::Limit("source bytes"));
        }
        let mut bytes = Vec::new();
        file.take(remaining as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > remaining {
            return Err(AssetError::Limit("source bytes"));
        }
        self.total += bytes.len();
        self.files.insert(name, bytes.clone());
        Ok(bytes)
    }
    pub fn uri(&mut self, source: &Path, uri: &str) -> Result<Vec<u8>> {
        if let Some(data) = uri.strip_prefix("data:") {
            let (header, encoded) = data
                .split_once(',')
                .ok_or_else(|| invalid("malformed data URI"))?;
            if !matches!(
                header,
                "application/octet-stream;base64" | "application/gltf-buffer;base64"
            ) {
                return Err(AssetError::Unsupported("buffer data URI media type".into()));
            }
            if encoded.len() > (MAX_SOURCE_BYTES - self.total) / 3 * 4 + 4 {
                return Err(AssetError::Limit("source bytes"));
            }
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| invalid("invalid base64 buffer"))?;
            self.total = self
                .total
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_SOURCE_BYTES)
                .ok_or(AssetError::Limit("source bytes"))?;
            return Ok(bytes);
        }
        let path = percent_encoding::percent_decode_str(uri)
            .decode_utf8()
            .map_err(|_| invalid("URI is not UTF-8"))?;
        if path.contains([':', '?', '#', '\\', '\0']) || path.starts_with('/') {
            return Err(invalid("only project-local resource URIs are supported"));
        }
        self.read(&source.parent().unwrap_or(Path::new("")).join(path.as_ref()))
    }
    pub fn dependencies(&self) -> Vec<Dependency> {
        self.files
            .iter()
            .map(|(path, bytes)| Dependency {
                path: path.clone(),
                sha256: sha256(bytes),
            })
            .collect()
    }
    /// Version salt makes importer/codec changes invalidate older cooked entries.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(b"incant-static-gltf-v1-meshopt-0.6.2\0");
        for d in self.dependencies() {
            hash.update((d.path.len() as u64).to_le_bytes());
            hash.update(d.path.as_bytes());
            hash.update(d.sha256.as_bytes());
        }
        format!("{:x}", hash.finalize())
    }
}
