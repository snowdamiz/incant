use crate::{DocumentError, Project};
use loro::{ExportMode, LoroDoc, LoroMap, LoroValue, ToJson};
use serde_json::Value;

/// Nested maps merge at field granularity; vectors/arrays are atomic values.
/// Import on a fork first: invalid remote state can never poison the live document.
pub struct CollaborativeDocument {
    doc: LoroDoc,
}
impl CollaborativeDocument {
    pub fn new(project: &Project) -> Result<Self, DocumentError> {
        project.validate()?;
        let this = Self {
            doc: LoroDoc::new(),
        };
        update_map(
            &this.doc.get_map("project"),
            &serde_json::to_value(project)?,
        )?;
        this.doc.commit();
        Ok(this)
    }
    pub fn from_export(bytes: &[u8]) -> Result<Self, DocumentError> {
        let this = Self {
            doc: LoroDoc::new(),
        };
        this.doc
            .import(bytes)
            .map_err(|e| DocumentError::Crdt(e.to_string()))?;
        this.project()?;
        Ok(this)
    }
    pub fn project(&self) -> Result<Project, DocumentError> {
        let p: Project =
            serde_json::from_value(self.doc.get_map("project").get_deep_value().to_json_value())?;
        p.validate()?;
        Ok(p)
    }
    pub fn fork(&self) -> Self {
        Self {
            doc: self.doc.fork(),
        }
    }
    pub fn replace(&mut self, project: &Project) -> Result<(), DocumentError> {
        project.validate()?;
        if project.id != self.project()?.id {
            return Err(DocumentError::Crdt("cannot change project identity".into()));
        }
        let candidate = self.doc.fork();
        update_map(
            &candidate.get_map("project"),
            &serde_json::to_value(project)?,
        )?;
        candidate.commit();
        self.doc = candidate;
        Ok(())
    }
    pub fn export(&self) -> Result<Vec<u8>, DocumentError> {
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|e| DocumentError::Crdt(e.to_string()))
    }
    pub fn merge(&mut self, bytes: &[u8]) -> Result<(), DocumentError> {
        let candidate = Self {
            doc: self.doc.fork(),
        };
        candidate
            .doc
            .import(bytes)
            .map_err(|e| DocumentError::Crdt(e.to_string()))?;
        let project = candidate.project()?;
        if project.id != self.project()?.id {
            return Err(DocumentError::Crdt(
                "remote project identity mismatch".into(),
            ));
        }
        self.doc = candidate.doc;
        Ok(())
    }
}
fn update_map(map: &LoroMap, value: &Value) -> Result<(), DocumentError> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Crdt("expected object".into()))?;
    let current = map.get_deep_value().to_json_value();
    if let Some(old) = current.as_object() {
        for key in old.keys().filter(|key| !object.contains_key(*key)) {
            map.delete(key).map_err(err)?;
        }
    }
    for (key, value) in object {
        if current.get(key) == Some(value) {
            continue;
        }
        if value.is_object() {
            // A deliberate type change replaces the slot. Independent lazy creation
            // uses mergeable maps so concurrent edits don't lose a whole component.
            if current.get(key).is_some_and(|v| !v.is_object()) {
                map.delete(key).map_err(err)?;
            }
            let child = map.ensure_mergeable_map(key).map_err(err)?;
            update_map(&child, value)?;
        } else {
            let value: LoroValue = serde_json::from_value(value.clone())?;
            map.insert(key, value).map_err(err)?;
        }
    }
    Ok(())
}
fn err(e: impl std::fmt::Display) -> DocumentError {
    DocumentError::Crdt(e.to_string())
}
