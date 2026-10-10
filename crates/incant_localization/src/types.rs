use crate::{LocalizationError, invalid};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A typed document asset. Message keys remain stable when translations change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StringTable {
    pub id: String,
    pub name: String,
    pub source_locale: String,
    /// Stable key -> canonical BCP-47 locale -> complete message pattern.
    pub messages: BTreeMap<String, BTreeMap<String, String>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocaleSettings {
    pub locale: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub pseudo: bool,
}
impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            locale: "en".into(),
            fallbacks: vec![],
            pseudo: false,
        }
    }
}
impl LocaleSettings {
    pub fn validate(&self) -> Result<(), LocalizationError> {
        locale(&self.locale)?;
        if self.fallbacks.len() > 16 {
            return Err(invalid("fallbacks", "at most 16 locales"));
        }
        let mut seen = BTreeSet::new();
        for fallback in &self.fallbacks {
            locale(fallback)?;
            if !seen.insert(fallback) {
                return Err(invalid("fallbacks", "duplicate locale"));
            }
        }
        Ok(())
    }
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DateLength {
    Short,
    #[default]
    Medium,
    Long,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MessageArgument {
    Text(String),
    Number(f64),
    Date(CalendarDate),
}
pub type MessageArguments = BTreeMap<String, MessageArgument>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalizeRequest {
    pub table_id: String,
    pub key: String,
    #[serde(default)]
    pub arguments: MessageArguments,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissingKind {
    Translation,
    Key,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MissingString {
    pub table_id: String,
    pub key: String,
    pub requested_locale: String,
    pub resolved_locale: Option<String>,
    pub kind: MissingKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalizedText {
    pub text: String,
    pub requested_locale: String,
    pub resolved_locale: Option<String>,
    pub missing: Option<MissingString>,
}
pub(crate) fn locale(value: &str) -> Result<icu_locale::Locale, LocalizationError> {
    if value.len() > 128 {
        return Err(invalid("locale", "tag exceeds 128 bytes"));
    }
    let parsed: icu_locale::Locale = value
        .parse()
        .map_err(|_| invalid("locale", "expected a canonical BCP-47 tag"))?;
    if parsed.to_string() != value {
        return Err(invalid("locale", "expected a canonical BCP-47 tag"));
    }
    Ok(parsed)
}
pub(crate) fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
