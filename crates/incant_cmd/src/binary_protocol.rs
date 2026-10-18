//! Separate binary WAL records preserve all historical text-record encodings.
use super::*;
use incant_source::{BinaryEdit, SourceStore};
impl CommandBus {
    pub(super) fn record_binary(
        &mut self,
        change: journal::Change,
        edit: &BinaryEdit,
        next: Option<&CollaborativeDocument>,
    ) -> Result<(), CommandError> {
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let current = self.export_crdt()?;
        let desired = next.ok_or(SourceError::Unavailable)?.export()?;
        let journal = self.journal.as_mut().ok_or(SourceError::Unavailable)?;
        let operation = new_id();
        let prepare = journal::Action::BinaryPrepare {
            operation: operation.clone(),
            edit: edit.clone(),
            change,
        };
        journal.admit_binary(&prepare, &operation, &current, &desired)?;
        if let Err(error) = store.begin_binary(&operation, edit) {
            store
                .ensure_ready()
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error.into());
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("marker");
        if let Err(error) = journal.append(prepare, &desired) {
            if journal.poisoned {
                return Err(SourceError::RecoveryRequired.into());
            }
            store
                .finish_binary(&operation)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error);
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("prepare");
        let installed = store
            .install_binary(&operation, edit)
            .map_err(CommandError::from);
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("installed");
        let committed = installed.and_then(|()| {
            journal.append(
                journal::Action::BinaryCommit {
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
                .rollback_binary(&operation, edit)
                .map_err(|_| SourceError::RecoveryRequired)?;
            journal
                .append(
                    journal::Action::BinaryAbort {
                        operation: operation.clone(),
                    },
                    &current,
                )
                .map_err(|_| SourceError::RecoveryRequired)?;
            store
                .finish_binary(&operation)
                .map_err(|_| SourceError::RecoveryRequired)?;
            return Err(error);
        }
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("commit");
        let _ = store.finish_binary(&operation);
        #[cfg(test)]
        crate::source_protocol_tests::crash_point("finish");
        Ok(())
    }
    pub(super) fn recover_binary(
        &mut self,
        store: &SourceStore,
        last: Option<journal::BinaryRecord>,
    ) -> Result<(), CommandError> {
        match (store.binary_pending()?, last) {
            (None, Some(record)) if !record.finished => {
                return Err(SourceError::RecoveryRequired.into());
            }
            (None, _) => {}
            (Some(pending), record) => {
                match record.filter(|record| record.operation == pending.operation) {
                    Some(record) => {
                        if record.edit != pending.edit {
                            return Err(SourceError::RecoveryRequired.into());
                        }
                        if !record.finished {
                            store
                                .rollback_binary(&pending.operation, &pending.edit)
                                .map_err(|_| SourceError::RecoveryRequired)?;
                            let crdt = self.export_crdt()?;
                            self.journal
                                .as_mut()
                                .ok_or(SourceError::Unavailable)?
                                .append(
                                    journal::Action::BinaryAbort {
                                        operation: pending.operation.clone(),
                                    },
                                    &crdt,
                                )?;
                        }
                    }
                    None => store
                        .check_binary_preimage(&pending.edit)
                        .map_err(|_| SourceError::RecoveryRequired)?,
                }
                store
                    .finish_binary(&pending.operation)
                    .map_err(|_| SourceError::RecoveryRequired)?;
            }
        }
        Ok(())
    }
}
