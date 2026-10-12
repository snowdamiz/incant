//! Shared cold/structural/numeric ownership policy. No authoring validation.
use crate::SceneError;
use incant_types::{
    BodyMotion, Collider, ColliderShape, RigidBody, RootMotionApplication, Transform, Velocity,
};

pub(crate) fn validate_character_catalog(root: &super::NativeRootMotion) -> Result<(), SceneError> {
    if root.application != RootMotionApplication::Character3d {
        return Ok(());
    }
    let reference = root.catalog.model_reference();
    let inverse = reference
        .inverse()
        .map_err(|_| SceneError::Invalid("character root reference"))?;
    for (i, clip) in root.catalog.clips().enumerate() {
        let Some(track) = clip.rotation() else {
            continue;
        };
        let sampler = root.catalog.sampler(i).expect("catalog clip");
        for &time in track.times() {
            let relative = sampler
                .clamped_delta(0., f64::from(time))
                .and_then(|d| reference.compose(d))
                .and_then(|d| d.compose(inverse))
                .map_err(|_| SceneError::Invalid("character root rotation range"))?;
            if !yaw_only(relative.rotation) {
                return Err(SceneError::Invalid(
                    "character root catalog contains pitch or roll",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate(
    policy: RootMotionApplication,
    parent: bool,
    transform: &Transform,
    velocity: Option<&Velocity>,
    body: Option<&RigidBody>,
    collider: Option<&Collider>,
    planar: bool,
) -> Result<(), SceneError> {
    match policy {
        RootMotionApplication::ExtractOnly => Ok(()),
        RootMotionApplication::Transform => {
            if collider.is_some() || planar || velocity.is_some() {
                return Err(SceneError::Invalid(
                    "transform root motion requires bodyless velocity-free owner",
                ));
            }
            if !uniform(transform.scale) {
                return Err(SceneError::Invalid(
                    "transform root motion requires positive uniform scale",
                ));
            }
            Ok(())
        }
        RootMotionApplication::Kinematic3d | RootMotionApplication::Character3d => {
            if parent
                || planar
                || transform.scale != [1.; 3]
                || !body.is_some_and(|b| b.motion == BodyMotion::Kinematic)
                || collider.is_none()
            {
                return Err(SceneError::Invalid(
                    "root motion requires an unparented unit-scale 3D kinematic body",
                ));
            }
            if policy == RootMotionApplication::Character3d {
                let collider = collider.expect("checked collider");
                if collider.sensor
                    || !matches!(
                        collider.shape,
                        ColliderShape::Sphere { .. } | ColliderShape::Capsule { .. }
                    )
                    || !yaw_only(transform.rotation)
                {
                    return Err(SceneError::Invalid(
                        "character root motion requires upright nonsensor sphere or Y capsule",
                    ));
                }
            }
            Ok(())
        }
    }
}
pub(crate) fn uniform(scale: [f64; 3]) -> bool {
    scale[0].is_finite() && scale[0] > 0.0 && scale[1] == scale[0] && scale[2] == scale[0]
}
pub(crate) fn yaw_only(rotation: [f64; 4]) -> bool {
    rotation[0].abs() <= 1e-12 && rotation[2].abs() <= 1e-12
}

pub(crate) fn validate_scene(
    scene: &crate::CookedScene,
    entities: &std::collections::BTreeMap<crate::StableId, &crate::CookedEntity>,
) -> Result<(), SceneError> {
    let mut checked = std::collections::BTreeSet::new();
    for node in &scene.spatial {
        let Some(root) = &node.root_motion else {
            continue;
        };
        if root.application == RootMotionApplication::Character3d
            && checked.insert((root.catalog.fingerprint(), root.catalog.root()))
        {
            validate_character_catalog(root)?;
        }
        if root.application == RootMotionApplication::Character3d
            && scene.physics.len() > incant_physics_runtime::MAX_CHARACTER_QUERY_COLLIDERS
        {
            return Err(SceneError::Invalid("character query collider capacity"));
        }
        let entity = entities[&node.entity];
        let physics = scene
            .physics
            .binary_search_by_key(&node.entity, |p| p.entity)
            .ok()
            .map(|i| &scene.physics[i]);
        let planar = scene
            .physics_2d
            .binary_search_by_key(&node.entity, |p| p.entity)
            .is_ok()
            || node
                .tilemap
                .as_ref()
                .is_some_and(|m| m.layers.iter().any(|l| l.solid));
        validate(
            root.application,
            entity.parent.is_some(),
            &entity.transform,
            entity.velocity.as_ref(),
            physics.and_then(|p| p.body.as_ref()),
            physics.map(|p| &p.collider),
            planar,
        )?;
        if root.application == RootMotionApplication::Transform {
            let mut parent = entity.parent;
            while let Some(id) = parent {
                if !uniform(entities[&id].transform.scale) {
                    return Err(SceneError::Invalid(
                        "root-motion ancestors require positive uniform scale",
                    ));
                }
                parent = entities[&id].parent;
            }
        }
    }
    Ok(())
}
