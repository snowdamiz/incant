use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Collider, Entity, Project, RigidBody, Scene, Transform};
use serde_json::json;
#[test]
fn physics_components_are_atomic_reversible_and_validate_cross_component_constraints() {
    let mut project = Project::empty("Physics authoring");
    let mut scene = Scene::new("World");
    let entity = Entity::new("Body");
    let (sid, eid) = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let before = bus.project().clone();
    let set = |component: &str, value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: component.into(),
        value,
    };
    let actor = Actor::agent("physics-test", "session-test");
    assert!(
        bus.execute(
            vec![set("RigidBody", json!(RigidBody::default()))],
            actor.clone(),
            "Invalid incomplete body",
            None
        )
        .is_err()
    );
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), 0);
    bus.execute(
        vec![
            set("RigidBody", json!(RigidBody::default())),
            set("Collider", json!(Collider::default())),
            set("Transform", json!(Transform::default())),
        ],
        actor.clone(),
        "Create complete body",
        None,
    )
    .unwrap();
    let body = bus.project().clone();
    assert_eq!(
        body.scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("physics-test")
    );
    for command in [
        set(
            "Transform",
            json!(Transform {
                scale: [2.; 3],
                ..Default::default()
            }),
        ),
        set(
            "Collider",
            json!({"shape":{"type":"mesh"},"density":1000,"friction":0.5,"restitution":0,"sensor":false,"memberships":1,"filter":1}),
        ),
        Command::RemoveComponent {
            scene_id: sid.clone(),
            entity_id: eid.clone(),
            component: "Collider".into(),
        },
    ] {
        assert!(
            bus.execute(
                vec![command],
                actor.clone(),
                "Reject unsupported physics",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project(), &body);
        assert_eq!(bus.revision(), 1);
    }
    bus.undo().unwrap();
    assert_eq!(bus.project().scenes, before.scenes);
    bus.redo().unwrap();
    assert_eq!(bus.project().scenes, body.scenes);
}
