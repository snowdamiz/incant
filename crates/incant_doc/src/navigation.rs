//! Explicit sources keep decorative meshes, sensors and moving physics bodies
//! out of the bake. Navigation owns derived data only; sources stay ordinary entities.
use crate::{BodyMotion, Collider, MeshRenderer, Project, RigidBody};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationMesh {
    pub settings: incant_nav::NavigationSettings,
    #[schemars(length(max = 1024))]
    pub sources: Vec<NavigationSource>,
    #[serde(default)]
    #[schemars(length(max = 128))]
    pub links: Vec<incant_nav::OffMeshLink>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationSource {
    pub entity: String,
    pub geometry: NavigationSourceKind,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NavigationSourceKind {
    Collider,
    Mesh,
}
pub(crate) fn validate(value: &serde_json::Value) -> Result<(), String> {
    let mesh: NavigationMesh = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    mesh.settings.validate().map_err(|e| e.to_string())?;
    incant_nav::validate_links(&mesh.links).map_err(|e| e.to_string())?;
    if mesh.sources.len() > 1024 {
        return Err("navigation mesh exceeds 1024 sources".into());
    }
    let mut seen = BTreeSet::new();
    for source in &mesh.sources {
        if ulid::Ulid::from_string(&source.entity).is_err()
            || !seen.insert((&source.entity, source.geometry))
        {
            return Err(
                "navigation sources require unique entity/geometry pairs with ULIDs".into(),
            );
        }
    }
    Ok(())
}
pub(crate) fn validate_graph(project: &Project) -> Result<(), String> {
    let mut count = 0;
    let mut grids = 0;
    let mut grid_cells = 0;
    for scene in project.scenes.values() {
        for entity in scene.entities.values() {
            if let Some(value) = entity.components.get("NavigationGrid") {
                let grid: incant_nav::NavigationGrid =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                grids += 1;
                grid_cells += grid.costs.len();
                if grids > 8 || grid_cells > incant_nav::MAX_GRID_CELLS {
                    return Err(
                        "project exceeds eight navigation grids or 65536 total grid cells".into(),
                    );
                }
            }
            let Some(value) = entity.components.get("NavigationMesh") else {
                continue;
            };
            count += 1;
            if count > 8 {
                return Err("project exceeds eight navigation meshes".into());
            }
            let mesh: NavigationMesh =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            for source in mesh.sources {
                let entity = scene
                    .entities
                    .get(&source.entity)
                    .ok_or("navigation source must belong to the same scene")?;
                let mut ancestor = Some(entity);
                let mut visited = BTreeSet::new();
                while let Some(input) = ancestor {
                    if !visited.insert(&input.id) {
                        return Err("cyclic navigation source ancestry".into());
                    }
                    if let Some(value) = input.components.get("RigidBody") {
                        let body: RigidBody =
                            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                        if body.motion != BodyMotion::Fixed {
                            return Err("navigation sources and ancestors cannot be dynamic or kinematic bodies".into());
                        }
                    }
                    if let Some(value) = input.components.get("Velocity") {
                        let velocity: crate::Velocity =
                            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                        if velocity.linear != [0.; 3] {
                            return Err("navigation sources and ancestors must have zero simulation velocity; move assembled rooms through Transform commands".into());
                        }
                    }
                    ancestor = input
                        .parent
                        .as_ref()
                        .map(|id| {
                            scene
                                .entities
                                .get(id)
                                .ok_or("navigation source parent is absent")
                        })
                        .transpose()?;
                }
                match source.geometry {
                    NavigationSourceKind::Collider => {
                        let collider: Collider = serde_json::from_value(
                            entity
                                .components
                                .get("Collider")
                                .ok_or("navigation collider source requires Collider")?
                                .clone(),
                        )
                        .map_err(|e| e.to_string())?;
                        if collider.sensor {
                            return Err("navigation sources cannot be sensors".into());
                        }
                    }
                    NavigationSourceKind::Mesh => {
                        let _: MeshRenderer = serde_json::from_value(
                            entity
                                .components
                                .get("MeshRenderer")
                                .ok_or("navigation mesh source requires MeshRenderer")?
                                .clone(),
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
    Ok(())
}
