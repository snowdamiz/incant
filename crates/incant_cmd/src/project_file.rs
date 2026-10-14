//! Host-bound exact-current pinning. No application RPC/CLI endpoint is wired.
//! The only file capability is constructed from the host's opened project path.
use super::*;
use incant_source::{OpenedProject, ProjectEdit, ProjectSnapshot};
use journal::{Action, Change, ProjectRecord, ProjectStatus, scan};
use std::{io::Read, path::Path};

pub(crate) struct PendingReview {
    id: Id,
    revision: u64,
    snapshot: ProjectSnapshot,
    edit: ProjectEdit,
}
/// Returned review data is not a file capability. A bus retains one matching
/// review internally; arbitrary IDs or deserialized contents grant no authority.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProjectPinReview {
    pub review_id: Id,
    pub expected_revision: u64,
    pub source_sha256: String,
    pub candidate_sha256: String,
    pub candidate_text: String,
    pub command: Command,
}
pub(super) fn validate_transaction(tx: &Transaction) -> Result<(), CommandError> {
    let Some(edit) = &tx.project_edit else {
        return Ok(());
    };
    edit.validate()?;
    let target = incant_doc::EngineRequirement::exact_current();
    if tx.source_edit.is_some()
        || tx.before.engine_version.is_some()
        || tx.commands
            != [Command::SetEngineVersion {
                requirement: target.clone(),
            }]
        || edit.project_id != tx.before.id
        || tx.before.id != tx.after.id
        || Project::from_text(&edit.before)? != tx.before
        || Project::from_text(&edit.after)? != tx.after
    {
        return Err(CommandError::Corrupt(
            "project edit is not the exact reviewed legacy pin".into(),
        ));
    }
    let mut expected = tx.before.clone();
    expected.engine_version = Some(target);
    if tx.after != expected || edit.after != expected.canonical_text()? {
        return Err(CommandError::Corrupt(
            "project pin changes unrelated content".into(),
        ));
    }
    Ok(())
}
impl CommandBus {
    /// Trusted host API, not a deserializable path tool. Opens the derived journal
    /// before normal document parsing, including a crash with no document node.
    pub fn open_project_file(path: impl AsRef<Path>) -> Result<Self, CommandError> {
        let (mut opened, mut file) = OpenedProject::lock(path.as_ref())?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take((journal::MAX_PROJECT_JOURNAL_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > journal::MAX_PROJECT_JOURNAL_BYTES {
            return Err(CommandError::Invalid(
                "project journal exceeds 256 MiB".into(),
            ));
        }
        // Initialize is bounded and checked before it can provide an identity.
        let initial = match scan::initialize(&bytes)? {
            Some(initial) => initial,
            None => {
                if !bytes.is_empty() {
                    return Err(CommandError::Corrupt(
                        "nonempty journal lacks a complete Initialize".into(),
                    ));
                }
                // A fresh initial journal cannot authorize recovery. The
                // original file must exist and independently validate.
                let snapshot = opened.read()?.ok_or(SourceError::RecoveryRequired)?;
                Project::from_text(&snapshot.text)?
            }
        };
        let result = scan::replay(file, &bytes, initial.clone(), true)?;
        let store = opened.source_store(&initial.id)?;
        let pending = opened.pending(&store)?;
        // Marker text is correspondence only. The complete journal chain and
        // logical checkpoint set were validated first. A pending preimage must
        // join that set before any rollback/cleanup filesystem effect.
        let text = if let Some(pending) = &pending {
            pending.edit.before.clone()
        } else {
            let snapshot = opened.read()?.ok_or(SourceError::RecoveryRequired)?;
            opened.admit_snapshot(&snapshot);
            snapshot.text
        };
        let checkpoint = Project::from_text(&text)?;
        if checkpoint.id != initial.id
            || !result
                .checkpoints
                .contains(&scan::project_digest(&checkpoint)?)
        {
            return Err(CommandError::Corrupt(
                "opened project is not a journal checkpoint".into(),
            ));
        }
        let scan::Replay {
            mut bus,
            last_source,
            last_project,
            complete,
            ..
        } = result;
        // Preflight marker-to-journal joins before truncating a torn tail or
        // executing unrelated script recovery.
        check_recovery_join(&bus, pending.as_ref(), last_project.as_ref())?;
        if let Some(p) = &pending {
            let status = last_project
                .as_ref()
                .filter(|r| r.operation == p.operation)
                .map(|r| r.status);
            if status == Some(ProjectStatus::Prepared) {
                opened.check_rollback(&store, p)?;
            } else {
                opened.verify_result(&store, p, status == Some(ProjectStatus::Committed))?;
            }
        }
        if pending.is_some() && last_source.as_ref().is_some_and(|r| !r.finished) {
            return Err(SourceError::RecoveryRequired.into());
        }
        bus.journal
            .as_mut()
            .expect("journal")
            .finish_read(complete, bytes.len())?;
        bus.initialize_journal()?;
        bus.recover_sources(store, last_source)?;
        bus.project_file = Some(opened);
        bus.recover_project(last_project)?;
        bus.ensure_sources_ready()?;
        bus.project_file
            .as_ref()
            .expect("host capability")
            .read_admitted()?;
        Ok(bus)
    }
    pub(super) fn initialize_journal(&mut self) -> Result<(), CommandError> {
        let current = self.export_crdt()?;
        self.journal
            .as_mut()
            .ok_or(SourceError::Unavailable)?
            .initialize(&self.project, &current)
    }
    /// At most one host-owned review; replacing it invalidates the previous ID.
    pub fn review_project_file_pin(&mut self) -> Result<ProjectPinReview, CommandError> {
        self.ensure_sources_ready()?;
        let file = self.project_file.as_ref().ok_or(SourceError::Unavailable)?;
        let snapshot = file.read_admitted()?;
        let review = self.review_engine_pin(&snapshot.text, self.revision)?;
        let id = new_id();
        self.project_review = Some(PendingReview {
            id: id.clone(),
            revision: self.revision,
            snapshot,
            edit: ProjectEdit {
                project_id: self.project.id.clone(),
                before: String::new(),
                after: review.candidate_text.clone(),
            },
        });
        let pending = self.project_review.as_mut().expect("review stored");
        pending.edit.before.clone_from(&pending.snapshot.text);
        Ok(ProjectPinReview {
            review_id: id,
            expected_revision: self.revision,
            source_sha256: review.source_sha256,
            candidate_sha256: review.candidate_sha256,
            candidate_text: review.candidate_text,
            command: review.commands.into_iter().next().expect("pin command"),
        })
    }
    /// Applies only the one internally retained review. The host may expose this
    /// after protocol qualification; no application endpoint is registered here.
    pub fn apply_project_file_pin(
        &mut self,
        review_id: &str,
        actor: Actor,
    ) -> Result<&Transaction, CommandError> {
        self.ensure_sources_ready()?;
        let pending = self
            .project_review
            .as_ref()
            .filter(|r| r.id == review_id)
            .ok_or_else(|| CommandError::Invalid("unknown or retired project pin review".into()))?;
        self.check_revision(Some(pending.revision))?;
        let file = self.project_file.as_ref().ok_or(SourceError::Unavailable)?;
        if file.read()?.as_ref() != Some(&pending.snapshot) {
            return Err(SourceError::Conflict.into());
        }
        // Revalidate document compatibility, exact source, and current revision;
        // no proposed payload from the caller is applied.
        self.review_engine_pin(&pending.snapshot.text, pending.revision)?;
        let pending = self.project_review.take().expect("checked review");
        let id = new_id();
        actor.provenance(&id)?;
        let transaction = Transaction {
            id,
            description: "Pin opened project to the current engine".into(),
            actor,
            commands: vec![Command::SetEngineVersion {
                requirement: incant_doc::EngineRequirement::exact_current(),
            }],
            before: self.project.clone(),
            after: Project::from_text(&pending.edit.after)?,
            source_edit: None,
            project_edit: Some(pending.edit),
        };
        validate_transaction(&transaction)?;
        self.commit_transaction(transaction)?;
        Ok(self.history.last().expect("committed pin"))
    }
    pub(super) fn record_project(
        &mut self,
        change: Change,
        edit: &ProjectEdit,
        next: Option<&CollaborativeDocument>,
    ) -> Result<(), CommandError> {
        let current = self.export_crdt()?;
        let desired = next.ok_or(SourceError::Unavailable)?.export()?;
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let file = self.project_file.as_mut().ok_or(SourceError::Unavailable)?;
        let snapshot = file.read_admitted()?;
        if snapshot.text != edit.before {
            return Err(SourceError::Conflict.into());
        }
        let operation = operation_id();
        let prepare = Action::ProjectPrepare {
            operation: operation.clone(),
            edit: edit.clone(),
            change,
        };
        let journal = self.journal.as_mut().ok_or(SourceError::Unavailable)?;
        journal.admit_project(&prepare, &operation, &current, &desired)?;
        let pending = match file.begin(store, &operation, edit, &snapshot) {
            Ok(pending) => pending,
            Err(e) => {
                store
                    .ensure_ready()
                    .map_err(|_| SourceError::RecoveryRequired)?;
                return Err(e.into());
            }
        };
        crash("marker");
        if let Err(e) = journal.append(prepare, &desired) {
            if journal.poisoned {
                return Err(SourceError::RecoveryRequired.into());
            }
            file.finish(store, &operation, edit, false)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(e);
        }
        crash("prepare");
        let installed = file
            .displace(store, &operation, edit)
            .and_then(|()| {
                crash("displaced");
                file.publish(store, &operation, edit)
            })
            .map_err(CommandError::from);
        crash("installed");
        let committed = installed.and_then(|()| {
            journal.append(
                Action::ProjectCommit {
                    operation: operation.clone(),
                },
                &desired,
            )
        });
        if let Err(e) = committed {
            if journal.poisoned {
                return Err(SourceError::RecoveryRequired.into());
            }
            file.rollback(store, &operation, edit)
                .map_err(|_| SourceError::RecoveryRequired)?;
            journal
                .append(
                    Action::ProjectAbort {
                        operation: operation.clone(),
                    },
                    &current,
                )
                .map_err(|_| SourceError::RecoveryRequired)?;
            file.finish(store, &operation, edit, false)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(e);
        }
        crash("commit");
        file.admit_result(&pending, true)?;
        // A durable Commit advances logical state even if cleanup fails. The
        // pending marker then blocks all subsequent edits until reopen recovery.
        let _ = file.finish(store, &operation, edit, true);
        crash("finish");
        Ok(())
    }
    fn recover_project(&mut self, last: Option<ProjectRecord>) -> Result<(), CommandError> {
        let current = self.export_crdt()?;
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let file = self.project_file.as_mut().ok_or(SourceError::Unavailable)?;
        let Some(pending) = file.pending(store)? else {
            return Ok(());
        };
        let record = last.filter(|r| r.operation == pending.operation);
        match record.as_ref().map(|r| r.status) {
            Some(ProjectStatus::Prepared) => {
                file.rollback(store, &pending.operation, &pending.edit)
                    .map_err(|_| SourceError::RecoveryRequired)?;
                self.journal.as_mut().expect("journal").append(
                    Action::ProjectAbort {
                        operation: pending.operation.clone(),
                    },
                    &current,
                )?;
            }
            Some(ProjectStatus::Committed) => {
                file.verify_result(store, &pending, true)?;
            }
            Some(ProjectStatus::Aborted) | None => {
                file.verify_result(store, &pending, false)?;
            }
        }
        let committed = record.is_some_and(|r| r.status == ProjectStatus::Committed);
        file.admit_result(&pending, committed)?;
        file.finish(store, &pending.operation, &pending.edit, committed)?;
        Ok(())
    }
}
fn check_recovery_join(
    bus: &CommandBus,
    pending: Option<&incant_source::ProjectPending>,
    last: Option<&ProjectRecord>,
) -> Result<(), CommandError> {
    if last.is_some_and(|r| {
        r.status == ProjectStatus::Prepared && pending.is_none_or(|p| p.operation != r.operation)
    }) {
        return Err(SourceError::RecoveryRequired.into());
    }
    match (pending, last) {
        (Some(p), Some(r)) if r.operation == p.operation => {
            if r.edit != p.edit {
                return Err(SourceError::RecoveryRequired.into());
            }
            Ok(())
        }
        (Some(p), _) => {
            // Marker-before-Prepare may only describe the current logical state.
            if Project::from_text(&p.edit.before)? != bus.project {
                return Err(SourceError::RecoveryRequired.into());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn crash(_phase: &str) {
    #[cfg(test)]
    if std::env::var("INCANT_PROJECT_PROTOCOL_CRASH").as_deref() == Ok(_phase) {
        std::process::exit(73);
    }
}

fn operation_id() -> Id {
    #[cfg(test)]
    if let Ok(id) = std::env::var("INCANT_PROJECT_TEST_OPERATION_ID") {
        return id;
    }
    new_id()
}
