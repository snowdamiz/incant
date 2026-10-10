use incant_doc::{Diagnostic, DocumentError, Project};
pub use incant_nav::{GridPath, GridPathRequest};
use incant_nav::{NavigationError, NavigationGrid};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GridNavigationQuery {
    pub scene_id: String,
    pub grid_entity: String,
    pub path: GridPathRequest,
}
struct Entry {
    scene: String,
    authored: serde_json::Value,
    grid: NavigationGrid,
    generation: u64,
}
#[derive(Clone, Default)]
pub(crate) struct GridNavigationRuntime {
    entries: BTreeMap<String, Arc<Entry>>,
    generation: u64,
}
impl GridNavigationRuntime {
    pub fn find_path(
        &self,
        query: GridNavigationQuery,
    ) -> Result<Option<GridPath>, NavigationError> {
        let entry = self
            .entries
            .get(&query.grid_entity)
            .filter(|entry| entry.scene == query.scene_id)
            .ok_or_else(|| {
                NavigationError::Invalid(
                    "navigation grid is absent from the requested scene".into(),
                )
            })?;
        Ok(entry.grid.find_path(&query.path)?.map(|mut path| {
            path.generation = entry.generation;
            path
        }))
    }
    /// Derived snapshots publish only after the entire engine sync validates.
    pub fn prepare(&self, project: &Project) -> Result<Self, DocumentError> {
        let mut next = Self {
            entries: BTreeMap::new(),
            generation: self.generation,
        };
        let mut changed = false;
        let bump = |generation: &mut u64| -> Result<(), DocumentError> {
            *generation = generation.checked_add(1).ok_or_else(|| {
                DocumentError::Validation(vec![Diagnostic {
                    path: "/scenes".into(),
                    message: "grid navigation generation exhausted".into(),
                }])
            })?;
            Ok(())
        };
        for scene in project.scenes.values() {
            for entity in scene.entities.values() {
                let Some(value) = entity.components.get("NavigationGrid") else {
                    continue;
                };
                let entry = if let Some(old) = self
                    .entries
                    .get(&entity.id)
                    .filter(|e| e.scene == scene.id && e.authored == *value)
                {
                    old.clone()
                } else {
                    let grid: NavigationGrid = serde_json::from_value(value.clone())?;
                    grid.validate().map_err(|error| {
                        DocumentError::Validation(vec![Diagnostic {
                            path: format!(
                                "/scenes/{}/entities/{}/components/NavigationGrid",
                                scene.id, entity.id
                            ),
                            message: error.to_string(),
                        }])
                    })?;
                    if !changed {
                        bump(&mut next.generation)?;
                        changed = true;
                    }
                    Arc::new(Entry {
                        scene: scene.id.clone(),
                        authored: value.clone(),
                        grid,
                        generation: next.generation,
                    })
                };
                next.entries.insert(entity.id.clone(), entry);
            }
        }
        if !changed && self.entries.keys().any(|id| !next.entries.contains_key(id)) {
            bump(&mut next.generation)?;
        }
        Ok(next)
    }
}
