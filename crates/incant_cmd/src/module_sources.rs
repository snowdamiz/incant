//! Registered source reads share the same held root/journal as all source writes.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleSourceRead {
    pub module_id: Id,
    pub path: String,
    pub revision: u64,
    pub source: Option<String>,
    pub version: SourceVersion,
    pub matches_history: Option<bool>,
}
impl CommandBus {
    pub fn read_module_source(&self, module_id: &str) -> Result<ModuleSourceRead, CommandError> {
        self.ensure_sources_ready()?;
        let store = self.sources.as_ref().ok_or(SourceError::Unavailable)?;
        let module = self
            .project
            .module_sources
            .get(module_id)
            .ok_or_else(|| CommandError::Invalid("module source does not exist".into()))?;
        incant_source::validate_rust_path(&module.path)?;
        let source = store.read(&module.path)?;
        if let Some(text) = &source {
            incant_source::SourceKind::Rust.validate_source(text)?;
        }
        let last = self
            .history
            .iter()
            .rev()
            .filter_map(|tx| tx.source_edit.as_ref())
            .find(|edit| edit.path == module.path);
        Ok(ModuleSourceRead {
            module_id: module.id.clone(),
            path: module.path.clone(),
            revision: self.revision,
            version: SourceVersion::of(source.as_deref()),
            matches_history: last.map(|edit| edit.after == source),
            source,
        })
    }
}
