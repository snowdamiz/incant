//! Parent-first runtime projection. The document validates scene membership and
//! cycles before this module runs. Iteration avoids recursion on deep hierarchies.
use crate::Position;
use bevy_ecs::prelude::*;
use glam::{DMat4, DQuat, DVec3};
use incant_doc::{Entity as DocEntity, MeshRenderer, Project, Scene, Transform, Velocity};
use std::collections::{BTreeMap, VecDeque};

#[derive(Component)]
pub(crate) struct ParentId(pub Option<String>);
#[derive(Component)]
pub(crate) struct LocalFrame {
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}
#[derive(Component)]
pub(crate) struct WorldFrame(pub [[f64; 4]; 4]);
#[derive(Resource, Default)]
pub(crate) struct SceneOrder {
    pub entities: Vec<(Entity, Option<usize>)>,
    pub worlds: Vec<DMat4>,
}

pub(crate) struct PreparedEntity {
    pub id: String,
    pub scene: String,
    pub parent_id: Option<String>,
    pub parent_index: Option<usize>,
    pub local: Transform,
    pub velocity: [f64; 3],
    pub world: DMat4,
    pub mesh: Option<MeshRenderer>,
}

/// All validation and decoding precede changes to the running ECS world.
pub(crate) fn prepare(project: &Project) -> Result<Vec<PreparedEntity>, incant_doc::DocumentError> {
    project.validate()?;
    let mut staged: Vec<PreparedEntity> = Vec::new();
    for (scene, entity, parent_index) in ordered_entities(project) {
        let local = entity
            .components
            .get("Transform")
            .map(|value| serde_json::from_value::<Transform>(value.clone()))
            .transpose()?
            .unwrap_or_default();
        let velocity = entity
            .components
            .get("Velocity")
            .map(|value| serde_json::from_value::<Velocity>(value.clone()))
            .transpose()?
            .map_or([0.; 3], |value| value.linear);
        let mesh = entity
            .components
            .get("MeshRenderer")
            .map(|value| serde_json::from_value::<MeshRenderer>(value.clone()))
            .transpose()?;
        let local_matrix = matrix(&local);
        let world = parent_index.map_or(local_matrix, |parent| staged[parent].world * local_matrix);
        if !world.is_finite() {
            return Err(incant_doc::DocumentError::Validation(vec![
                incant_doc::Diagnostic {
                    path: format!(
                        "/scenes/{}/entities/{}/components/Transform",
                        scene.id, entity.id
                    ),
                    message: "composed world transform exceeds runtime numeric range".into(),
                },
            ]));
        }
        staged.push(PreparedEntity {
            id: entity.id.clone(),
            scene: scene.id.clone(),
            parent_id: entity.parent.clone(),
            parent_index,
            local,
            velocity,
            world,
            mesh,
        });
    }
    Ok(staged)
}

pub(crate) fn matrix(transform: &Transform) -> DMat4 {
    DMat4::from_scale_rotation_translation(
        DVec3::from_array(transform.scale),
        DQuat::from_array(transform.rotation).normalize(),
        DVec3::from_array(transform.translation),
    )
}

pub(crate) fn ordered_entities(project: &Project) -> Vec<(&Scene, &DocEntity, Option<usize>)> {
    let mut children = BTreeMap::<&str, Vec<(&Scene, &DocEntity)>>::new();
    let mut pending = VecDeque::new();
    for scene in project.scenes.values() {
        for entity in scene.entities.values() {
            if let Some(parent) = &entity.parent {
                children.entry(parent).or_default().push((scene, entity));
            } else {
                pending.push_back((scene, entity, None));
            }
        }
    }
    let mut ordered = Vec::new();
    while let Some((scene, entity, parent)) = pending.pop_front() {
        let index = ordered.len();
        ordered.push((scene, entity, parent));
        if let Some(children) = children.remove(entity.id.as_str()) {
            pending.extend(
                children
                    .into_iter()
                    .map(|(scene, child)| (scene, child, Some(index))),
            );
        }
    }
    ordered
}

pub(crate) fn propagate(
    mut order: ResMut<SceneOrder>,
    mut query: Query<(&Position, &LocalFrame, &mut WorldFrame)>,
) {
    for index in 0..order.entities.len() {
        let (entity, parent) = order.entities[index];
        // Runtime entity topology changes only through Engine::sync, which
        // rebuilds this order atomically with its ECS world.
        let (position, frame, mut world) =
            query.get_mut(entity).expect("runtime topology invariant");
        let local = matrix(&Transform {
            translation: position.0,
            rotation: frame.rotation,
            scale: frame.scale,
        });
        let global = parent.map_or(local, |parent| order.worlds[parent] * local);
        order.worlds[index] = global;
        world.0 = global.to_cols_array_2d();
    }
}
