//! Authoring-side scene cooking. The runtime depends on neither this crate nor
//! incant_doc. Unsupported components fail explicitly until their migration.
use incant_doc::{Project, Transform, Velocity};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedPhysics, CookedScene, NativeAudioBus, NativeAudioSource,
    SceneError, StableId,
};
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
    #[error("material overrides on {0} have not migrated to the native renderer")]
    MaterialOverrides(String),
    #[error("invalid visual asset binding {0}")]
    VisualAsset(String),
    #[error("invalid font asset binding {0}")]
    FontAsset(String),
    #[error("missing or stale cooked navigation geometry for model {0}")]
    NavigationResource(String),
    #[error("missing, invalid or stale cooked animation resource for model {0}")]
    AnimationResource(String),
    #[error("invalid stable ID {0}")]
    InvalidId(String),
    #[error("component decoding failed: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Validate authoring once, then convert a selected scene to a checked, canonical
/// binary image. Missing Transform projects to identity, as in the existing ECS.
pub fn cook_scene(project: &Project, scene_id: &str) -> Result<Vec<u8>, CookError> {
    let validated = project.validated()?;
    cook_validated_scene(validated, scene_id, &std::collections::BTreeSet::new())
}

pub(crate) fn cook_validated_scene(
    validated: incant_doc::ValidatedProject<'_>,
    scene_id: &str,
    attached_scripts: &std::collections::BTreeSet<String>,
) -> Result<Vec<u8>, CookError> {
    cook_validated_scene_with_navigation(validated, scene_id, attached_scripts, &Default::default())
}

/// Explicit authoring cook with checked cooked-model navigation resources.
/// Their recipe fingerprints must match the same validated project snapshot.
pub fn cook_scene_with_navigation(
    project: &Project,
    scene_id: &str,
    resources: &incant_nav::NavigationResources,
) -> Result<Vec<u8>, CookError> {
    cook_validated_scene_with_navigation(
        project.validated()?,
        scene_id,
        &Default::default(),
        resources,
    )
}
pub(crate) fn cook_validated_scene_with_navigation(
    validated: incant_doc::ValidatedProject<'_>,
    scene_id: &str,
    attached_scripts: &std::collections::BTreeSet<String>,
    resources: &incant_nav::NavigationResources,
) -> Result<Vec<u8>, CookError> {
    cook_validated_scene_with_resources(
        validated,
        scene_id,
        attached_scripts,
        resources,
        &Default::default(),
    )
}

/// Cook one validated project snapshot with sealed startup animation metadata
/// and checked navigation resources. Legacy entry points refuse missing data.
pub fn cook_scene_with_resources(
    project: &Project,
    scene_id: &str,
    navigation: &incant_nav::NavigationResources,
    animations: &crate::AnimationResources,
) -> Result<Vec<u8>, CookError> {
    cook_validated_scene_with_resources(
        project.validated()?,
        scene_id,
        &Default::default(),
        navigation,
        animations,
    )
}
pub(crate) fn cook_validated_scene_with_resources(
    validated: incant_doc::ValidatedProject<'_>,
    scene_id: &str,
    attached_scripts: &std::collections::BTreeSet<String>,
    resources: &incant_nav::NavigationResources,
    animations: &crate::AnimationResources,
) -> Result<Vec<u8>, CookError> {
    let project = validated.project();
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| CookError::MissingScene(scene_id.into()))?;
    let mut texts = Vec::new();
    let mut ui = Vec::new();
    let mut font_assets = std::collections::BTreeMap::new();
    let mut entities = Vec::with_capacity(scene.entities.len());
    let mut physics = Vec::new();
    let mut physics_2d = Vec::new();
    let mut audio = Vec::new();
    let mut spatial = Vec::new();
    let mut navigation = Vec::new();
    let mut visual_assets = std::collections::BTreeMap::new();
    for entity in scene.entities.values() {
        for component in entity.components.keys() {
            // Only the program-aware cook constructs this exact attachment set
            // from the same validated project and returns it with compiled code.
            if component == "Script" && attached_scripts.contains(&entity.id) {
                continue;
            }
            if ![
                "Text",
                "UiNode",
                "NavigationMesh",
                "NavigationGrid",
                "Transform",
                "Velocity",
                "RigidBody",
                "Collider",
                "AngularVelocity",
                "RigidBody2D",
                "Collider2D",
                "AngularVelocity2D",
                "AudioBus",
                "AudioSource",
                "AudioListener",
                "Camera",
                "MeshRenderer",
                "Animation",
                "SpriteRenderer",
                "SpriteFlipbook",
                "Tilemap",
                "DirectionalLight",
                "PointLight",
                "SpotLight",
                "EnvironmentLight",
            ]
            .contains(&component.as_str())
            {
                return Err(CookError::UnsupportedComponent {
                    entity: entity.id.clone(),
                    component: component.clone(),
                });
            }
        }
        if let Some(node) = crate::ui::cook(project, entity, &mut font_assets)? {
            ui.push(node);
        }
        if let Some(node) = crate::text::cook(project, entity, &mut font_assets)? {
            texts.push(node);
        }
        if let Some(node) = crate::navigation::cook(project, scene, entity, resources)? {
            navigation.push(node);
        }
        if let Some(node) = crate::spatial::cook(project, entity, &mut visual_assets, animations)? {
            spatial.push(node);
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
        if let Some(collider) = entity.components.get("Collider") {
            physics.push(CookedPhysics {
                entity: stable_id(&entity.id)?,
                collider: serde_json::from_value(collider.clone())?,
                body: entity
                    .components
                    .get("RigidBody")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?,
                angular_velocity: entity
                    .components
                    .get("AngularVelocity")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?,
            });
        }
        if let Some(collider) = entity.components.get("Collider2D") {
            physics_2d.push(incant_runtime::CookedPhysics2D {
                entity: stable_id(&entity.id)?,
                collider: serde_json::from_value(collider.clone())?,
                body: entity
                    .components
                    .get("RigidBody2D")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?,
                angular_velocity: entity
                    .components
                    .get("AngularVelocity2D")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?,
            });
        }
        let bus = entity
            .components
            .get("AudioBus")
            .map(|v| serde_json::from_value::<incant_doc::AudioBus>(v.clone()))
            .transpose()?;
        let source = entity
            .components
            .get("AudioSource")
            .map(|v| serde_json::from_value::<incant_doc::AudioSource>(v.clone()))
            .transpose()?;
        let listener = entity.components.contains_key("AudioListener");
        if bus.is_some() || source.is_some() || listener {
            let bus = bus
                .map(|b| {
                    Ok::<_, CookError>(NativeAudioBus {
                        parent: b.parent.as_deref().map(stable_id).transpose()?,
                        gain_db: b.gain_db,
                    })
                })
                .transpose()?;
            let source = source
                .map(|s| {
                    Ok::<_, CookError>(NativeAudioSource {
                        clip: stable_id(&s.clip)?,
                        bus: s.bus.as_deref().map(stable_id).transpose()?,
                        gain_db: s.gain_db,
                        pan: s.pan,
                        rate: s.rate,
                        start_seconds: s.start_seconds,
                        playing: s.playing,
                        looping: s.looping,
                        streaming: s.streaming,
                        spatial: s.spatial,
                    })
                })
                .transpose()?;
            audio.push(CookedAudio {
                entity: stable_id(&entity.id)?,
                bus,
                source,
                listener,
            });
        }
        entities.push(CookedEntity {
            id: stable_id(&entity.id)?,
            parent: entity.parent.as_deref().map(stable_id).transpose()?,
            transform,
            velocity,
        });
    }
    Ok(
        CookedScene::new(stable_id(&scene.id)?, project.settings.tick_rate, entities)?
            .with_physics(physics)?
            .with_audio(audio)?
            .with_spatial(visual_assets.into_values().collect(), spatial)?
            .with_physics_2d(physics_2d)?
            .with_navigation(navigation)?
            .with_localization(incant_runtime::NativeLocalization::new(
                project.string_tables.clone(),
                project.settings.localization.clone(),
            )?)?
            .with_text(font_assets.into_values().collect(), texts)?
            .with_ui(ui)?
            .to_bytes(),
    )
}

pub fn stable_id(id: &str) -> Result<StableId, CookError> {
    id.parse::<ulid::Ulid>()
        .map(|id| StableId(id.to_bytes()))
        .map_err(|_| CookError::InvalidId(id.into()))
}
