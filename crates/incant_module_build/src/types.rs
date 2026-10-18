use crate::{Error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceQuery {
    pub package: String,
    pub entry: String,
    pub expected_revision: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildRequest {
    pub snapshot_id: String,
    pub expected_revision: u64,
    pub description: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobQuery {
    pub snapshot_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobStatus {
    pub snapshot_id: String,
    pub build_id: Option<String>,
    pub state: String,
    pub candidate_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePackage {
    pub entry: String,
    pub files: Vec<SourceFile>,
}
impl SourcePackage {
    pub fn digest(&self) -> Result<String> {
        relative(&self.entry, true)?;
        if self.files.is_empty() || self.files.len() > 32 {
            return Err(Error::Invalid("source file count"));
        }
        let mut rows = self.files.iter().collect::<Vec<_>>();
        rows.sort_by(|a, b| a.path.cmp(&b.path));
        let mut total = 0;
        let mut names = std::collections::BTreeSet::new();
        let mut hash = Sha256::new();
        hash.update(b"incant-rust-source-package-v1\0");
        hash.update((self.entry.len() as u64).to_le_bytes());
        hash.update(self.entry.as_bytes());
        for row in rows {
            relative(&row.path, true)?;
            if !names.insert(row.path.to_ascii_lowercase()) || row.text.len() > 65536 {
                return Err(Error::Invalid("source alias/size"));
            }
            total += row.text.len();
            if total > 256 * 1024 {
                return Err(Error::Invalid("source byte count"));
            }
            hash.update((row.path.len() as u64).to_le_bytes());
            hash.update(row.path.as_bytes());
            hash.update((row.text.len() as u64).to_le_bytes());
            hash.update(row.text.as_bytes());
        }
        if !self.files.iter().any(|f| f.path == self.entry) {
            return Err(Error::Invalid("missing entry"));
        }
        Ok(format!("{:x}", hash.finalize()))
    }
}
pub(crate) fn relative(value: &str, rust: bool) -> Result<()> {
    if value.is_empty()
        || value.len() > 1024
        || value.split('/').count() > 8
        || value.split('/').any(|p| {
            p.is_empty()
                || p.starts_with('.')
                || p.len() > 128
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
        || (rust
            && (!value.ends_with(".rs")
                || value.split('/').any(|p| p.eq_ignore_ascii_case("build.rs"))))
    {
        return Err(Error::Invalid("portable bounded Rust source path required"));
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceView {
    /// Present only for the additive registered-ID protocol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_source: Option<RegisteredSource>,
    pub snapshot_id: String,
    pub revision: u64,
    pub source_digest: String,
    pub untrusted_source_data: SourcePackage,
}
#[derive(Clone, Debug, Serialize)]
pub struct BuildResult {
    pub build_id: String,
    pub candidate_id: String,
    pub source_digest: String,
    pub untrusted_diagnostics: String,
    pub provenance: serde_json::Value,
}
/// Supplied by the engine's existing actor boundary, never deserialized from tools.
#[derive(Clone, Debug, Serialize)]
pub struct Actor {
    pub origin: String,
    pub actor: String,
    pub model: Option<String>,
    pub conversation_id: Option<String>,
}

/// Trusted adapter authority, never deserialized from model/tool arguments.
/// A snapshot belongs to one command-bus lifetime as well as a registration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegisteredSource {
    pub bus_session_id: String,
    pub module_id: String,
    pub path: String,
    pub version: incant_source::SourceVersion,
}
impl RegisteredSource {
    pub(crate) fn query(&self, revision: u64) -> Result<SourceQuery> {
        for id in [&self.bus_session_id, &self.module_id] {
            ulid::Ulid::from_string(id).map_err(|_| Error::Invalid("registered source ULID"))?;
        }
        incant_source::validate_rust_path(&self.path)
            .map_err(|_| Error::Invalid("registered Rust path"))?;
        if matches!(self.version, incant_source::SourceVersion::Missing) {
            return Err(Error::Invalid(
                "registered source is missing; write it first",
            ));
        }
        let (package, entry) = self
            .path
            .rsplit_once('/')
            .ok_or(Error::Invalid("registered package"))?;
        Ok(SourceQuery {
            package: package.into(),
            entry: entry.into(),
            expected_revision: revision,
        })
    }
    pub(crate) fn matches_entry(&self, source: &SourcePackage) -> bool {
        self.path
            .rsplit_once('/')
            .is_some_and(|(_, entry)| entry == source.entry)
            && source
                .files
                .iter()
                .find(|file| file.path == source.entry)
                .is_some_and(|file| {
                    incant_source::SourceVersion::of(Some(&file.text)) == self.version
                })
    }
}
