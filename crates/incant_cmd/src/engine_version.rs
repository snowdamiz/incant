//! Read-only review of a metadata pin. This is not a file migration capability.
use crate::{Command, CommandBus, CommandError};
use incant_doc::{EngineRequirement, Id, Project};
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_ENGINE_REVIEW_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum EngineReviewError {
    #[error("engine pin review exceeds the 32 MiB input or canonical output limit")]
    Limit,
    #[error("source project differs from the currently opened document; reload or save explicitly")]
    SourceChanged,
    #[error("engine pin review applies only to a legacy project without an engine declaration")]
    AlreadyDeclared,
}

/// Review data only. The embedded ordinary command can pin an in-memory
/// document through `execute`, but cannot authorize a backup or filesystem edit.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct EnginePinReview {
    pub project_id: Id,
    pub expected_revision: u64,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub previous_requirement: Option<EngineRequirement>,
    pub target_requirement: EngineRequirement,
    pub commands: Vec<Command>,
    pub candidate_sha256: String,
    pub candidate_text: String,
    pub file_migration_supported: bool,
}

impl CommandBus {
    pub fn review_engine_pin(
        &self,
        source: &str,
        expected_revision: u64,
    ) -> Result<EnginePinReview, CommandError> {
        self.ensure_sources_ready()?;
        if expected_revision != self.revision {
            return Err(CommandError::Conflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if source.len() > MAX_ENGINE_REVIEW_BYTES {
            return Err(EngineReviewError::Limit.into());
        }
        // Parsing performs format and required-engine checks before review.
        // Whitespace can differ: retain a digest of the exact original bytes.
        let mut candidate = Project::from_text(source)?;
        if candidate != self.project {
            return Err(EngineReviewError::SourceChanged.into());
        }
        if candidate.engine_version.is_some() {
            return Err(EngineReviewError::AlreadyDeclared.into());
        }
        let requirement = EngineRequirement::exact_current();
        candidate.engine_version = Some(requirement.clone());
        let candidate_text = candidate.canonical_text()?;
        if candidate_text.len() > MAX_ENGINE_REVIEW_BYTES {
            return Err(EngineReviewError::Limit.into());
        }
        Ok(EnginePinReview {
            project_id: candidate.id,
            expected_revision,
            source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
            source_bytes: source.len(),
            previous_requirement: None,
            target_requirement: requirement.clone(),
            commands: vec![Command::SetEngineVersion { requirement }],
            candidate_sha256: format!("{:x}", Sha256::digest(candidate_text.as_bytes())),
            candidate_text,
            file_migration_supported: false,
        })
    }
}
