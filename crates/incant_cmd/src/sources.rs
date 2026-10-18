use super::*;
use incant_source::{SourceEdit, SourceKind, SourceStore};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScriptRead {
    pub script_id: Id,
    pub path: String,
    pub revision: u64,
    pub source: Option<String>,
    pub version: SourceVersion,
    /// None means no source transaction has tracked this path. False means an
    /// external edit is visible; reading never reconciles or overwrites it.
    pub matches_history: Option<bool>,
}
impl CommandBus {
    pub fn persistent_with_sources(
        path: impl AsRef<Path>,
        initial: Project,
        root: impl AsRef<Path>,
    ) -> Result<Self, CommandError> {
        Journal::open(path.as_ref(), initial, Some(root.as_ref()))
    }
    pub fn has_source_store(&self) -> bool {
        self.sources.is_some()
    }
    pub fn ensure_sources_ready(&self) -> Result<(), CommandError> {
        if self
            .journal
            .as_ref()
            .is_some_and(|journal| journal.poisoned)
        {
            return Err(SourceError::RecoveryRequired.into());
        }
        if let Some(store) = &self.sources {
            store.ensure_ready()?;
        }
        Ok(())
    }
    pub fn read_script(&self, script_id: &str) -> Result<ScriptRead, CommandError> {
        self.ensure_sources_ready()?;
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let script = self
            .project
            .scripts
            .get(script_id)
            .ok_or_else(|| CommandError::Invalid("script does not exist".into()))?;
        incant_source::validate_path(&script.path)?;
        let source = store.read(&script.path)?;
        let last = self
            .history
            .iter()
            .rev()
            .filter_map(|tx| tx.source_edit.as_ref())
            .find(|edit| edit.path == script.path);
        Ok(ScriptRead {
            script_id: script.id.clone(),
            path: script.path.clone(),
            revision: self.revision,
            version: SourceVersion::of(source.as_deref()),
            matches_history: last.map(|edit| edit.after == source),
            source,
        })
    }
    pub(super) fn prepare_source_edit(
        &self,
        commands: &[Command],
        after: &Project,
        revision: Option<u64>,
    ) -> Result<Option<SourceEdit>, CommandError> {
        let mut writes = commands.iter().filter_map(|command| match command {
            Command::WriteScript {
                script_id,
                expected_source,
                source,
            } => Some((SourceKind::TypeScript, script_id, expected_source, source)),
            Command::WriteModuleSource {
                module_id,
                expected_source,
                source,
            } => Some((SourceKind::Rust, module_id, expected_source, source)),
            _ => None,
        });
        let Some((kind, id, expected, source)) = writes.next() else {
            return Ok(None);
        };
        if writes.next().is_some() {
            return Err(CommandError::Invalid(
                "one source write per transaction is supported (TypeScript or Rust)".into(),
            ));
        }
        if revision.is_none() {
            return Err(CommandError::Invalid(
                "source writes require expected_revision".into(),
            ));
        }
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        if self.journal.is_none() {
            return Err(SourceError::Unavailable.into());
        }
        let path = match kind {
            SourceKind::TypeScript => after.scripts.get(id).map(|source| &source.path),
            SourceKind::Rust => after.module_sources.get(id).map(|source| &source.path),
        }
        .ok_or_else(|| {
            CommandError::Invalid("written source must exist in the final project".into())
        })?;
        kind.validate_path(path)?;
        if after
            .scripts
            .values()
            .map(|s| (&s.id, &s.path))
            .chain(after.module_sources.values().map(|s| (&s.id, &s.path)))
            .any(|(other_id, other_path)| {
                other_id != id
                    && (other_path.eq_ignore_ascii_case(path)
                        || store.same_file(path, other_path).unwrap_or(false))
            })
        {
            return Err(CommandError::Invalid(
                "multiple source IDs alias the written path".into(),
            ));
        }
        kind.validate_source(source)?;
        let before = store.read(path)?;
        if let Some(text) = &before {
            kind.validate_source(text)?;
        }
        if SourceVersion::of(before.as_deref()) != *expected {
            return Err(SourceError::Conflict.into());
        }
        let created_tree = if kind == SourceKind::Rust && before.is_none() {
            store.prepare_creation(&new_id(), path, source)?
        } else {
            None
        };
        Ok(Some(SourceEdit {
            script_id: id.clone(),
            kind,
            path: path.clone(),
            before,
            after: Some(source.clone()),
            created_tree,
        }))
    }
    pub(super) fn record(
        &mut self,
        change: journal::Change,
        edit: Option<&SourceEdit>,
        project_edit: Option<&incant_source::ProjectEdit>,
        next: Option<&CollaborativeDocument>,
    ) -> Result<(), CommandError> {
        if self.replaying {
            return Ok(());
        }
        if let Some(project_edit) = project_edit {
            if edit.is_some() {
                return Err(CommandError::Invalid("mixed file edits".into()));
            }
            if matches!(
                change,
                journal::Change::Undo { .. } | journal::Change::Redo { .. }
            ) {
                return self.record_checkpoint_history(change, project_edit, next);
            }
            return self.record_project(change, project_edit, next);
        }
        let Some(edit) = edit else {
            if let Some(journal) = &mut self.journal {
                journal.append(change.into(), &next.expect("persistent document").export()?)?;
            }
            return Ok(());
        };
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let current = self.export_crdt()?;
        let desired = next.expect("persistent document").export()?;
        let journal = self.journal.as_mut().ok_or(SourceError::Unavailable)?;
        let operation = new_id();
        if let Err(error) = store.begin(&operation, edit) {
            // A rename may have published the marker before directory sync
            // failed. Do not report an ordinary refusal while recovery is pending.
            store
                .ensure_ready()
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error.into());
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("marker");
        if let Err(error) = journal.append(
            journal::Action::Prepare {
                operation: operation.clone(),
                edit: edit.clone(),
                change,
            },
            &desired,
        ) {
            if journal.poisoned {
                return Err(SourceError::RecoveryRequired.into());
            }
            // No file installation precedes a durable Prepare. A failure to clear
            // the marker must stay visible as recovery-required.
            store
                .finish(&operation)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error);
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("prepare");
        let installed = store.install(&operation, edit).map_err(CommandError::from);
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("installed");
        let committed = installed.and_then(|()| {
            journal.append(
                journal::Action::Commit {
                    operation: operation.clone(),
                },
                &desired,
            )
        });
        if let Err(error) = committed {
            if journal.poisoned {
                return Err(SourceError::RecoveryRequired.into());
            }
            store
                .rollback(&operation, edit)
                .map_err(|_| SourceError::RecoveryRequired)?;
            journal
                .append(
                    journal::Action::Abort {
                        operation: operation.clone(),
                    },
                    &current,
                )
                .map_err(|_| SourceError::RecoveryRequired)?;
            store
                .finish(&operation)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error);
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("commit");
        // A durable commit is authoritative even if marker cleanup fails. Leave
        // it for reopen recovery and publish the committed logical transaction.
        // Subsequent operations fail ensure_ready until recovery completes.
        let _ = store.finish(&operation);
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("finish");
        Ok(())
    }
    pub(super) fn recover_sources(
        &mut self,
        store: SourceStore,
        last: Option<journal::SourceRecord>,
    ) -> Result<(), CommandError> {
        match (store.pending()?, last) {
            (None, Some(record)) if !record.finished => {
                return Err(SourceError::RecoveryRequired.into());
            }
            (None, _) => {}
            (Some(pending), record) => {
                match record.filter(|r| r.operation == pending.operation) {
                    Some(record) => {
                        if record.edit != pending.edit {
                            return Err(SourceError::RecoveryRequired.into());
                        }
                        if !record.finished {
                            store
                                .rollback(&pending.operation, &pending.edit)
                                .map_err(|_| SourceError::RecoveryRequired)?;
                            let crdt = self.export_crdt()?;
                            self.journal.as_mut().expect("journal attached").append(
                                journal::Action::Abort {
                                    operation: pending.operation.clone(),
                                },
                                &crdt,
                            )?;
                        }
                    }
                    None => {
                        // Crash after durable marker but before durable Prepare:
                        // installation cannot have begun, so foreign changes block.
                        store
                            .check_preimage(&pending.edit)
                            .map_err(|_| SourceError::RecoveryRequired)?;
                    }
                }
                store
                    .finish(&pending.operation)
                    .map_err(|_| SourceError::RecoveryRequired)?;
            }
        }
        self.sources = Some(store);
        Ok(())
    }
}
