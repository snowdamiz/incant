#![cfg(feature = "scene")]
use incant_cook::{CookError, cook_scene};
use incant_doc::{Entity, PhysicsMode, Project, Scene, Transform, Velocity};
use incant_runtime::{NativeWorld, PHYSICS_VARIANT, SceneError};
use serde_json::json;

#[test]
fn only_a_matching_authoring_mode_cooks_and_runs_without_the_document() {
    let mut project = Project::empty("Selected physics");
    project.settings.physics_mode = PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let scene_id = scene.id.clone();
    let mut entity = Entity::new("Mover");
    let entity_id = incant_cook::stable_id(&entity.id).unwrap();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity.components.insert(
        "Velocity".into(),
        json!(Velocity {
            linear: [3., 0., 0.]
        }),
    );
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let bytes = cook_scene(&project, &scene_id).unwrap();
    let opposite = match PHYSICS_VARIANT.mode() {
        PhysicsMode::Fast => PhysicsMode::Deterministic,
        PhysicsMode::Deterministic => PhysicsMode::Fast,
    };
    project.settings.physics_mode = opposite;
    let before = project.canonical_text().unwrap();
    assert!(matches!(cook_scene(&project, &scene_id),
        Err(CookError::Scene(SceneError::PhysicsMode { requested, compiled }))
        if requested == opposite && compiled == PHYSICS_VARIANT));
    assert_eq!(project.canonical_text().unwrap(), before);
    drop(project);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    world.step().unwrap();
    assert_eq!(
        world.inspect(entity_id).unwrap().transform.translation,
        [0.05, 0., 0.]
    );
}
