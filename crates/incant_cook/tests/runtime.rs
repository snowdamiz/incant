use incant_cook::{CookError, cook_scene, stable_id};
use incant_doc::{Entity, Project, Scene, Transform, Velocity, new_id};
use incant_runtime::NativeWorld;
use serde_json::json;
use std::collections::BTreeMap;

fn fixture() -> (Project, String, String) {
    let mut project = Project::empty("Cooked motion");
    let scene_id = new_id();
    let entity_id = new_id();
    let entity = Entity {
        id: entity_id.clone(),
        name: "Mover".into(),
        parent: None,
        provenance: None,
        components: BTreeMap::from([
            (
                "Transform".into(),
                serde_json::to_value(Transform::default()).unwrap(),
            ),
            (
                "Velocity".into(),
                serde_json::to_value(Velocity {
                    linear: [3., 0., 0.],
                })
                .unwrap(),
            ),
        ]),
    };
    project.scenes.insert(
        scene_id.clone(),
        Scene {
            id: scene_id.clone(),
            name: "Main".into(),
            entities: BTreeMap::from([(entity_id.clone(), entity)]),
        },
    );
    (project, scene_id, entity_id)
}

#[test]
fn cooked_world_runs_after_authoring_is_discarded_and_cook_is_read_only() {
    let (project, scene, entity) = fixture();
    let before = serde_json::to_vec(&project).unwrap();
    let bytes = cook_scene(&project, &scene).unwrap();
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    assert_eq!(bytes, cook_scene(&project, &scene).unwrap());
    drop(project);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    assert!(
        (world
            .inspect(stable_id(&entity).unwrap())
            .unwrap()
            .transform
            .translation[0]
            - 3.)
            .abs()
            < 1e-12
    );
}

#[test]
fn unsupported_component_is_a_named_error_not_silently_omitted() {
    let (mut project, scene, entity) = fixture();
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components
        .insert(
            "Camera".into(),
            json!({"fov_degrees":60.,"near":0.1,"far":100.}),
        );
    let error = cook_scene(&project, &scene).unwrap_err();
    assert!(
        matches!(error, CookError::UnsupportedComponent { entity: id, component } if id == entity && component == "Camera")
    );
}

#[test]
fn invalid_authoring_and_missing_scene_do_not_produce_bytes() {
    let (mut project, scene, _) = fixture();
    assert!(matches!(
        cook_scene(&project, &new_id()),
        Err(CookError::MissingScene(_))
    ));
    project.settings.tick_rate = 0;
    assert!(matches!(
        cook_scene(&project, &scene),
        Err(CookError::Document(_))
    ));
    assert!(stable_id("not-a-ulid").is_err());
}
