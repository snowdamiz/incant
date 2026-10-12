//! Canonical GPU-independent resolution from typed, retained scene input.
use glam::Mat4;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use incant_render_data::{MAX_INSTANCES, SceneInput};
pub use incant_render_data::{SceneError, SceneStats};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance {
    pub world: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
}
impl Instance {
    pub(crate) fn new(world: Mat4) -> Result<Self, SceneError> {
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

#[derive(Clone, Copy)]
pub(crate) struct Placement {
    pub node: usize,
    pub local: glam::DMat4,
}
pub(crate) struct ModelPlan {
    pub source: Arc<RuntimeAsset>,
    pub primitives: BTreeMap<(usize, bool), Vec<Instance>>,
    pub placements: BTreeMap<(usize, bool), Vec<Placement>>,
}
pub(crate) struct ResolvedScene {
    pub diagnostics: Vec<Mat4>,
    pub models: BTreeMap<(String, incant_assets_runtime::AssetVersionId), ModelPlan>,
    pub stats: SceneStats,
    pub environment: Option<crate::environment::EnvironmentPlan>,
    pub lights: crate::lighting::scene::LightPlan,
    pub cameras: BTreeMap<String, crate::camera::CameraView>,
    pub sprites: Vec<crate::sprites::SpritePlan>,
    pub tilemaps: Vec<crate::sprites::TilemapPlan>,
    pub animations: Vec<crate::models::animation::Plan>,
    pub text: Option<crate::text::TextPlan>,
}

/// Resolve startup/revision input. Native moving-frame extraction will use a
/// retained topology/instance path; this entry point does not claim that migration.
pub(crate) fn resolve(input: &SceneInput) -> Result<ResolvedScene, SceneError> {
    crate::models::animation::validate_ik_capacity(input)?;
    if input.nodes.len() > 1_000_000 {
        return Err(SceneError::Input("node limit"));
    }
    // Text nodes carry their own transforms; count and check every handle now.
    let text = input
        .text
        .as_ref()
        .filter(|text| !text.nodes.is_empty())
        .map(crate::text::plan)
        .transpose()?;
    let mut result = ResolvedScene {
        diagnostics: Vec::new(),
        models: BTreeMap::new(),
        stats: SceneStats::default(),
        environment: None,
        lights: Default::default(),
        cameras: BTreeMap::new(),
        sprites: Vec::new(),
        tilemaps: Vec::new(),
        animations: Vec::new(),
        text: None,
    };
    if let Some(text) = text {
        result.stats.text_entities = text.nodes.len();
        result.stats.text_glyph_quads = text.quads;
        result.stats.text_missing_glyphs = text.missing;
        result.text = Some(text);
    }
    if let Some(environment) = &input.environment {
        if !(0.0..=100.).contains(&environment.intensity)
            || !(-360.0..=360.).contains(&environment.rotation_degrees)
        {
            return Err(SceneError::Input("environment range"));
        }
        if !matches!(environment.source.data(), RuntimeAssetData::Texture(_))
            || environment.source.usage() == Some(incant_assets_runtime::TextureUsage::Normal)
        {
            return Err(SceneError::MissingEnvironment(
                environment.source.info().id.clone(),
            ));
        }
        result.environment = Some(crate::environment::EnvironmentPlan {
            source: Arc::clone(&environment.source),
            intensity: environment.intensity as f32,
            rotation: environment.rotation_degrees.to_radians() as f32,
        });
    }
    let mut ids = BTreeSet::new();
    let mut tile_slots = 0usize;
    let mut animation_bytes = 0usize;
    for (entity_index, entity) in input.nodes.iter().enumerate() {
        if entity.id.is_empty() || entity.id.len() > 256 || !ids.insert(entity.id.as_str()) {
            return Err(SceneError::Input("empty, oversized or duplicate node ID"));
        }
        if usize::from(entity.mesh.is_some())
            + usize::from(entity.sprite.is_some())
            + usize::from(entity.tilemap.is_some())
            > 1
        {
            return Err(SceneError::Input(
                "ambiguous mesh, sprite or tilemap binding",
            ));
        }
        let world64 = entity.world;
        if !world64.is_finite() || world64.row(3) != glam::DVec4::W {
            return Err(SceneError::TransformRange);
        }
        if let Some(light) = &entity.light {
            result.lights.add_typed(light, world64)?;
        }
        if let Some(camera) = &entity.camera {
            result.cameras.insert(
                entity.id.clone(),
                crate::camera::CameraView::authored(camera, world64)?,
            );
        }
        let world = world64.as_mat4();
        Instance::new(world)?;
        if let Some(tilemap) = &entity.tilemap {
            tile_slots = tile_slots
                .checked_add(tilemap.cell_slots())
                .ok_or(SceneError::Input("tilemap slot limit"))?;
            if result.tilemaps.len() == incant_render_data::MAX_TILEMAPS
                || tile_slots > incant_render_data::MAX_TILE_SLOTS
            {
                return Err(SceneError::Input("tilemap slot or entity limit"));
            }
            result.stats.tilemap_entities += 1;
            result.stats.tile_cells += tilemap.cells().len();
            result.tilemaps.push(crate::sprites::TilemapPlan {
                node: entity_index,
                data: Arc::clone(tilemap),
                world: world64,
                id: entity.id.clone(),
            });
            continue;
        }
        if let Some(sprite) = &entity.sprite {
            if !matches!(sprite.source.data(), RuntimeAssetData::Texture(_))
                || sprite.source.usage() != Some(incant_assets_runtime::TextureUsage::Color)
            {
                return Err(SceneError::MissingSprite(sprite.source.info().id.clone()));
            }
            result.sprites.push(crate::sprites::SpritePlan {
                node: entity_index,
                source: Arc::clone(&sprite.source),
                sprite: sprite.sprite.clone(),
                frames: sprite.frames.clone(),
                frame: sprite.frame,
                world: world64,
                id: entity.id.clone(),
            });
            result.stats.sprite_entities += 1;
            continue;
        }
        let Some(binding) = &entity.mesh else {
            if entity.diagnostic {
                result.diagnostics.push(world);
                result.stats.diagnostic_entities += 1;
            }
            continue;
        };
        let source = Arc::clone(&binding.source);
        let RuntimeAssetData::Model(model) = source.data() else {
            return Err(SceneError::MissingModel(binding.source.info().id.clone()));
        };
        if model.animation.is_some() {
            let animation = crate::models::animation::resolve(
                entity_index,
                entity.world,
                binding,
                &mut result.stats,
                &mut animation_bytes,
            )?;
            let key = (source.info().fingerprint.clone(), source.version_id());
            result.models.entry(key).or_insert_with(|| ModelPlan {
                source: Arc::clone(&source),
                primitives: BTreeMap::new(),
                placements: BTreeMap::new(),
            });
            result.animations.push(animation);
            continue;
        }
        if binding.animation.is_some()
            || binding.animation_blend.is_some()
            || binding.animation_pose.is_some()
            || binding.animation_ik.is_some()
            || binding.animation_root.is_some()
        {
            return Err(SceneError::Input(
                "animation sample requires animated model",
            ));
        }
        let scene = model.metadata.default_scene.unwrap_or(0);
        let roots = model
            .metadata
            .scenes
            .get(scene)
            .ok_or_else(|| SceneError::NoScene(binding.source.info().id.clone()))?;
        let mut pending: Vec<_> = roots
            .iter()
            .rev()
            .map(|&index| (index, glam::DMat4::IDENTITY))
            .collect();
        let mut visited = vec![false; model.metadata.nodes.len()];
        let key = (source.info().fingerprint.clone(), source.version_id());
        let plan = result.models.entry(key).or_insert_with(|| ModelPlan {
            source: Arc::clone(&source),
            primitives: BTreeMap::new(),
            placements: BTreeMap::new(),
        });
        result.stats.model_entities += 1;
        while let Some((index, parent)) = pending.pop() {
            if std::mem::replace(&mut visited[index], true) {
                return Err(SceneError::RepeatedNode(binding.source.info().id.clone()));
            }
            let node = &model.metadata.nodes[index];
            let world = parent
                * glam::DMat4::from_cols_array_2d(
                    &node.transform.map(|column| column.map(f64::from)),
                );
            let instance = Instance::new((world64 * world).as_mat4())?;
            for &primitive in &node.meshes {
                result.stats.primitive_instances += 1;
                if result.stats.primitive_instances > MAX_INSTANCES {
                    return Err(SceneError::InstanceLimit);
                }
                result.stats.model_triangles += (model.meshes[primitive].indices.len() / 3) as u64;
                plan.primitives
                    .entry((primitive, binding.cast_shadows))
                    .or_default()
                    .push(instance);
                plan.placements
                    .entry((primitive, binding.cast_shadows))
                    .or_default()
                    .push(Placement {
                        node: entity_index,
                        local: world,
                    });
            }
            pending.extend(node.children.iter().rev().map(|&child| (child, world)));
        }
    }
    result.stats.model_draw_calls = result.models.values().map(|m| m.primitives.len()).sum();
    Ok(result)
}

#[cfg(test)]
#[path = "typed_scene_tests.rs"]
mod typed_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support as support;
    fn resolve(
        project: &incant_doc::Project,
        assets: Option<&incant_assets::AssetStore>,
    ) -> Result<ResolvedScene, SceneError> {
        let input = incant_render_authoring::project_input(project, assets).map_err(|error| {
            *error
                .downcast::<SceneError>()
                .expect("typed scene binding error")
        })?;
        super::resolve(&input)
    }

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
        let instances = &model.primitives[&(0, true)];
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
    #[test]
    fn camera_rig_frames_hide_diagnostic_cubes_but_preserve_explicit_meshes() {
        use incant_doc::{Entity, Transform};
        use serde_json::json;
        let temp = tempfile::tempdir().unwrap();
        let (mut project, assets, _) = support::fixture(temp.path());
        let entities = &mut project.scenes.values_mut().next().unwrap().entities;
        let model_id = entities.keys().next().unwrap().clone();
        let mut frame = Entity::new("Camera rig frame");
        frame
            .components
            .insert("Transform".into(), json!(Transform::default()));
        entities.get_mut(&model_id).unwrap().parent = Some(frame.id.clone());
        entities.insert(frame.id.clone(), frame);
        let mut camera = Entity::new("Attached camera");
        camera.parent = Some(model_id);
        camera.components.insert(
            "Camera".into(),
            json!({"fov_degrees":60,"near":0.1,"far":100}),
        );
        let camera_id = camera.id.clone();
        entities.insert(camera.id.clone(), camera);
        let mut diagnostic = Entity::new("Unrelated diagnostic cube");
        diagnostic
            .components
            .insert("Transform".into(), json!(Transform::default()));
        entities.insert(diagnostic.id.clone(), diagnostic);
        let result = resolve(&project, Some(&assets)).unwrap();
        assert_eq!(result.stats.model_entities, 2);
        assert_eq!(result.stats.diagnostic_entities, 1);
        assert!(result.cameras.contains_key(&camera_id));
    }
    #[test]
    fn shadow_caster_flags_split_instances_without_dropping_geometry() {
        let directory = tempfile::tempdir().unwrap();
        let (mut project, assets, _) = support::fixture(directory.path());
        let first = project
            .scenes
            .values_mut()
            .next()
            .unwrap()
            .entities
            .values_mut()
            .next()
            .unwrap();
        first.components.get_mut("MeshRenderer").unwrap()["cast_shadows"] =
            serde_json::json!(false);
        let resolved = resolve(&project, Some(&assets)).unwrap();
        assert_eq!(resolved.stats.primitive_instances, 2);
        assert_eq!(resolved.stats.model_draw_calls, 2);
        let primitive = &resolved.models.values().next().unwrap().primitives;
        assert_eq!(primitive[&(0, true)].len(), 1);
        assert_eq!(primitive[&(0, false)].len(), 1);
        assert_ne!(
            primitive[&(0, true)][0].world,
            primitive[&(0, false)][0].world
        );
    }
}
