//! Bounded project bootstrap shares the existing replay semantics. Scanning is
//! read-only; no marker or document is changed before the complete chain passes.
use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectStatus {
    Prepared,
    Committed,
    Aborted,
}
pub(crate) struct ProjectRecord {
    pub operation: Id,
    pub edit: incant_source::ProjectEdit,
    pub status: ProjectStatus,
    pub checkpoint: bool,
}
enum Pending {
    Checkpoint {
        operation: Id,
        change: Option<Change>,
        crdt: String,
    },
    Source {
        operation: Id,
        change: Change,
        crdt: String,
    },
    Project {
        operation: Id,
        change: Change,
        crdt: String,
    },
}
pub(crate) struct Replay {
    pub bus: CommandBus,
    pub checkpoints: BTreeSet<String>,
    pub complete: usize,
    pub last_source: Option<SourceRecord>,
    pub last_project: Option<ProjectRecord>,
}
pub(crate) fn project_digest(project: &Project) -> Result<String, CommandError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(project.canonical_text()?.as_bytes())
    ))
}
pub(crate) fn project_edit(
    bus: &CommandBus,
    change: &Change,
) -> Option<incant_source::ProjectEdit> {
    match change {
        Change::Apply { transaction } => transaction.project_edit.clone(),
        Change::Undo { transaction_id } => bus
            .history
            .last()
            .filter(|tx| tx.id == *transaction_id)
            .and_then(|tx| {
                tx.project_edit
                    .as_ref()
                    .map(incant_source::ProjectEdit::reversed)
            }),
        Change::Redo { transaction_id } => bus
            .redo
            .last()
            .filter(|tx| tx.id == *transaction_id)
            .and_then(|tx| tx.project_edit.clone()),
    }
}
fn corrupt(message: &str) -> CommandError {
    CommandError::Corrupt(message.into())
}
pub(crate) fn initialize(bytes: &[u8]) -> Result<Option<Project>, CommandError> {
    let Some(end) = bytes.iter().position(|b| *b == b'\n') else {
        return Ok(None);
    };
    if end > MAX_PROJECT_RECORD_BYTES {
        return Err(corrupt("initial journal record exceeds limit"));
    }
    let e: Entry = serde_json::from_slice(&bytes[..end]).map_err(|e| corrupt(&e.to_string()))?;
    if e.version != 1
        || e.sequence != 0
        || !e.previous.is_empty()
        || e.digest != digest(e.version, e.sequence, &e.previous, &e.action, &e.crdt)?
    {
        return Err(corrupt("invalid journal initialization"));
    }
    let Action::Initialize { project } = e.action else {
        return Err(corrupt("journal lacks initialization"));
    };
    project.validate()?;
    Ok(Some(*project))
}
pub(crate) fn replay(
    file: File,
    bytes: &[u8],
    initial: Project,
    bounded: bool,
) -> Result<Replay, CommandError> {
    initial.validate()?;
    let complete = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    let mut bus = CommandBus::new(initial.clone())?;
    let mut journal = Journal {
        file,
        poisoned: false,
        sequence: 0,
        previous: String::new(),
        bounded,
        checkpoint: None,
    };
    bus.replaying = true;
    let mut pending = None;
    let mut last_source = None;
    let mut last_project = None;
    let mut checkpoints = BTreeSet::new();
    for line in bytes[..complete]
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        if bounded && line.len() > MAX_PROJECT_RECORD_BYTES {
            return Err(corrupt("journal record exceeds limit"));
        }
        let entry: Entry = serde_json::from_slice(line).map_err(|e| corrupt(&e.to_string()))?;
        if !matches!(entry.version, 1..=4)
            || (!bounded && entry.version >= 3)
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
            return Err(corrupt("journal version, hash chain or sequence mismatch"));
        }
        let mut logical = true;
        let committed = matches!(
            entry.action,
            Action::Commit { .. } | Action::ProjectCommit { .. } | Action::CheckpointCommit { .. }
        );
        match entry.action {
            Action::Initialize { project } => {
                if entry.version != 1 || journal.sequence != 0 || project.id != initial.id {
                    return Err(corrupt("journal initial project mismatch"));
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
                    || project_edit(&bus, &change).is_some()
                    || ulid::Ulid::from_string(&operation).is_err()
                {
                    return Err(corrupt("invalid source prepare"));
                }
                edit.validate()?;
                last_source = Some(SourceRecord {
                    operation: operation.clone(),
                    edit,
                    finished: false,
                });
                pending = Some(Pending::Source {
                    operation,
                    change,
                    crdt: entry.crdt.clone(),
                });
                logical = false;
            }
            Action::ProjectPrepare {
                operation,
                edit,
                change,
            } => {
                if entry.version != 3
                    || journal.sequence == 0
                    || pending.is_some()
                    || project_edit(&bus, &change).as_ref() != Some(&edit)
                    || change_edit(&bus, &change).is_some()
                    || ulid::Ulid::from_string(&operation)
                        .map(|id| id.to_string())
                        .ok()
                        .as_ref()
                        != Some(&operation)
                {
                    return Err(corrupt("invalid project prepare"));
                }
                // Validate even a never-committed prepare before consulting its marker.
                if let Change::Apply { transaction } = &change {
                    super::super::project_file::validate_transaction(transaction)?;
                    if transaction.before != bus.project {
                        return Err(corrupt("project prepare base mismatch"));
                    }
                    transaction.actor.provenance(&transaction.id)?;
                }
                edit.validate()?;
                let desired = STANDARD
                    .decode(&entry.crdt)
                    .map_err(|_| corrupt("invalid project prepare CRDT"))?;
                if CollaborativeDocument::from_export(&desired)?.project()?
                    != Project::from_text(&edit.after)?
                {
                    return Err(corrupt("project prepare projection mismatch"));
                }
                if Project::from_text(&edit.before)? != bus.project {
                    return Err(corrupt("project preimage mismatch"));
                }
                last_project = Some(ProjectRecord {
                    operation: operation.clone(),
                    edit,
                    status: ProjectStatus::Prepared,
                    checkpoint: false,
                });
                pending = Some(Pending::Project {
                    operation,
                    change,
                    crdt: entry.crdt.clone(),
                });
                logical = false;
            }
            Action::CheckpointPrepare {
                operation,
                edit,
                revision,
                actor,
                change,
            } => {
                if entry.version != 4
                    || journal.sequence == 0
                    || pending.is_some()
                    || ulid::Ulid::from_string(&operation)
                        .map(|id| id.to_string())
                        .ok()
                        .as_ref()
                        != Some(&operation)
                    || !checkpoints.contains(&project_digest(&Project::from_text(&edit.before)?)?)
                {
                    return Err(corrupt("invalid checkpoint prepare"));
                }
                super::super::project_checkpoint::validate_prepare(
                    &bus, &edit, &change, revision, &actor, &operation,
                )?;
                let desired = STANDARD
                    .decode(&entry.crdt)
                    .map_err(|_| corrupt("invalid checkpoint CRDT"))?;
                if CollaborativeDocument::from_export(&desired)?.project()?
                    != Project::from_text(&edit.after)?
                    || (change.is_none() && journal.checkpoint.as_ref() != Some(&entry.crdt))
                {
                    return Err(corrupt("checkpoint causal state mismatch"));
                }
                last_project = Some(ProjectRecord {
                    operation: operation.clone(),
                    edit,
                    status: ProjectStatus::Prepared,
                    checkpoint: true,
                });
                pending = Some(Pending::Checkpoint {
                    operation,
                    change,
                    crdt: entry.crdt.clone(),
                });
                logical = false;
            }
            Action::CheckpointCommit { operation } | Action::CheckpointAbort { operation } => {
                let Some(Pending::Checkpoint {
                    operation: prepared,
                    change,
                    crdt,
                }) = pending.take()
                else {
                    return Err(corrupt("checkpoint terminal without prepare"));
                };
                if entry.version != 4 || operation != prepared {
                    return Err(corrupt("checkpoint operation mismatch"));
                }
                if committed {
                    if entry.crdt != crdt {
                        return Err(corrupt("checkpoint commit causal mismatch"));
                    }
                    if let Some(change) = change {
                        replay_change(&mut bus, change, &initial.id)?;
                    }
                } else if journal.checkpoint.as_ref() != Some(&entry.crdt) {
                    return Err(corrupt("checkpoint abort causal mismatch"));
                }
                last_project.as_mut().expect("prepared checkpoint").status = if committed {
                    ProjectStatus::Committed
                } else {
                    ProjectStatus::Aborted
                };
            }
            Action::Commit { operation } | Action::Abort { operation } => {
                let Some(Pending::Source {
                    operation: prepared,
                    change,
                    crdt,
                }) = pending.take()
                else {
                    return Err(corrupt("source terminal without prepare"));
                };
                if entry.version != 2 || operation != prepared {
                    return Err(corrupt("source operation mismatch"));
                }
                if committed {
                    if entry.crdt != crdt {
                        return Err(corrupt("source commit projection mismatch"));
                    }
                    replay_change(&mut bus, change, &initial.id)?;
                }
                last_source.as_mut().expect("prepared source").finished = true;
            }
            Action::ProjectCommit { operation } | Action::ProjectAbort { operation } => {
                let Some(Pending::Project {
                    operation: prepared,
                    change,
                    crdt,
                }) = pending.take()
                else {
                    return Err(corrupt("project terminal without prepare"));
                };
                if entry.version != 3 || operation != prepared {
                    return Err(corrupt("project operation mismatch"));
                }
                if committed {
                    if entry.crdt != crdt {
                        return Err(corrupt("project commit projection mismatch"));
                    }
                    replay_change(&mut bus, change, &initial.id)?;
                } else if journal.checkpoint.as_ref() != Some(&entry.crdt) {
                    return Err(corrupt("project abort projection mismatch"));
                }
                last_project.as_mut().expect("prepared project").status = if committed {
                    ProjectStatus::Committed
                } else {
                    ProjectStatus::Aborted
                };
            }
            action => {
                if entry.version != 1 || pending.is_some() || journal.sequence == 0 {
                    return Err(corrupt("change overlaps pending operation"));
                }
                let change = match action {
                    Action::Apply { transaction } => Change::Apply { transaction },
                    Action::Undo { transaction_id } => Change::Undo { transaction_id },
                    Action::Redo { transaction_id } => Change::Redo { transaction_id },
                    _ => unreachable!(),
                };
                if change_edit(&bus, &change).is_some() || project_edit(&bus, &change).is_some() {
                    return Err(corrupt("file change lacks prepare/commit"));
                }
                replay_change(&mut bus, change, &initial.id)?;
            }
        }
        if logical {
            // Check each checkpoint, not merely the last one. A corrupt earlier
            // projection cannot become bootstrap authority through later entries.
            let raw = STANDARD
                .decode(&entry.crdt)
                .map_err(|_| corrupt("invalid CRDT encoding"))?;
            let restored = CollaborativeDocument::from_export(&raw)?;
            if restored.project()? != bus.project {
                return Err(corrupt("CRDT projection and journal state differ"));
            }
            bus.document = Some(restored);
            if bounded {
                journal.checkpoint = Some(entry.crdt);
            }
            checkpoints.insert(project_digest(&bus.project)?);
        }
        journal.sequence = journal
            .sequence
            .checked_add(1)
            .ok_or_else(|| corrupt("journal sequence exhausted"))?;
        journal.previous = entry.digest;
    }
    if journal.sequence == 0 {
        // Only after the entire bounded scan passes may a fresh Initialize write.
        // Project reads and root lock are established by the new host opener first.
        checkpoints.insert(project_digest(&initial)?);
    }
    bus.replaying = false;
    bus.journal = Some(journal);
    Ok(Replay {
        bus,
        checkpoints,
        complete,
        last_source,
        last_project,
    })
}
