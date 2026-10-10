//! Authoring-side scene cooking. The runtime depends on neither this crate nor
//! incant_doc. Unsupported components fail explicitly until their migration.
use incant_doc::{Project, Transform, Velocity};
use incant_runtime::{CookedEntity, CookedScene, SceneError, StableId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CookError {
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error(transparent)]
    Scene(#[from] SceneError),
    #[error("scene {0} does not exist")]
    MissingScene(String),
    #[error("component {component} on {entity} has not migrated to the cooked runtime")]
    UnsupportedComponent { entity: String, component: String },
    #[error("invalid stable ID {0}")]
    InvalidId(String),
    #[error("component decoding failed: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Validate authoring once, then convert a selected scene to a checked, canonical
/// binary image. Missing Transform projects to identity, as in the existing ECS.
pub fn cook_scene(project: &Project, scene_id: &str) -> Result<Vec<u8>, CookError> {
    let validated = project.validated()?;
    let project = validated.project();
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| CookError::MissingScene(scene_id.into()))?;
    let mut entities = Vec::with_capacity(scene.entities.len());
    for entity in scene.entities.values() {
        for component in entity.components.keys() {
            if component != "Transform" && component != "Velocity" {
                return Err(CookError::UnsupportedComponent {
                    entity: entity.id.clone(),
                    component: component.clone(),
                });
            }
        }
        let transform: Transform = entity
            .components
            .get("Transform")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();
        let velocity: Option<Velocity> = entity
            .components
            .get("Velocity")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;
        entities.push(CookedEntity {
            id: stable_id(&entity.id)?,
            parent: entity.parent.as_deref().map(stable_id).transpose()?,
            transform,
            velocity,
        });
    }
    Ok(CookedScene::new(stable_id(&scene.id)?, project.settings.tick_rate, entities)?.to_bytes())
}

pub fn stable_id(id: &str) -> Result<StableId, CookError> {
    id.parse::<ulid::Ulid>()
        .map(|id| StableId(id.to_bytes()))
        .map_err(|_| CookError::InvalidId(id.into()))
}
