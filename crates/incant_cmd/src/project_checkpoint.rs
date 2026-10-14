//! File checkpoints publish the current logical document without an undo step.
//! Version four is separate from the exact legacy-pin transaction in version three.
use super::*;
use incant_source::ProjectEdit;
use journal::{Action, Change};
use sha2::{Digest, Sha256};

/// The application shares this closed request with RPC and native IPC. No path,
/// candidate text, backup, command or actor can be supplied through this boundary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectFileRequest {
    ReviewPin {
        expected_revision: u64,
    },
    ApplyPin {
        expected_revision: u64,
        review_id: Id,
    },
    Save {
        expected_revision: u64,
    },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ProjectFileResponse {
    ReviewPin { review: Box<ProjectPinReview> },
    ApplyPin { transaction_id: Id, revision: u64 },
    Save { receipt: ProjectSaveReceipt },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProjectSaveReceipt {
    pub revision: u64,
    pub changed: bool,
    pub source_sha256: String,
    pub candidate_sha256: String,
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

impl CommandBus {
    pub fn has_project_file(&self) -> bool {
        self.project_file.is_some()
    }
    /// A watcher may observe readiness without reading arbitrary paths or
    /// accepting an external same-byte replacement as an owned publication.
    pub fn check_project_file(&self) -> Result<(), CommandError> {
        self.ensure_sources_ready()?;
        self.project_file
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .read_admitted()?;
        Ok(())
    }
    pub fn project_file_request(
        &mut self,
        request: ProjectFileRequest,
        actor: Actor,
    ) -> Result<ProjectFileResponse, CommandError> {
        match request {
            ProjectFileRequest::ReviewPin { expected_revision } => {
                self.check_revision(Some(expected_revision))?;
                Ok(ProjectFileResponse::ReviewPin {
                    review: Box::new(self.review_project_file_pin()?),
                })
            }
            ProjectFileRequest::ApplyPin {
                expected_revision,
                review_id,
            } => {
                self.check_revision(Some(expected_revision))?;
                let transaction_id = self.apply_project_file_pin(&review_id, actor)?.id.clone();
                Ok(ProjectFileResponse::ApplyPin {
                    transaction_id,
                    revision: self.revision,
                })
            }
            ProjectFileRequest::Save { expected_revision } => Ok(ProjectFileResponse::Save {
                receipt: self.save_project_file(expected_revision, actor)?,
            }),
        }
    }
    /// Same bus, same admitted node, same exact durable CRDT checkpoint. Save
    /// changes neither command history nor revision and never imports file edits.
    pub fn save_project_file(
        &mut self,
        expected_revision: u64,
        actor: Actor,
    ) -> Result<ProjectSaveReceipt, CommandError> {
        self.check_revision(Some(expected_revision))?;
        self.check_project_file()?;
        let before = self
            .project_file
            .as_ref()
            .expect("checked file")
            .read_admitted()?
            .text;
        let after = self.project.canonical_text()?;
        let receipt = ProjectSaveReceipt {
            revision: self.revision,
            changed: before != after,
            source_sha256: hash(&before),
            candidate_sha256: hash(&after),
        };
        let operation = crate::project_file::operation_id();
        actor.provenance(&operation)?;
        if receipt.changed {
            let edit = ProjectEdit {
                project_id: self.project.id.clone(),
                before,
                after,
            };
            let crdt = self
                .journal
                .as_ref()
                .ok_or(SourceError::Unavailable)?
                .checkpoint_bytes()?;
            let prepare = Action::CheckpointPrepare {
                operation: operation.clone(),
                edit: edit.clone(),
                revision: self.revision,
                actor,
                change: None,
            };
            self.publish_project(prepare, &edit, &operation, &crdt, &crdt, true)?;
        }
        self.project_review = None;
        Ok(receipt)
    }
    pub(super) fn record_checkpoint_history(
        &mut self,
        change: Change,
        original: &ProjectEdit,
        next: Option<&CollaborativeDocument>,
    ) -> Result<(), CommandError> {
        self.check_project_file()?;
        let snapshot = self
            .project_file
            .as_ref()
            .expect("checked file")
            .read_admitted()?;
        // Keep the reviewed exact preimage formatting when undoing the pin. The
        // current disk may be an earlier/later checkpoint after ordinary edits.
        let edit = ProjectEdit {
            project_id: self.project.id.clone(),
            before: snapshot.text,
            after: original.after.clone(),
        };
        let current = self
            .journal
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .checkpoint_bytes()?;
        let desired = next.ok_or(SourceError::Unavailable)?.export()?;
        let operation = crate::project_file::operation_id();
        let actor = match &change {
            Change::Undo { .. } => {
                &self
                    .history
                    .last()
                    .ok_or(CommandError::Empty("undo"))?
                    .actor
            }
            Change::Redo { .. } => &self.redo.last().ok_or(CommandError::Empty("redo"))?.actor,
            Change::Apply { .. } => {
                return Err(CommandError::Invalid(
                    "checkpoint history must undo or redo".into(),
                ));
            }
        }
        .clone();
        let prepare = Action::CheckpointPrepare {
            operation: operation.clone(),
            edit: edit.clone(),
            revision: self.revision,
            actor,
            change: Some(change),
        };
        self.publish_project(prepare, &edit, &operation, &current, &desired, true)
    }
}

/// Scan validation is read-only and checks the original history action. The
/// checkpoint preimage need not equal live logical state, but must be a validated
/// earlier journal checkpoint (checked by the caller).
pub(super) fn validate_prepare(
    bus: &CommandBus,
    edit: &ProjectEdit,
    change: &Option<Change>,
    revision: u64,
    actor: &Actor,
    operation: &str,
) -> Result<(), CommandError> {
    edit.validate()?;
    actor.provenance(operation)?;
    if revision != bus.revision || edit.project_id != bus.project.id {
        return Err(CommandError::Corrupt(
            "checkpoint identity or revision mismatch".into(),
        ));
    }
    let expected = match change {
        None => bus.project.canonical_text()?,
        Some(change @ (Change::Undo { .. } | Change::Redo { .. })) => {
            journal::scan::project_edit(bus, change)
                .ok_or_else(|| {
                    CommandError::Corrupt("checkpoint history lacks original pin".into())
                })?
                .after
        }
        Some(Change::Apply { .. }) => {
            return Err(CommandError::Corrupt(
                "checkpoint cannot apply a transaction".into(),
            ));
        }
    };
    if edit.after != expected {
        return Err(CommandError::Corrupt("checkpoint target mismatch".into()));
    }
    Ok(())
}

/// The same wire definitions are advertised by native IPC and JSON-RPC.
pub fn project_file_schemas() -> std::collections::BTreeMap<String, Value> {
    std::collections::BTreeMap::from([
        (
            "ProjectPinReview".into(),
            serde_json::json!(schemars::schema_for!(ProjectPinReview)),
        ),
        (
            "ProjectFileRequest".into(),
            serde_json::json!(schemars::schema_for!(ProjectFileRequest)),
        ),
        (
            "ProjectFileResponse".into(),
            serde_json::json!(schemars::schema_for!(ProjectFileResponse)),
        ),
        (
            "ProjectSaveReceipt".into(),
            serde_json::json!(schemars::schema_for!(ProjectSaveReceipt)),
        ),
    ])
}
