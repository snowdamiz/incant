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
    let mut physics = Vec::new();
    let mut audio = Vec::new();
    let mut spatial = Vec::new();
    let mut visual_assets = std::collections::BTreeMap::new();
    for entity in scene.entities.values() {
        for component in entity.components.keys() {
            if ![
                "Transform",
                "Velocity",
                "RigidBody",
                "Collider",
                "AngularVelocity",
                "AudioBus",
                "AudioSource",
                "AudioListener",
                "Camera",
                "MeshRenderer",
                "SpriteRenderer",
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
        if let Some(node) = crate::spatial::cook(project, entity, &mut visual_assets)? {
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
            .to_bytes(),
    )
}

pub fn stable_id(id: &str) -> Result<StableId, CookError> {
    id.parse::<ulid::Ulid>()
        .map(|id| StableId(id.to_bytes()))
        .map_err(|_| CookError::InvalidId(id.into()))
}
