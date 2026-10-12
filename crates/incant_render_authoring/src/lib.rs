//! Authoring-only projection and cache-grant checks for the canonical renderer.
//! All project/JSON interpretation stays here; simulation never calls this adapter.
use incant_assets::{AssetStore, RuntimeAsset, RuntimeAssetData};
use incant_doc::Project;
use incant_render_data::*;
use std::{collections::BTreeSet, sync::Arc};

fn asset(
    project: &Project,
    assets: Option<&AssetStore>,
    id: &str,
    kind: &str,
) -> Option<Arc<RuntimeAsset>> {
    let registered = project.assets.get(id)?;
    assets?
        .get(id)
        .filter(|asset| asset.info().fingerprint == registered.sha256 && asset.info().kind == kind)
}
/// Validate and resolve authored state once after a project revision. Returned
/// bindings retain immutable cooked versions through later imports/removals.
pub fn project_input(project: &Project, assets: Option<&AssetStore>) -> Result<SceneInput> {
    let entities = incant_core::project_entities(project)?;
    let mut result = SceneInput::default();
    let mut render_frames = BTreeSet::new();
    for scene in project.scenes.values() {
        for entity in scene.entities.values() {
            if let Some(value) = entity.components.get("EnvironmentLight") {
                let binding: incant_doc::EnvironmentLight = serde_json::from_value(value.clone())?;
                let source = asset(project, assets, &binding.texture, "texture")
                    .ok_or_else(|| SceneError::MissingEnvironment(binding.texture.clone()))?;
                result.environment = Some(EnvironmentInput {
                    source,
                    intensity: binding.intensity,
                    rotation_degrees: binding.rotation_degrees,
                });
            }
            if entity.components.contains_key("Camera")
                || entity.components.contains_key("SpriteRenderer")
                || entity.components.contains_key("Tilemap")
            {
                let mut current = Some(entity.id.as_str());
                while let Some(id) = current {
                    if !render_frames.insert(id.to_owned()) {
                        break;
                    }
                    current = scene.entities[id].parent.as_deref();
                }
            }
        }
    }
    for entity in entities.values() {
        let components = &project.scenes[&entity.scene_id].entities[&entity.id].components;
        let camera = components
            .get("Camera")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;
        let light = if let Some(v) = components.get("DirectionalLight") {
            Some(SceneLight::Directional(serde_json::from_value(v.clone())?))
        } else if let Some(v) = components.get("PointLight") {
            Some(SceneLight::Point(serde_json::from_value(v.clone())?))
        } else if let Some(v) = components.get("SpotLight") {
            Some(SceneLight::Spot(serde_json::from_value(v.clone())?))
        } else {
            None
        };
        let sprite = if let Some(value) = components.get("SpriteRenderer") {
            let mut sprite: incant_doc::SpriteRenderer = serde_json::from_value(value.clone())?;
            let source = asset(project, assets, &sprite.texture, "texture")
                .ok_or_else(|| SceneError::MissingSprite(sprite.texture.clone()))?;
            let flipbook = components
                .get("SpriteFlipbook")
                .map(|v| serde_json::from_value::<incant_doc::SpriteFlipbook>(v.clone()))
                .transpose()?;
            if let Some(f) = &flipbook {
                sprite.region = f
                    .frames
                    .first()
                    .ok_or(SceneError::Input("flipbook frame count"))?
                    .clone();
            }
            if let incant_doc::SpriteRegion::Atlas { frame } = &sprite.region {
                let region = match source.data() {
                    RuntimeAssetData::Texture(texture) => texture
                        .metadata
                        .atlas
                        .as_ref()
                        .and_then(|atlas| atlas.frames.get(frame))
                        .map(|frame| frame.region),
                    _ => None,
                }
                .ok_or_else(|| SceneError::MissingSpriteFrame {
                    texture: sprite.texture.clone(),
                    frame: frame.clone(),
                })?;
                sprite.region = region.into();
            }
            let frames = flipbook
                .as_ref()
                .map(|f| {
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Arc::new(
                        SpriteFrames::resolve(&source, &sprite, &f.frames)?,
                    ))
                })
                .transpose()?;
            Some(SpriteInput {
                source,
                sprite,
                frames,
                frame: 0,
            })
        } else {
            None
        };
        let tilemap = components
            .get("Tilemap")
            .map(|value| {
                let map: incant_doc::Tilemap = serde_json::from_value(value.clone())?;
                let source = asset(project, assets, &map.tileset, "texture")
                    .ok_or(SceneError::TilemapBinding)?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Arc::new(
                    TilemapInput::from_string_ids(source, &map)?,
                ))
            })
            .transpose()?;
        // Sprite precedence matches the existing authoring preview.
        let mesh = if sprite.is_some() || tilemap.is_some() {
            None
        } else if let Some(binding) = &entity.mesh {
            if !binding.materials.is_empty() {
                return Err(SceneError::MaterialOverrides.into());
            }
            let source = asset(project, assets, &binding.mesh, "model")
                .ok_or_else(|| SceneError::MissingModel(binding.mesh.clone()))?;
            let animation = components
                .get("Animation")
                .map(|value| {
                    let animation: incant_types::Animation = serde_json::from_value(value.clone())?;
                    animation.validate().map_err(SceneError::Input)?;
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
                        incant_types::AnimationSample {
                            clip: animation.clip,
                            seconds: animation.seconds,
                        },
                    )
                })
                .transpose()?;
            Some(MeshInput {
                source,
                cast_shadows: binding.cast_shadows,
                animation,
                animation_blend: None,
                animation_pose: None,
                animation_ik: None,
                animation_root: None,
            })
        } else {
            None
        };
        if components.contains_key("Animation") && mesh.is_none() {
            return Err(SceneError::Input("Animation requires a MeshRenderer").into());
        }
        let diagnostic = sprite.is_none()
            && tilemap.is_none()
            && mesh.is_none()
            && components.contains_key("Transform")
            && !render_frames.contains(&entity.id)
            && ![
                "Camera",
                "EnvironmentLight",
                "DirectionalLight",
                "PointLight",
                "SpotLight",
            ]
            .iter()
            .any(|kind| components.contains_key(*kind));
        result.nodes.push(SceneNode {
            id: entity.id.clone(),
            world: glam::DMat4::from_cols_array_2d(&entity.world_transform),
            camera,
            light,
            mesh,
            sprite,
            tilemap,
            diagnostic,
        });
    }
    Ok(result)
}
pub trait RendererAuthoringExt: ScenePreparer {
    fn prepare_scene(&self, project: &Project, assets: &AssetStore) -> Result<Self::Prepared> {
        self.prepare_input(&project_input(project, Some(assets))?)
    }
    fn diagnostic_scene(&self, project: &Project) -> Result<Self::Prepared> {
        self.prepare_input(&project_input(project, None)?)
    }
    fn screenshot_png(&self, project: &Project, width: u32, height: u32) -> Result<Vec<u8>> {
        self.capture_prepared(&self.diagnostic_scene(project)?, width, height)
    }
}
impl<T: ScenePreparer> RendererAuthoringExt for T {}
