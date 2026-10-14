#![cfg(feature = "scene")]
use incant_cook::{CookError, cook_scene};
use incant_doc::{EngineRequirement, Entity, Project, Scene, Transform, Velocity};

#[test]
fn engine_pin_checks_precede_cook_and_leave_supported_cooked_runtime_bytes_unchanged() {
    let mut project = Project::empty("Engine compatibility");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    project.engine_version = None;
    let mut scene = Scene::new("World");
    let mut entity = Entity::new("Moving");
    entity.components.insert(
        "Transform".into(),
        serde_json::to_value(Transform::default()).unwrap(),
    );
    entity.components.insert(
        "Velocity".into(),
        serde_json::to_value(Velocity {
            linear: [1., 0., 0.],
        })
        .unwrap(),
    );
    scene.entities.insert(entity.id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    let original = cook_scene(&project, &scene_id).unwrap();
    project.engine_version = Some(EngineRequirement::exact_current());
    assert_eq!(cook_scene(&project, &scene_id).unwrap(), original);
    project.engine_version = Some(EngineRequirement::parse(">=99.0.0").unwrap());
    assert!(matches!(
        cook_scene(&project, &scene_id),
        Err(CookError::Document(_))
    ));
    drop(project);
    let mut world = incant_runtime::NativeWorld::from_bytes(&original).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    assert_eq!(world.entity_count(), 1);
    assert!((world.snapshot()[0].transform.translation[0] - 1.).abs() < 1e-12);
}
