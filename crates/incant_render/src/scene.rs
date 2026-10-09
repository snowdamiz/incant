//! Bind immutable cooked versions to the ECS projection before publishing GPU state.
use glam::Mat4;
use incant_assets::{AssetStore, RuntimeAsset, RuntimeAssetData};
use incant_doc::Project;
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;

const MAX_INSTANCES: usize = 100_000;

#[derive(Debug, Error)]
pub enum SceneError {
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error("model asset {0} is not loaded at the document's current fingerprint")]
    MissingModel(String),
    #[error("authored material asset overrides are not supported by the imported preview")]
    MaterialOverrides,
    #[error("model {0} has no scene to instantiate")]
    NoScene(String),
    #[error("model {0} contains repeated roots or nodes in its selected scene")]
    RepeatedNode(String),
    #[error("world transform exceeds renderer numeric range")]
    TransformRange,
    #[error("scene exceeds {MAX_INSTANCES} primitive instances")]
    InstanceLimit,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SceneStats {
    pub diagnostic_entities: usize,
    pub model_entities: usize,
    pub primitive_instances: usize,
    pub model_triangles: u64,
    pub model_draw_calls: usize,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance {
    pub world: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
}
impl Instance {
    fn new(world: Mat4) -> Result<Self, SceneError> {
        let inverse = world.inverse();
        if !world.is_finite() || !inverse.is_finite() {
            return Err(SceneError::TransformRange);
        }
        let mut columns = inverse.transpose().to_cols_array_2d();
        columns[0][3] = if world.as_dmat4().determinant() < 0. {
            -1.
        } else {
            1.
        };
        Ok(Self {
            world: world.to_cols_array_2d(),
            normal: [columns[0], columns[1], columns[2]],
        })
    }
}

pub(crate) struct ModelPlan {
    pub source: Arc<RuntimeAsset>,
    pub primitives: BTreeMap<usize, Vec<Instance>>,
}
pub(crate) struct ResolvedScene {
    pub diagnostics: Vec<Mat4>,
    pub models: BTreeMap<String, ModelPlan>,
    pub stats: SceneStats,
}

/// A model's default scene is used, falling back to its first declared scene.
/// Unreferenced library meshes/nodes are not instantiated. Reused primitives are
/// grouped by cooked fingerprint so one GPU draw can instance many entities.
pub(crate) fn resolve(
    project: &Project,
    assets: Option<&AssetStore>,
) -> Result<ResolvedScene, SceneError> {
    let state = incant_core::Engine::new(project)?.snapshot();
    let mut result = ResolvedScene {
        diagnostics: Vec::new(),
        models: BTreeMap::new(),
        stats: SceneStats::default(),
    };
    for entity in state.entities.values() {
        let world64 = glam::DMat4::from_cols_array_2d(&entity.world_transform);
        let world = world64.as_mat4();
        // Validate even empty mesh nodes so invalid ranges never reach GPU buffers.
        Instance::new(world)?;
        let Some(binding) = &entity.mesh else {
            if project.scenes[&entity.scene_id].entities[&entity.id]
                .components
                .contains_key("Transform")
            {
                result.diagnostics.push(world);
                result.stats.diagnostic_entities += 1;
            }
            continue;
        };
        if !binding.materials.is_empty() {
            return Err(SceneError::MaterialOverrides);
        }
        let registered = &project.assets[&binding.mesh];
        let source = assets
            .and_then(|assets| assets.get(&binding.mesh))
            .filter(|asset| {
                asset.info().fingerprint == registered.sha256 && asset.info().kind == "model"
            })
            .ok_or_else(|| SceneError::MissingModel(binding.mesh.clone()))?;
        let RuntimeAssetData::Model(model) = source.data() else {
            return Err(SceneError::MissingModel(binding.mesh.clone()));
        };
        let scene = model.metadata.default_scene.unwrap_or(0);
        let roots = model
            .metadata
            .scenes
            .get(scene)
            .ok_or_else(|| SceneError::NoScene(binding.mesh.clone()))?;
        let mut pending: Vec<_> = roots.iter().rev().map(|&index| (index, world64)).collect();
        let mut visited = vec![false; model.metadata.nodes.len()];
        let key = source.info().fingerprint.clone();
        let plan = result.models.entry(key).or_insert_with(|| ModelPlan {
            source: Arc::clone(&source),
            primitives: BTreeMap::new(),
        });
        result.stats.model_entities += 1;
        while let Some((index, parent)) = pending.pop() {
            if std::mem::replace(&mut visited[index], true) {
                return Err(SceneError::RepeatedNode(binding.mesh.clone()));
            }
            let node = &model.metadata.nodes[index];
            let world = parent
                * glam::DMat4::from_cols_array_2d(
                    &node.transform.map(|column| column.map(f64::from)),
                );
            let instance = Instance::new(world.as_mat4())?;
            for &primitive in &node.meshes {
                result.stats.primitive_instances += 1;
                if result.stats.primitive_instances > MAX_INSTANCES {
                    return Err(SceneError::InstanceLimit);
                }
                result.stats.model_triangles += (model.meshes[primitive].indices.len() / 3) as u64;
                plan.primitives.entry(primitive).or_default().push(instance);
            }
            pending.extend(node.children.iter().rev().map(|&child| (child, world)));
        }
    }
    result.stats.model_draw_calls = result.models.values().map(|m| m.primitives.len()).sum();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support as support;

    #[test]
    fn model_nodes_use_default_scene_and_repeated_entities_share_one_draw() {
        let temp = tempfile::tempdir().unwrap();
        let (project, assets, _) = support::fixture(temp.path());
        let scene = resolve(&project, Some(&assets)).unwrap();
        assert_eq!(scene.stats.model_entities, 2);
        assert_eq!(scene.stats.primitive_instances, 2);
        assert_eq!(scene.stats.model_draw_calls, 1);
        assert_eq!(scene.stats.model_triangles, 2);
        assert_eq!(scene.stats.diagnostic_entities, 0);
        let model = scene.models.values().next().unwrap();
        let instances = &model.primitives[&0];
        let mut origins: Vec<_> = instances.iter().map(|i| i.world[3]).collect();
        origins.sort_by(|a, b| a[0].total_cmp(&b[0]));
        assert_eq!(origins, vec![[-2., 2., 0., 1.], [1., 2., 0., 1.]]);
    }

    #[test]
    fn unloaded_or_stale_models_fail_instead_of_falling_back_to_cubes() {
        let temp = tempfile::tempdir().unwrap();
        let (mut project, assets, id) = support::fixture(temp.path());
        assert!(matches!(
            resolve(&project, None),
            Err(SceneError::MissingModel(_))
        ));
        project.assets.get_mut(&id).unwrap().sha256 = "0".repeat(64);
        assert!(matches!(
            resolve(&project, Some(&assets)),
            Err(SceneError::MissingModel(_))
        ));
    }
}
