use crate::scene::PreparedEntity;
use incant_doc::{
    Collider, Diagnostic, DocumentError, NavigationMesh as AuthoredMesh, NavigationSourceKind,
    Project,
};
pub use incant_nav::{
    NavigationError, NavigationPath, NavigationResources, PathRequest, RebuildReport,
};
use incant_nav::{NavigationGeometry, NavigationMesh};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationQuery {
    pub scene_id: String,
    pub mesh_entity: String,
    pub path: PathRequest,
}
#[derive(Clone)]
struct Entry {
    scene_id: String,
    stamp: String,
    mesh: Arc<NavigationMesh>,
    report: RebuildReport,
}
#[derive(Clone, Default)]
pub(crate) struct NavigationRuntime {
    entries: BTreeMap<String, Entry>,
    generation: u64,
}
impl NavigationRuntime {
    pub fn reports(&self) -> BTreeMap<String, RebuildReport> {
        self.entries
            .iter()
            .map(|(id, e)| (id.clone(), e.report.clone()))
            .collect()
    }
    pub fn find_path(
        &self,
        query: NavigationQuery,
    ) -> Result<Option<NavigationPath>, NavigationError> {
        let entry = self
            .entries
            .get(&query.mesh_entity)
            .filter(|e| e.scene_id == query.scene_id)
            .ok_or_else(|| {
                NavigationError::Invalid(
                    "navigation mesh is absent from the requested scene".into(),
                )
            })?;
        entry.mesh.find_path(&query.path).map(|path| {
            path.map(|mut path| {
                path.generation = entry.report.generation;
                path
            })
        })
    }
    /// Stage every mesh before physics or ECS changes. Unchanged source stamps
    /// share the old mesh without rerasterizing or copying its polygon graph.
    pub fn prepare(
        &self,
        project: &Project,
        prepared: &[PreparedEntity],
        resources: &NavigationResources,
    ) -> Result<Self, DocumentError> {
        let worlds: BTreeMap<_, _> = prepared
            .iter()
            .map(|e| (e.id.as_str(), e.world.to_cols_array_2d()))
            .collect();
        let mut next = Self {
            entries: BTreeMap::new(),
            generation: self.generation,
        };
        for scene in project.scenes.values() {
            for entity in scene.entities.values() {
                let Some(value) = entity.components.get("NavigationMesh") else {
                    continue;
                };
                let authored: AuthoredMesh = serde_json::from_value(value.clone())?;
                let failure = |error: NavigationError| {
                    DocumentError::Validation(vec![Diagnostic {
                        path: format!(
                            "/scenes/{}/entities/{}/components/NavigationMesh",
                            scene.id, entity.id
                        ),
                        message: error.to_string(),
                    }])
                };
                let mut stamps = Vec::new();
                for source in &authored.sources {
                    let input = &scene.entities[&source.entity];
                    let fingerprint = if source.geometry == NavigationSourceKind::Mesh {
                        let binding: incant_doc::MeshRenderer =
                            serde_json::from_value(input.components["MeshRenderer"].clone())?;
                        let registered = &project.assets[&binding.mesh];
                        if !resources
                            .get(&binding.mesh)
                            .is_some_and(|asset| asset.fingerprint == registered.sha256)
                        {
                            return Err(failure(NavigationError::Invalid(format!(
                                "missing or stale navigation resource {}",
                                binding.mesh
                            ))));
                        }
                        Some(registered.sha256.clone())
                    } else {
                        None
                    };
                    let component = match source.geometry {
                        NavigationSourceKind::Collider => &input.components["Collider"],
                        NavigationSourceKind::Mesh => &input.components["MeshRenderer"],
                    };
                    stamps.push(serde_json::json!([
                        source,
                        worlds[source.entity.as_str()],
                        component,
                        fingerprint
                    ]));
                }
                let stamp = serde_json::to_string(&(&scene.id, &authored.settings, stamps))?;
                let old = self.entries.get(&entity.id);
                if let Some(old) = old.filter(|old| old.stamp == stamp) {
                    let mut entry = old.clone();
                    entry.report.reused.append(&mut entry.report.rebuilt);
                    entry.report.reused.sort();
                    entry.report.removed.clear();
                    next.entries.insert(entity.id.clone(), entry);
                    continue;
                }
                let mut geometry = BTreeMap::new();
                for source in &authored.sources {
                    let input = &scene.entities[&source.entity];
                    let local: NavigationGeometry = match source.geometry {
                        NavigationSourceKind::Collider => {
                            let collider: Collider =
                                serde_json::from_value(input.components["Collider"].clone())?;
                            incant_physics::navigation_geometry(&collider.shape).map_err(failure)?
                        }
                        NavigationSourceKind::Mesh => {
                            let binding: incant_doc::MeshRenderer =
                                serde_json::from_value(input.components["MeshRenderer"].clone())?;
                            let asset = resources
                                .get(&binding.mesh)
                                .filter(|a| a.fingerprint == project.assets[&binding.mesh].sha256)
                                .ok_or_else(|| {
                                    failure(NavigationError::Invalid(format!(
                                        "missing or stale navigation resource {}",
                                        binding.mesh
                                    )))
                                })?;
                            (*asset.geometry().map_err(failure)?).clone()
                        }
                    };
                    geometry.insert(
                        format!("{}:{:?}", source.entity, source.geometry),
                        local
                            .transformed(worlds[source.entity.as_str()])
                            .map_err(failure)?,
                    );
                }
                let mut mesh = old.map_or_else(NavigationMesh::default, |e| (*e.mesh).clone());
                let mut report = mesh
                    .rebuild(&authored.settings, &geometry)
                    .map_err(failure)?;
                if let Some(old) = old.filter(|old| old.mesh.generation() == mesh.generation()) {
                    report.generation = old.report.generation;
                } else {
                    next.generation = next.generation.checked_add(1).ok_or_else(|| {
                        failure(NavigationError::Limit(
                            "runtime generation exhausted".into(),
                        ))
                    })?;
                    report.generation = next.generation;
                }
                next.entries.insert(
                    entity.id.clone(),
                    Entry {
                        scene_id: scene.id.clone(),
                        stamp,
                        mesh: Arc::new(mesh),
                        report,
                    },
                );
            }
        }
        Ok(next)
    }
}
