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
pub(super) enum Action {
    Initialize { project: Project },
    Apply { transaction: Box<Transaction> },
    Undo { transaction_id: Id },
    Redo { transaction_id: Id },
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
    sequence: u64,
    previous: String,
}
fn digest(
    sequence: u64,
    previous: &str,
    action: &Action,
    crdt: &str,
) -> Result<String, CommandError> {
    let bytes = serde_json::to_vec(&(1u32, sequence, previous, action, crdt))
        .map_err(|e| CommandError::Corrupt(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
impl Journal {
    pub(super) fn open(path: &Path, initial: Project) -> Result<CommandBus, CommandError> {
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
            sequence: 0,
            previous: String::new(),
        };
        let mut checkpoint_matches = false;
        let mut last_crdt = None;
        for line in bytes[..complete]
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
        {
            let entry: Entry =
                serde_json::from_slice(line).map_err(|e| CommandError::Corrupt(e.to_string()))?;
            if entry.version != 1
                || entry.sequence != journal.sequence
                || entry.previous != journal.previous
                || entry.digest
                    != digest(entry.sequence, &entry.previous, &entry.action, &entry.crdt)?
            {
                return Err(CommandError::Corrupt(
                    "hash chain or sequence mismatch".into(),
                ));
            }
            match &entry.action {
                Action::Initialize { project } => {
                    if journal.sequence != 0 || project.id != initial.id {
                        return Err(CommandError::Corrupt(
                            "journal initial project mismatch".into(),
                        ));
                    }
                    bus = CommandBus::new(project.clone())?;
                }
                Action::Apply { transaction } => {
                    if journal.sequence == 0 || transaction.before != bus.project {
                        return Err(CommandError::Corrupt("transaction base mismatch".into()));
                    }
                    transaction.actor.provenance(&transaction.id)?;
                    transaction.after.validate()?;
                    if transaction.after.id != initial.id {
                        return Err(CommandError::Corrupt("project identity changed".into()));
                    }
                    bus.commit_transaction(*transaction.clone())?;
                }
                Action::Undo { transaction_id } => {
                    if &bus.undo()? != transaction_id {
                        return Err(CommandError::Corrupt("undo ID mismatch".into()));
                    }
                }
                Action::Redo { transaction_id } => {
                    if &bus.redo()? != transaction_id {
                        return Err(CommandError::Corrupt("redo ID mismatch".into()));
                    }
                }
            }
            last_crdt = Some(entry.crdt);
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
            journal.append(Action::Initialize { project: initial }, &bus.export_crdt()?)?;
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
        bus.journal = Some(journal);
        Ok(bus)
    }
    pub(super) fn append(&mut self, action: Action, crdt_bytes: &[u8]) -> Result<(), CommandError> {
        let crdt = STANDARD.encode(crdt_bytes);
        let hash = digest(self.sequence, &self.previous, &action, &crdt)?;
        let entry = Entry {
            version: 1,
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
        if let Err(error) = self
            .file
            .write_all(&bytes)
            .and_then(|_| self.file.sync_all())
        {
            self.file.set_len(position)?;
            self.file.seek(SeekFrom::End(0))?;
            self.file.sync_all()?;
            return Err(error.into());
        }
        self.sequence += 1;
        self.previous = hash;
        Ok(())
    }
}
