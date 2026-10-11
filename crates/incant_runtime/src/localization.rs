//! Immutable bounded catalog and cheap committed-locale snapshots. Message
//! parsing and coverage preparation happen on load or a locale command, never
//! while querying borrowed ECS columns.
use crate::{FrameCommands, NativeWorld, SceneError, StructuralCommand};
use incant_localization::{
    Catalog, LocaleSettings, LocalizationReport, LocalizationReportRequest, LocalizeRequest,
    LocalizedText, MissingString, StringTable,
};
use std::{collections::BTreeMap, sync::Arc};

struct Prepared {
    tables: BTreeMap<String, StringTable>,
    catalog: Catalog,
}
#[derive(Clone)]
pub struct NativeLocalization {
    data: Arc<Prepared>,
    settings: Arc<LocaleSettings>,
    coverage: Arc<Vec<MissingString>>,
}
impl std::fmt::Debug for NativeLocalization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeLocalization")
            .field("tables", &self.data.tables.len())
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}
pub(crate) fn error(e: impl std::fmt::Display) -> SceneError {
    SceneError::Localization(e.to_string())
}
impl Default for NativeLocalization {
    fn default() -> Self {
        Self::new(Default::default(), Default::default()).expect("empty localization")
    }
}
impl NativeLocalization {
    pub fn new(
        tables: BTreeMap<String, StringTable>,
        settings: LocaleSettings,
    ) -> Result<Self, SceneError> {
        for id in tables.keys() {
            if id.len() != 26
                || id.as_bytes()[0] > b'7'
                || !id
                    .bytes()
                    .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
            {
                return Err(SceneError::Invalid("localization table ID"));
            }
        }
        let catalog = Catalog::compile(&tables).map_err(error)?;
        let coverage = catalog.missing_strings(&settings).map_err(error)?;
        let result = Self {
            data: Arc::new(Prepared { tables, catalog }),
            settings: Arc::new(settings),
            coverage: Arc::new(coverage),
        };
        if crate::localization_binary::size(&result) > crate::localization_binary::MAX_BYTES {
            return Err(SceneError::Invalid("localization binary size"));
        }
        Ok(result)
    }
    pub(crate) fn with_settings(&self, settings: LocaleSettings) -> Result<Self, SceneError> {
        settings.validate().map_err(error)?;
        if settings == *self.settings {
            return Ok(self.clone());
        }
        let coverage = self
            .data
            .catalog
            .missing_strings(&settings)
            .map_err(error)?;
        Ok(Self {
            data: Arc::clone(&self.data),
            settings: Arc::new(settings),
            coverage: Arc::new(coverage),
        })
    }
    pub fn settings(&self) -> &LocaleSettings {
        &self.settings
    }
    pub(crate) fn tables(&self) -> &BTreeMap<String, StringTable> {
        &self.data.tables
    }
    pub(crate) fn is_default(&self) -> bool {
        self.data.tables.is_empty() && self.settings.is_default()
    }
    pub fn localize(&self, request: &LocalizeRequest) -> Result<LocalizedText, SceneError> {
        self.data
            .catalog
            .localize(&self.settings, request)
            .map_err(error)
    }
    pub fn report(
        &self,
        request: LocalizationReportRequest,
    ) -> Result<LocalizationReport, SceneError> {
        if request.offset > incant_localization::MAX_MESSAGES as u32
            || !(1..=128).contains(&request.limit)
        {
            return Err(SceneError::Invalid("localization report bounds"));
        }
        Ok(LocalizationReport {
            total: self.coverage.len() as u32,
            entries: self
                .coverage
                .iter()
                .skip(request.offset as usize)
                .take(request.limit as usize)
                .cloned()
                .collect(),
        })
    }
}
impl NativeWorld {
    pub fn localization(&self) -> &NativeLocalization {
        &self.localization
    }
    pub(crate) fn prepare_locale(
        &self,
        commands: &FrameCommands,
    ) -> Result<Option<NativeLocalization>, SceneError> {
        let mut settings = None;
        for command in commands.iter() {
            if let StructuralCommand::SetLocale(value) = command {
                value.validate().map_err(error)?;
                settings = Some(value);
            }
        }
        settings
            .map(|s| self.localization.with_settings(s.clone()))
            .transpose()
    }
}
