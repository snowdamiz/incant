//! Checked render declarations, independent of GPU resources and authoring.
use crate::{SceneError, StableId};
use bevy_ecs::prelude::Component;
use incant_types::{Camera, CameraProjection, DirectionalLight, PointLight, SpotLight};
use std::collections::BTreeSet;

pub use incant_types::{VisualAssetKind, VisualAssetRef};
pub const MAX_MESH_ENTITIES: usize = 100_000;
pub const MAX_VISUAL_ASSETS: usize = MAX_MESH_ENTITIES + crate::MAX_SPRITE_ENTITIES + 1;
pub const MAX_DIRECTIONAL_LIGHTS: usize = 16;
pub const MAX_LOCAL_LIGHTS: usize = 4096;
pub const MAX_SHADOW_LIGHTS: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct NativeMeshRenderer {
    pub mesh: StableId,
    pub cast_shadows: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeEnvironmentLight {
    pub texture: StableId,
    pub intensity: f64,
    pub rotation_degrees: f64,
}

/// One entity's optional render configuration. Material overrides are not a
/// migrated capability; the authoring cook rejects them explicitly.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct CookedSpatial {
    pub entity: StableId,
    pub camera: Option<Camera>,
    pub mesh: Option<NativeMeshRenderer>,
    pub animation: Option<std::sync::Arc<crate::NativeAnimation>>,
    /// Cold binding data stays indirect so every unrelated structural command
    /// and spatial node does not pay for a full sprite configuration.
    pub sprite: Option<Box<crate::NativeSpriteRenderer>>,
    pub tilemap: Option<Box<crate::NativeTilemap>>,
    pub flipbook: Option<std::sync::Arc<crate::NativeFlipbook>>,
    pub directional: Option<DirectionalLight>,
    pub point: Option<PointLight>,
    pub spot: Option<SpotLight>,
    pub environment: Option<NativeEnvironmentLight>,
}
impl CookedSpatial {
    pub fn empty(entity: StableId) -> Self {
        Self {
            entity,
            camera: None,
            mesh: None,
            animation: None,
            sprite: None,
            tilemap: None,
            flipbook: None,
            directional: None,
            point: None,
            spot: None,
            environment: None,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.camera.is_none()
            && self.mesh.is_none()
            && self.animation.is_none()
            && self.sprite.is_none()
            && self.tilemap.is_none()
            && self.flipbook.is_none()
            && self.directional.is_none()
            && self.point.is_none()
            && self.spot.is_none()
            && self.environment.is_none()
    }
}

pub(crate) fn validate_assets(
    assets: &[VisualAssetRef],
    scene: StableId,
    exists: impl Fn(StableId) -> bool,
) -> Result<(), SceneError> {
    if assets.len() > MAX_VISUAL_ASSETS {
        return Err(SceneError::Invalid("visual asset count"));
    }
    let mut ids = BTreeSet::new();
    for asset in assets {
        if asset.id == scene || exists(asset.id) || !ids.insert(asset.id) {
            return Err(SceneError::Invalid(
                "duplicate or colliding visual asset ID",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate<'a>(
    nodes: impl IntoIterator<Item = &'a CookedSpatial>,
    assets: &[VisualAssetRef],
    exists: impl Fn(StableId) -> bool,
) -> Result<(), SceneError> {
    let invalid = || SceneError::Invalid("spatial declarations, limits or references");
    let mut ids = BTreeSet::new();
    let (mut tilemaps, mut tile_slots) = (0, 0usize);
    let mut flipbook_frames = 0usize;
    let mut animation_clips = 0usize;
    let mut graphs = 0usize;
    let (mut meshes, mut sprites, mut directional, mut local, mut shadows, mut environments) =
        (0, 0, 0, 0, 0, 0);
    let asset = |id, kind| {
        assets
            .binary_search_by_key(&id, |a| a.id)
            .is_ok_and(|i| assets[i].kind == kind)
    };
    for node in nodes {
        if !exists(node.entity)
            || !ids.insert(node.entity)
            || node.is_empty()
            || usize::from(node.mesh.is_some())
                + usize::from(node.sprite.is_some())
                + usize::from(node.tilemap.is_some())
                > 1
            || usize::from(node.directional.is_some())
                + usize::from(node.point.is_some())
                + usize::from(node.spot.is_some())
                > 1
        {
            return Err(invalid());
        }
        if let Some(camera) = &node.camera {
            validate_camera(camera)?;
        }
        if let Some(mesh) = &node.mesh {
            meshes += 1;
            if !asset(mesh.mesh, VisualAssetKind::Model) {
                return Err(invalid());
            }
        }
        if let Some(animation) = &node.animation {
            animation.validate()?;
            let mesh = node.mesh.as_ref().ok_or_else(invalid)?;
            let reference = assets
                .binary_search_by_key(&mesh.mesh, |a| a.id)
                .map_err(|_| invalid())?;
            if assets[reference].fingerprint != animation.mesh_fingerprint {
                return Err(invalid());
            }
            graphs += usize::from(animation.graph.is_some());
            if graphs > crate::MAX_ANIMATION_GRAPH_INSTANCES {
                return Err(invalid());
            }
            animation_clips = animation_clips
                .checked_add(animation.clip_durations.len())
                .ok_or_else(invalid)?;
            if animation_clips > crate::MAX_SCENE_ANIMATION_CLIPS {
                return Err(invalid());
            }
        }
        if let Some(sprite) = &node.sprite {
            sprites += 1;
            sprite.validate()?;
            if !asset(sprite.texture, VisualAssetKind::Texture) {
                return Err(invalid());
            }
        }
        if let Some(flipbook) = &node.flipbook {
            flipbook.validate(node.sprite.as_deref().ok_or_else(invalid)?)?;
            flipbook_frames = flipbook_frames
                .checked_add(flipbook.frames.len())
                .ok_or_else(invalid)?;
            if flipbook_frames > crate::MAX_SCENE_FLIPBOOK_FRAMES {
                return Err(invalid());
            }
        }
        if let Some(map) = &node.tilemap {
            map.validate()
                .map_err(|_| SceneError::Invalid("tilemap values"))?;
            tilemaps += 1;
            tile_slots = tile_slots
                .checked_add(map.cell_slots())
                .ok_or_else(invalid)?;
            if tilemaps > crate::MAX_TILEMAP_ENTITIES
                || tile_slots > crate::MAX_SCENE_TILE_SLOTS
                || !asset(map.tileset, VisualAssetKind::Texture)
            {
                return Err(invalid());
            }
        }
        if let Some(light) = &node.directional {
            directional += 1;
            light_values(light.color, light.intensity, None)?;
            if let Some(shadow) = &light.shadows {
                shadows += 1;
                if !(0.01..=10000.).contains(&shadow.distance) {
                    return Err(invalid());
                }
            }
        }
        if let Some(light) = &node.point {
            local += 1;
            light_values(light.color, light.intensity, Some(light.range))?;
        }
        if let Some(light) = &node.spot {
            local += 1;
            light_values(light.color, light.intensity, Some(light.range))?;
            if !(0.0..=89.9).contains(&light.inner_degrees)
                || !(0.1..=89.9).contains(&light.outer_degrees)
                || light.inner_degrees >= light.outer_degrees
            {
                return Err(invalid());
            }
        }
        if let Some(environment) = &node.environment {
            environments += 1;
            if !asset(environment.texture, VisualAssetKind::Texture)
                || !(0.0..=100.).contains(&environment.intensity)
                || !(-360.0..=360.).contains(&environment.rotation_degrees)
            {
                return Err(invalid());
            }
        }
        if sprites > crate::MAX_SPRITE_ENTITIES
            || meshes > MAX_MESH_ENTITIES
            || directional > MAX_DIRECTIONAL_LIGHTS
            || local > MAX_LOCAL_LIGHTS
            || shadows > MAX_SHADOW_LIGHTS
            || environments > 1
        {
            return Err(invalid());
        }
    }
    Ok(())
}
fn light_values(color: [f64; 3], intensity: f64, range: Option<f64>) -> Result<(), SceneError> {
    if !color.iter().all(|v| (0.0..=1.0).contains(v))
        || !(0.0..=1_000_000.).contains(&intensity)
        || range.is_some_and(|v| !(0.001..=10000.).contains(&v))
    {
        return Err(SceneError::Invalid("light numeric range"));
    }
    Ok(())
}
fn validate_camera(c: &Camera) -> Result<(), SceneError> {
    let invalid = || SceneError::Invalid("camera projection");
    if !(0.1..179.).contains(&c.fov_degrees)
        || !c.near.is_finite()
        || c.near <= 0.
        || !c.far.is_finite()
        || c.far <= c.near
    {
        return Err(invalid());
    }
    match c.projection {
        CameraProjection::Perspective {} => (),
        CameraProjection::Orthographic { vertical_size } => {
            if !vertical_size.is_finite() || vertical_size <= 0. {
                return Err(invalid());
            }
        }
        CameraProjection::PixelPerfect {
            pixels_per_unit,
            reference_resolution: [w, h],
        } => {
            if !(1.0..=4096.).contains(&pixels_per_unit)
                || !(16..=4096).contains(&w)
                || !(16..=4096).contains(&h)
                || u64::from(w) * u64::from(h) > 4 * 1024 * 1024
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
