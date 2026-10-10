//! Bounded text-only XLIFF 2.0/2.1 exchange. File identifiers are table IDs, never paths.
mod reader;
use crate::{Catalog, LocalizationError, MAX_MESSAGE_BYTES, StringTable, invalid, types::locale};
use std::collections::BTreeMap;

pub const MAX_XLIFF_BYTES: usize = 32 * 1024 * 1024;
pub(super) const NS: &str = "urn:oasis:names:tc:xliff:document:2.0";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationUpdate {
    pub table_id: String,
    pub key: String,
    pub locale: String,
    pub value: String,
}
pub(super) fn error(message: impl Into<String>) -> LocalizationError {
    invalid("/xliff", message)
}
pub(super) fn valid_chars(text: &str) -> bool {
    text.chars().all(|c| matches!(c, '\t'|'\n'|'\r'|'\u{20}'..='\u{d7ff}'|'\u{e000}'..='\u{fffd}'|'\u{10000}'..='\u{10ffff}'))
}
fn escape(text: &str, attribute: bool) -> Result<String, LocalizationError> {
    if !valid_chars(text) {
        return Err(error("text contains a character outside XML 1.0"));
    }
    let mut escaped = quick_xml::escape::escape(text).replace('\r', "&#13;");
    if attribute {
        escaped = escaped.replace('\n', "&#10;").replace('\t', "&#9;");
    }
    Ok(escaped)
}
/// Export one table. Missing targets are omitted; present empty targets stay empty.
pub fn export_xliff(table: &StringTable, target_locale: &str) -> Result<String, LocalizationError> {
    Catalog::compile(&BTreeMap::from([(table.id.clone(), table.clone())]))?;
    locale(target_locale)?;
    if target_locale == table.source_locale {
        return Err(error("target must differ from the source locale"));
    }
    if table.messages.is_empty() {
        return Err(error("XLIFF export requires at least one message"));
    }
    let mut text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xliff xmlns=\"{NS}\" version=\"2.1\" srcLang=\"{}\" trgLang=\"{}\">\n  <file id=\"{}\" original=\"{}\">\n",
        escape(&table.source_locale, true)?,
        escape(target_locale, true)?,
        escape(&table.id, true)?,
        escape(&table.name, true)?
    );
    for (key, translations) in &table.messages {
        let target = translations.get(target_locale);
        text.push_str(&format!("    <unit id=\"{}\" canResegment=\"no\">\n      <segment id=\"1\" state=\"{}\">\n        <source xml:space=\"preserve\">{}</source>\n",
            escape(key,true)?,if target.is_some() {"translated"} else {"initial"},escape(&translations[&table.source_locale],false)?));
        if let Some(target) = target {
            text.push_str(&format!(
                "        <target xml:space=\"preserve\">{}</target>\n",
                escape(target, false)?
            ));
        }
        text.push_str("      </segment>\n    </unit>\n");
        if text.len() > MAX_XLIFF_BYTES - 32 {
            return Err(error("XLIFF exceeds 32 MiB"));
        }
    }
    text.push_str("  </file>\n</xliff>\n");
    // Never emit a document outside the importer's bounded profile.
    reader::read(&text)?;
    Ok(text)
}
/// Prepare translations without modifying the catalog. Applying them belongs to incant_cmd.
/// All identities and source patterns must match the current document; unknown/stale data fails.
pub fn translations_from_xliff(
    tables: &BTreeMap<String, StringTable>,
    document: &str,
) -> Result<Vec<TranslationUpdate>, LocalizationError> {
    let parsed = reader::read(document)?;
    locale(&parsed.source)?;
    locale(&parsed.target)?;
    if parsed.source == parsed.target {
        return Err(error("target must differ from the source locale"));
    }
    let mut updates = vec![];
    for (id, messages) in parsed.files {
        let table = tables
            .get(&id)
            .ok_or_else(|| error(format!("unknown table {id}")))?;
        if table.source_locale != parsed.source {
            return Err(error(format!("source locale changed for {id}")));
        }
        for (key, message) in messages {
            let values = table
                .messages
                .get(&key)
                .ok_or_else(|| error(format!("unknown message {id}/{key}")))?;
            let source = values
                .get(&parsed.source)
                .ok_or_else(|| error("table has no source translation"))?;
            if *source != message.source {
                return Err(error(format!(
                    "source changed for {id}/{key}; export again"
                )));
            }
            if let Some(value) = message.target {
                let translated = crate::message::parse(&value)?;
                crate::contracts::validate(&crate::message::parse(source)?, &translated)?;
                updates.push(TranslationUpdate {
                    table_id: id.clone(),
                    key,
                    locale: parsed.target.clone(),
                    value,
                });
            }
        }
    }
    Ok(updates)
}
