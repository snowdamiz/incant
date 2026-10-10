//! Read-only localization queries use the same fixed-tick budget as physics.
use crate::{ScriptError, ScriptHost};
use incant_doc::Project;
use incant_localization::{
    CalendarDate, Catalog, DateLength, LocaleSettings, LocalizationError, LocalizeRequest,
    StringTable,
};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::atomic::Ordering, time::Instant};

#[derive(Default)]
pub(super) struct State {
    tables: BTreeMap<String, StringTable>,
    catalog: Catalog,
    settings: LocaleSettings,
}
impl State {
    pub(super) fn prepare(&mut self, project: &Project) -> Result<(), LocalizationError> {
        if self.tables != project.string_tables {
            let catalog = Catalog::compile(&project.string_tables)?;
            self.catalog = catalog;
            self.tables = project.string_tables.clone();
        }
        project.settings.localization.validate()?;
        self.settings = project.settings.localization.clone();
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Query {
    Message {
        request: LocalizeRequest,
    },
    Number {
        value: f64,
    },
    Date {
        value: CalendarDate,
        length: DateLength,
    },
    Settings,
    Coverage,
}
impl ScriptHost {
    pub(super) fn install_localization(&self) -> Result<(), ScriptError> {
        let state = self.localization.clone();
        let count = self.query_count.clone();
        let deadline = self.deadline.clone();
        self.context
            .with(|ctx| {
                let function =
                    rquickjs::Function::new(ctx.clone(), move |text: String| -> String {
                        let response = (|| -> Result<serde_json::Value, String> {
                            if text.len() > 128 * 1024
                                || count.fetch_add(4, Ordering::Relaxed) > 252
                                || Instant::now()
                                    >= *deadline.lock().unwrap_or_else(|e| e.into_inner())
                            {
                                return Err("localization query budget exceeded".into());
                            }
                            let request: Query = serde_json::from_str(&text)
                                .map_err(|e| format!("invalid localization query: {e}"))?;
                            let state = state.lock().unwrap_or_else(|e| e.into_inner());
                            let value = match request {
                                Query::Message { request } => serde_json::to_value(
                                    state
                                        .catalog
                                        .localize(&state.settings, &request)
                                        .map_err(|e| e.to_string())?,
                                ),
                                Query::Number { value } => serde_json::to_value(
                                    incant_localization::format_number(
                                        &state.settings.locale,
                                        value,
                                    )
                                    .map_err(|e| e.to_string())?,
                                ),
                                Query::Date { value, length } => serde_json::to_value(
                                    incant_localization::format_date(
                                        &state.settings.locale,
                                        &value,
                                        length,
                                    )
                                    .map_err(|e| e.to_string())?,
                                ),
                                Query::Settings => serde_json::to_value(&state.settings),
                                Query::Coverage => serde_json::to_value(
                                    state
                                        .catalog
                                        .missing_strings(&state.settings)
                                        .map_err(|e| e.to_string())?,
                                ),
                            }
                            .map_err(|e| e.to_string())?;
                            if Instant::now() >= *deadline.lock().unwrap_or_else(|e| e.into_inner())
                            {
                                return Err("localization query deadline exceeded".into());
                            }
                            Ok(value)
                        })();
                        match response {
                            Ok(value) => serde_json::json!({"value":value}).to_string(),
                            Err(error) => serde_json::json!({"error":error}).to_string(),
                        }
                    })?;
                ctx.globals().set("__incantLocalization", function)
            })
            .map_err(|_| ScriptError::Execution)
    }
}
