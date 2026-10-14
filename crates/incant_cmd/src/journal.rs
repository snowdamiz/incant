pub(super) mod scan;
pub(super) use scan::{ProjectRecord, ProjectStatus};
pub(super) const MAX_PROJECT_RECORD_BYTES: usize = 128 * 1024 * 1024;
pub(super) const MAX_PROJECT_JOURNAL_BYTES: usize = 256 * 1024 * 1024;
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Change {
    Apply { transaction: Box<Transaction> },
    Undo { transaction_id: Id },
    Redo { transaction_id: Id },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    Initialize {
        project: Box<Project>,
    },
    Apply {
        transaction: Box<Transaction>,
    },
    Undo {
        transaction_id: Id,
    },
    Redo {
        transaction_id: Id,
    },
    Prepare {
        operation: Id,
        edit: incant_source::SourceEdit,
        change: Change,
    },
    Commit {
        operation: Id,
    },
    Abort {
        operation: Id,
    },
    ProjectPrepare {
        operation: Id,
        edit: incant_source::ProjectEdit,
        change: Change,
    },
    ProjectCommit {
        operation: Id,
    },
    ProjectAbort {
        operation: Id,
    },
}
impl From<Change> for Action {
    fn from(change: Change) -> Self {
        match change {
            Change::Apply { transaction } => Self::Apply { transaction },
            Change::Undo { transaction_id } => Self::Undo { transaction_id },
            Change::Redo { transaction_id } => Self::Redo { transaction_id },
        }
    }
}
pub(super) struct SourceRecord {
    pub operation: Id,
    pub edit: incant_source::SourceEdit,
    pub finished: bool,
}
pub(super) fn replay_change(
    bus: &mut CommandBus,
    change: Change,
    project_id: &str,
) -> Result<(), CommandError> {
    match change {
        Change::Apply { transaction } => {
            if transaction.before != bus.project {
                return Err(CommandError::Corrupt("transaction base mismatch".into()));
            }
            transaction.actor.provenance(&transaction.id)?;
            super::project_file::validate_transaction(&transaction)?;
            transaction.after.validate()?;
            if let Some(edit) = &transaction.source_edit {
                edit.validate()?;
            }
            if transaction.after.id != project_id {
                return Err(CommandError::Corrupt("project identity changed".into()));
            }
            bus.commit_transaction(*transaction)?;
        }
        Change::Undo { transaction_id } => {
            if bus.undo()? != transaction_id {
                return Err(CommandError::Corrupt("undo ID mismatch".into()));
            }
        }
        Change::Redo { transaction_id } => {
            if bus.redo()? != transaction_id {
                return Err(CommandError::Corrupt("redo ID mismatch".into()));
            }
        }
    }
    Ok(())
}
pub(super) fn change_edit(bus: &CommandBus, change: &Change) -> Option<incant_source::SourceEdit> {
    match change {
        Change::Apply { transaction } => transaction.source_edit.clone(),
        Change::Undo { transaction_id } => bus
            .history
            .last()
            .filter(|tx| tx.id == *transaction_id)
            .and_then(|tx| {
                tx.source_edit
                    .as_ref()
                    .map(incant_source::SourceEdit::reversed)
            }),
        Change::Redo { transaction_id } => bus
            .redo
            .last()
            .filter(|tx| tx.id == *transaction_id)
            .and_then(|tx| tx.source_edit.clone()),
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub version: u32,
    pub sequence: u64,
    pub previous: String,
    pub action: Action,
    pub crdt: String,
    pub digest: String,
}
/// Hash-chained JSONL write-ahead journal. Exclusive OS lock prevents two writers.
pub struct Journal {
    file: File,
    pub(super) poisoned: bool,
    sequence: u64,
    previous: String,
    bounded: bool,
    // Bounded host journals retain exact durable logical snapshot encoding.
    // Re-export after Loro import is not byte-canonical; Abort must reuse this.
    checkpoint: Option<String>,
}
pub(super) fn digest(
    version: u32,
    sequence: u64,
    previous: &str,
    action: &Action,
    crdt: &str,
) -> Result<String, CommandError> {
    let bytes = serde_json::to_vec(&(version, sequence, previous, action, crdt))
        .map_err(|e| CommandError::Corrupt(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
impl Journal {
    #[cfg(test)]
    pub(super) fn test_file(&mut self) -> &mut File {
        &mut self.file
    }

    pub(super) fn open(
        path: &Path,
        initial: Project,
        root: Option<&Path>,
    ) -> Result<CommandBus, CommandError> {
        initial.validate()?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.try_lock()
            .map_err(|e| CommandError::Io(std::io::Error::other(e)))?;
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        let result = scan::replay(file, &bytes, initial.clone(), false)?;
        if !result
            .checkpoints
            .contains(&scan::project_digest(&initial)?)
        {
            return Err(CommandError::Corrupt(
                "project file is not a checkpoint of this journal".into(),
            ));
        }
        let scan::Replay {
            mut bus,
            last_source,
            last_project,
            complete,
            ..
        } = result;
        if last_project.is_some() {
            return Err(SourceError::RecoveryRequired.into());
        }
        bus.journal
            .as_mut()
            .expect("replayed journal")
            .finish_read(complete, bytes.len())?;
        bus.initialize_journal()?;
        if let Some(root) = root {
            let store = incant_source::SourceStore::open(root, path, &bus.project.id)?;
            bus.recover_sources(store, last_source)?;
        } else if last_source.is_some_and(|r| !r.finished) {
            return Err(SourceError::RecoveryRequired.into());
        }
        Ok(bus)
    }
    pub(super) fn initialize(
        &mut self,
        project: &Project,
        crdt: &[u8],
    ) -> Result<(), CommandError> {
        if self.sequence == 0 {
            self.append(
                Action::Initialize {
                    project: Box::new(project.clone()),
                },
                crdt,
            )?;
        }
        Ok(())
    }
    pub(super) fn admit_project(
        &mut self,
        prepare: &Action,
        operation: &str,
        current: &[u8],
        desired: &[u8],
    ) -> Result<(), CommandError> {
        if !self.bounded || self.poisoned {
            return Err(SourceError::RecoveryRequired.into());
        }
        let size = |action: &Action, crdt: &[u8]| -> Result<usize, CommandError> {
            let crdt = if matches!(action, Action::ProjectAbort { .. }) {
                self.checkpoint
                    .clone()
                    .ok_or(SourceError::RecoveryRequired)?
            } else {
                STANDARD.encode(crdt)
            };
            let digest = digest(3, self.sequence, &self.previous, action, &crdt)?;
            // Sequence width may grow at Commit; reserve twenty decimal digits.
            let entry = Entry {
                version: 3,
                sequence: u64::MAX,
                previous: self.previous.clone(),
                action: action.clone(),
                crdt,
                digest,
            };
            Ok(incant_source::bounded_json(&entry, MAX_PROJECT_RECORD_BYTES)?.len() + 1)
        };
        let required = size(prepare, desired)?
            .checked_add(
                size(
                    &Action::ProjectCommit {
                        operation: operation.into(),
                    },
                    desired,
                )?
                .max(size(
                    &Action::ProjectAbort {
                        operation: operation.into(),
                    },
                    current,
                )?),
            )
            .ok_or_else(|| CommandError::Invalid("project journal size overflow".into()))?;
        if self.file.metadata()?.len().saturating_add(required as u64)
            > MAX_PROJECT_JOURNAL_BYTES as u64
        {
            return Err(CommandError::Invalid(
                "project journal has no room for Prepare and terminal record".into(),
            ));
        }
        Ok(())
    }
    pub(super) fn finish_read(&mut self, complete: usize, len: usize) -> Result<(), CommandError> {
        if complete < len {
            self.file.set_len(complete as u64)?;
            self.file.sync_all()?;
        }
        self.file.seek(SeekFrom::End(0))?;
        Ok(())
    }
    pub(super) fn append(&mut self, action: Action, crdt_bytes: &[u8]) -> Result<(), CommandError> {
        if self.poisoned {
            return Err(SourceError::RecoveryRequired.into());
        }
        let crdt = if matches!(action, Action::ProjectAbort { .. }) {
            self.checkpoint
                .clone()
                .ok_or(SourceError::RecoveryRequired)?
        } else {
            STANDARD.encode(crdt_bytes)
        };
        let logical = !matches!(
            action,
            Action::Prepare { .. } | Action::ProjectPrepare { .. }
        );
        let version = if matches!(
            action,
            Action::ProjectPrepare { .. }
                | Action::ProjectCommit { .. }
                | Action::ProjectAbort { .. }
        ) {
            if !self.bounded {
                return Err(SourceError::Unavailable.into());
            }
            3
        } else if matches!(
            action,
            Action::Prepare { .. } | Action::Commit { .. } | Action::Abort { .. }
        ) {
            2
        } else {
            1
        };
        let hash = digest(version, self.sequence, &self.previous, &action, &crdt)?;
        let entry = Entry {
            version,
            sequence: self.sequence,
            previous: self.previous.clone(),
            action,
            crdt,
            digest: hash.clone(),
        };
        let mut bytes = if self.bounded {
            incant_source::bounded_json(&entry, MAX_PROJECT_RECORD_BYTES)?
        } else {
            serde_json::to_vec(&entry).map_err(|e| CommandError::Corrupt(e.to_string()))?
        };
        bytes.push(b'\n');
        let position = self.file.seek(SeekFrom::End(0))?;
        if self.bounded
            && position.saturating_add(bytes.len() as u64) > MAX_PROJECT_JOURNAL_BYTES as u64
        {
            return Err(CommandError::Invalid(
                "project journal exceeds 256 MiB; no transaction written".into(),
            ));
        }
        #[cfg(test)]
        let injected =
            crate::source_protocol_tests::journal_failure(&entry.action, &mut self.file, &bytes)
                .and_then(|()| {
                    crate::project_file_tests::journal_failure(
                        &entry.action,
                        &mut self.file,
                        &bytes,
                    )
                });
        #[cfg(not(test))]
        let injected: std::io::Result<()> = Ok(());
        let result = injected
            .and_then(|()| self.file.write_all(&bytes))
            .and_then(|()| self.file.sync_all());
        if let Err(error) = result {
            if self
                .file
                .set_len(position)
                .and_then(|()| self.file.seek(SeekFrom::End(0)).map(|_| ()))
                .and_then(|()| self.file.sync_all())
                .is_err()
            {
                self.poisoned = true;
                return Err(SourceError::RecoveryRequired.into());
            }
            return Err(error.into());
        }
        self.sequence += 1;
        self.previous = hash;
        if self.bounded && logical {
            self.checkpoint = Some(entry.crdt);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "agent_budget_replay_tests.rs"]
mod agent_budget_replay_tests;
