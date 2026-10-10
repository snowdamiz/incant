use crate::{
    message::{Node, parse},
    types::{key, locale},
    *,
};
use std::collections::{BTreeMap, BTreeSet};

struct CompiledTable {
    source_locale: String,
    messages: BTreeMap<String, BTreeMap<String, Vec<Node>>>,
}
/// Immutable compiled data. Build a replacement before publishing a table edit.
#[derive(Default)]
pub struct Catalog {
    tables: BTreeMap<String, CompiledTable>,
}
impl Catalog {
    pub fn compile(tables: &BTreeMap<String, StringTable>) -> Result<Self, LocalizationError> {
        if tables.len() > MAX_TABLES {
            return Err(invalid("tables", "at most 64 string tables"));
        }
        let mut compiled = BTreeMap::new();
        let mut bytes = 0_usize;
        let mut messages = 0_usize;
        for (id, table) in tables {
            let path = format!("/string_tables/{id}");
            if ulid::Ulid::from_string(id).is_err() || table.id != *id {
                return Err(invalid(&path, "table ID must match its ULID key"));
            }
            if table.name.trim().is_empty() || table.name.len() > 256 {
                return Err(invalid(&path, "name must contain 1..256 bytes"));
            }
            locale(&table.source_locale).map_err(|e| invalid(&path, e.to_string()))?;
            messages += table.messages.len();
            if messages > MAX_MESSAGES {
                return Err(invalid(
                    &path,
                    "at most 4096 message keys across all tables",
                ));
            }
            let mut patterns = BTreeMap::new();
            let mut locales = BTreeSet::new();
            for (name, translations) in &table.messages {
                let message_path = format!("{path}/messages/{name}");
                if !key(name) {
                    return Err(invalid(&message_path, "invalid message key"));
                }
                if !translations.contains_key(&table.source_locale) {
                    return Err(invalid(
                        &message_path,
                        "every key requires its source-locale message",
                    ));
                }
                if translations.len() > MAX_LOCALES {
                    return Err(invalid(&message_path, "at most 32 locales"));
                }
                let mut translated = BTreeMap::new();
                for (language, pattern) in translations {
                    let location = format!("{message_path}/{language}");
                    locale(language).map_err(|e| invalid(&location, e.to_string()))?;
                    locales.insert(language);
                    if locales.len() > MAX_LOCALES {
                        return Err(invalid(&path, "at most 32 locales per table"));
                    }
                    bytes = bytes.saturating_add(pattern.len() + name.len() + language.len());
                    if bytes > MAX_CATALOG_BYTES {
                        return Err(invalid(&location, "catalog text exceeds 4 MiB"));
                    }
                    translated.insert(
                        language.clone(),
                        parse(pattern).map_err(|e| invalid(&location, e.to_string()))?,
                    );
                }
                let source = &translated[&table.source_locale];
                for (language, nodes) in &translated {
                    crate::contracts::validate(source, nodes).map_err(|e| {
                        invalid(format!("{message_path}/{language}"), e.to_string())
                    })?;
                }
                patterns.insert(name.clone(), translated);
            }
            compiled.insert(
                id.clone(),
                CompiledTable {
                    source_locale: table.source_locale.clone(),
                    messages: patterns,
                },
            );
        }
        Ok(Self { tables: compiled })
    }
    pub fn localize(
        &self,
        settings: &LocaleSettings,
        request: &LocalizeRequest,
    ) -> Result<LocalizedText, LocalizationError> {
        settings.validate()?;
        if !key(&request.key) {
            return Err(invalid("key", "invalid message key"));
        }
        if request.arguments.len() > MAX_ARGUMENTS {
            return Err(invalid("arguments", "at most 64 arguments"));
        }
        let mut argument_bytes = 0;
        for (name, argument) in &request.arguments {
            if !key(name) {
                return Err(invalid("arguments", "invalid argument name"));
            }
            match argument {
                MessageArgument::Text(text) => argument_bytes += text.len(),
                MessageArgument::Number(number) => format::valid_number(*number)?,
                MessageArgument::Date(date) => {
                    if !(-9999..=9999).contains(&date.year)
                        || icu_datetime::input::Date::try_new_iso(date.year, date.month, date.day)
                            .is_err()
                    {
                        return Err(LocalizationError::Argument(name.clone()));
                    }
                }
            }
            if argument_bytes > MAX_OUTPUT_BYTES {
                return Err(invalid("arguments", "text arguments exceed 64 KiB"));
            }
        }
        let table = self
            .tables
            .get(&request.table_id)
            .ok_or_else(|| LocalizationError::Table(request.table_id.clone()))?;
        let resolved = table.messages.get(&request.key).and_then(|translations| {
            candidates(settings, &table.source_locale)
                .into_iter()
                .find_map(|language| translations.get(&language).map(|nodes| (language, nodes)))
        });
        let Some((resolved_locale, nodes)) = resolved else {
            return Ok(LocalizedText {
                text: format!("⟦{}:{}⟧", request.table_id, request.key),
                requested_locale: settings.locale.clone(),
                resolved_locale: None,
                missing: Some(MissingString {
                    table_id: request.table_id.clone(),
                    key: request.key.clone(),
                    requested_locale: settings.locale.clone(),
                    resolved_locale: None,
                    kind: MissingKind::Key,
                }),
            });
        };
        let mut text = if settings.pseudo {
            "[!! ".to_string()
        } else {
            String::new()
        };
        format::evaluate(
            nodes,
            &resolved_locale,
            &request.arguments,
            settings.pseudo,
            None,
            &mut text,
        )?;
        if settings.pseudo {
            text.push_str(" !!]");
            if text.len() > MAX_OUTPUT_BYTES {
                return Err(LocalizationError::OutputLimit);
            }
        }
        let missing = (resolved_locale != settings.locale).then(|| MissingString {
            table_id: request.table_id.clone(),
            key: request.key.clone(),
            requested_locale: settings.locale.clone(),
            resolved_locale: Some(resolved_locale.clone()),
            kind: MissingKind::Translation,
        });
        Ok(LocalizedText {
            text,
            requested_locale: settings.locale.clone(),
            resolved_locale: Some(resolved_locale),
            missing,
        })
    }
    /// A deterministic coverage report; empty translations are intentional values.
    /// Parameter evaluation is unnecessary, so this includes every declared key.
    pub fn missing_strings(
        &self,
        settings: &LocaleSettings,
    ) -> Result<Vec<MissingString>, LocalizationError> {
        settings.validate()?;
        let mut missing = vec![];
        for (id, table) in &self.tables {
            let chain = candidates(settings, &table.source_locale);
            for (name, translations) in &table.messages {
                if translations.contains_key(&settings.locale) {
                    continue;
                }
                let resolved = chain
                    .iter()
                    .find(|language| translations.contains_key(*language))
                    .cloned();
                missing.push(MissingString {
                    table_id: id.clone(),
                    key: name.clone(),
                    requested_locale: settings.locale.clone(),
                    resolved_locale: resolved,
                    kind: MissingKind::Translation,
                });
            }
        }
        Ok(missing)
    }
}
fn candidates(settings: &LocaleSettings, source: &str) -> Vec<String> {
    let mut result = vec![];
    let mut seen = BTreeSet::new();
    for requested in std::iter::once(settings.locale.as_str())
        .chain(settings.fallbacks.iter().map(String::as_str))
        .chain(std::iter::once(source))
    {
        if seen.insert(requested.to_string()) {
            result.push(requested.to_string());
        }
        let mut parent = locale(requested).expect("validated locale").id.to_string();
        loop {
            if seen.insert(parent.clone()) {
                result.push(parent.clone());
            }
            let Some(index) = parent.rfind('-') else {
                break;
            };
            parent.truncate(index);
        }
    }
    result
}
