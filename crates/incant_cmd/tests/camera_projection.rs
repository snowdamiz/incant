use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Camera, CameraProjection, Entity, Project, Scene};
use serde_json::json;

#[test]
fn projection_changes_share_atomic_history_and_legacy_cameras_keep_perspective() {
    let legacy = json!({"fov_degrees":60., "near":0.1, "far":100.});
    let camera: Camera = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(camera.projection, CameraProjection::Perspective {});
    let mut project = Project::empty("Projection history");
    let mut scene = Scene::new("Scene");
    let mut entity = Entity::new("Camera");
    entity.components.insert("Camera".into(), legacy.clone());
    let ids = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let authored = project.canonical_text().unwrap();
    let mut bus = CommandBus::new(project).unwrap();
    let set = |projection| Command::SetComponent {
        scene_id: ids.0.clone(),
        entity_id: ids.1.clone(),
        component: "Camera".into(),
        value: json!({"fov_degrees":60., "near":0.1, "far":100., "projection":projection}),
    };
    for bad in [
        json!({"kind":"orthographic","vertical_size":0.}),
        json!({"kind":"orthographic","vertical_size":-3.}),
        json!({"kind":"orthographic"}),
        json!({"kind":"unknown"}),
        json!({"kind":"perspective","vertical_size":5.}),
    ] {
        assert!(
            bus.execute(
                vec![
                    Command::RenameEntity {
                        scene_id: ids.0.clone(),
                        entity_id: ids.1.clone(),
                        name: "partial".into()
                    },
                    set(bad)
                ],
                Actor::agent("camera-test", "test"),
                "invalid projection",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), authored);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set(json!({"kind":"orthographic","vertical_size":8.}))],
        Actor::agent("camera-test", "test"),
        "parallel camera",
        None,
    )
    .unwrap();
    let changed = bus.project().canonical_text().unwrap();
    let reloaded = Project::from_text(&changed).unwrap();
    let value = &reloaded.scenes[&ids.0].entities[&ids.1];
    assert_eq!(
        value.provenance.as_ref().unwrap().model.as_deref(),
        Some("camera-test")
    );
    assert_eq!(
        serde_json::from_value::<Camera>(value.components["Camera"].clone())
            .unwrap()
            .projection,
        CameraProjection::Orthographic { vertical_size: 8. }
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), changed);
}
