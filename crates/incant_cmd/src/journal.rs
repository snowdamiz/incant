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
fn replay_change(
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
fn change_edit(bus: &CommandBus, change: &Change) -> Option<incant_source::SourceEdit> {
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
struct Entry {
    version: u32,
    sequence: u64,
    previous: String,
    action: Action,
    crdt: String,
    digest: String,
}
/// Hash-chained JSONL write-ahead journal. Exclusive OS lock prevents two writers.
pub struct Journal {
    file: File,
    pub(super) poisoned: bool,
    sequence: u64,
    previous: String,
}
fn digest(
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
        // A crash may leave one incomplete last line. Never hide corruption of a complete line.
        let complete = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        let mut bus = CommandBus::new(initial.clone())?;
        let mut journal = Self {
            file,
            poisoned: false,
            sequence: 0,
            previous: String::new(),
        };
        bus.replaying = true;
        let mut pending: Option<(Id, incant_source::SourceEdit, Change, String)> = None;
        let mut last_source = None;
        let mut checkpoint_matches = false;
        let mut last_crdt = None;
        for line in bytes[..complete]
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
        {
            let entry: Entry =
                serde_json::from_slice(line).map_err(|e| CommandError::Corrupt(e.to_string()))?;
            if !matches!(entry.version, 1 | 2)
                || entry.sequence != journal.sequence
                || entry.previous != journal.previous
                || entry.digest
                    != digest(
                        entry.version,
                        entry.sequence,
                        &entry.previous,
                        &entry.action,
                        &entry.crdt,
                    )?
            {
                return Err(CommandError::Corrupt(
                    "hash chain or sequence mismatch".into(),
                ));
            }
            let mut logical = true;
            let committed = matches!(&entry.action, Action::Commit { .. });
            match entry.action {
                Action::Initialize { project } => {
                    if journal.sequence != 0 || project.id != initial.id {
                        return Err(CommandError::Corrupt(
                            "journal initial project mismatch".into(),
                        ));
                    }
                    bus = CommandBus::new(*project)?;
                    bus.replaying = true;
                }
                Action::Prepare {
                    operation,
                    edit,
                    change,
                } => {
                    if entry.version != 2
                        || journal.sequence == 0
                        || pending.is_some()
                        || change_edit(&bus, &change).as_ref() != Some(&edit)
                        || ulid::Ulid::from_string(&operation).is_err()
                    {
                        return Err(CommandError::Corrupt("invalid source prepare".into()));
                    }
                    edit.validate()?;
                    last_source = Some(SourceRecord {
                        operation: operation.clone(),
                        edit: edit.clone(),
                        finished: false,
                    });
                    pending = Some((operation, edit, change, entry.crdt.clone()));
                    logical = false;
                }
                Action::Commit { operation } | Action::Abort { operation } => {
                    let Some((prepared, _, change, crdt)) = pending.take() else {
                        return Err(CommandError::Corrupt(
                            "source terminal without prepare".into(),
                        ));
                    };
                    if entry.version != 2 || operation != prepared {
                        return Err(CommandError::Corrupt("source operation mismatch".into()));
                    }
                    if committed {
                        if entry.crdt != crdt {
                            return Err(CommandError::Corrupt(
                                "source commit projection mismatch".into(),
                            ));
                        }
                        replay_change(&mut bus, change, &initial.id)?;
                    }
                    last_source.as_mut().expect("prepared source").finished = true;
                }
                action => {
                    if pending.is_some() || journal.sequence == 0 {
                        return Err(CommandError::Corrupt(
                            "change overlaps pending source operation".into(),
                        ));
                    }
                    let change = match action {
                        Action::Apply { transaction } => Change::Apply { transaction },
                        Action::Undo { transaction_id } => Change::Undo { transaction_id },
                        Action::Redo { transaction_id } => Change::Redo { transaction_id },
                        _ => unreachable!(),
                    };
                    if change_edit(&bus, &change).is_some() {
                        return Err(CommandError::Corrupt(
                            "source change lacks prepare/commit".into(),
                        ));
                    }
                    replay_change(&mut bus, change, &initial.id)?;
                }
            }
            if logical {
                last_crdt = Some(entry.crdt);
            }
            checkpoint_matches |= bus.project == initial;
            journal.sequence += 1;
            journal.previous = entry.digest;
        }
        if journal.sequence > 0 && !checkpoint_matches {
            return Err(CommandError::Corrupt(
                "project file is not a checkpoint of this journal".into(),
            ));
        }
        if complete < bytes.len() {
            journal.file.set_len(complete as u64)?;
            journal.file.sync_all()?;
        }
        journal.file.seek(SeekFrom::End(0))?;
        if journal.sequence == 0 {
            journal.append(
                Action::Initialize {
                    project: Box::new(initial),
                },
                &bus.export_crdt()?,
            )?;
        }
        if let Some(encoded) = last_crdt {
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| CommandError::Corrupt("invalid CRDT encoding".into()))?;
            let restored = CollaborativeDocument::from_export(&bytes)?;
            if restored.project()? != bus.project {
                return Err(CommandError::Corrupt(
                    "CRDT projection and journal state differ".into(),
                ));
            }
            bus.document = Some(restored);
        }
        bus.replaying = false;
        bus.journal = Some(journal);
        if let Some(root) = root {
            let store = incant_source::SourceStore::open(root, path, &bus.project.id)?;
            bus.recover_sources(store, last_source)?;
        } else if pending.is_some() {
            return Err(SourceError::RecoveryRequired.into());
        }
        Ok(bus)
    }
    pub(super) fn append(&mut self, action: Action, crdt_bytes: &[u8]) -> Result<(), CommandError> {
        if self.poisoned {
            return Err(SourceError::RecoveryRequired.into());
        }
        let crdt = STANDARD.encode(crdt_bytes);
        let version = if matches!(
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
        let mut bytes =
            serde_json::to_vec(&entry).map_err(|e| CommandError::Corrupt(e.to_string()))?;
        bytes.push(b'\n');
        let position = self.file.seek(SeekFrom::End(0))?;
        #[cfg(test)]
        let injected =
            crate::source_protocol_tests::journal_failure(&entry.action, &mut self.file, &bytes);
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
        Ok(())
    }
}

#[cfg(test)]
#[path = "agent_budget_replay_tests.rs"]
mod agent_budget_replay_tests;
