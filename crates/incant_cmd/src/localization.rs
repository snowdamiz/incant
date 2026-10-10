//! XLIFF preparation produces ordinary table commands; commit uses the same bus as every author.
use crate::{Actor, Command, CommandBus, CommandError};
use incant_doc::Project;
use serde::Serialize;
use std::collections::BTreeSet;

pub struct PreparedTranslations {
    project: Project,
    revision: u64,
    commands: Vec<Command>,
    changed_messages: usize,
}
#[derive(Debug, Serialize)]
pub struct TranslationImportResult {
    pub changed_messages: usize,
    pub changed_tables: usize,
    pub transaction_id: Option<String>,
    pub revision: u64,
}
impl PreparedTranslations {
    /// Decode and validate every imported target before publishing any edit.
    pub fn prepare(bus: &CommandBus, document: &str) -> Result<Self, CommandError> {
        let project = bus.project().clone();
        let updates =
            incant_localization::translations_from_xliff(&project.string_tables, document)
                .map_err(|e| CommandError::Invalid(e.to_string()))?;
        let mut after = project.clone();
        let mut changed = BTreeSet::new();
        let mut changed_messages = 0;
        for update in updates {
            let values = after
                .string_tables
                .get_mut(&update.table_id)
                .unwrap()
                .messages
                .get_mut(&update.key)
                .unwrap();
            if values.get(&update.locale) != Some(&update.value) {
                values.insert(update.locale, update.value);
                changed.insert(update.table_id);
                changed_messages += 1;
            }
        }
        after.validate()?;
        let commands = changed
            .into_iter()
            .map(|id| Command::UpsertStringTable {
                table: after.string_tables[&id].clone(),
            })
            .collect();
        Ok(Self {
            project,
            revision: bus.revision(),
            commands,
            changed_messages,
        })
    }
    /// Consumes the immutable prepared batch. A stale or different project is rejected.
    pub fn commit(
        self,
        bus: &mut CommandBus,
        actor: Actor,
    ) -> Result<TranslationImportResult, CommandError> {
        if bus.revision() != self.revision {
            return Err(CommandError::Conflict {
                expected: self.revision,
                actual: bus.revision(),
            });
        }
        if bus.project() != &self.project {
            return Err(CommandError::Invalid(
                "translation import snapshot changed".into(),
            ));
        }
        let changed_tables = self.commands.len();
        let transaction_id = if self.commands.is_empty() {
            None
        } else {
            Some(
                bus.execute(
                    self.commands,
                    actor,
                    "Import XLIFF translations",
                    Some(self.revision),
                )?
                .id
                .clone(),
            )
        };
        Ok(TranslationImportResult {
            changed_messages: self.changed_messages,
            changed_tables,
            transaction_id,
            revision: bus.revision(),
        })
    }
}
