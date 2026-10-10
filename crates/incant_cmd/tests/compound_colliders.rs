use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{
    Collider, ColliderPart, ColliderShape, Entity, PrimitiveColliderShape, Project, Scene,
    Transform, new_id,
};
use serde_json::json;

#[test]
fn compound_part_edits_validate_atomically_and_survive_undo_redo_and_reopen() {
    let mut project = Project::empty("Compound authoring");
    let mut scene = Scene::new("Scene");
    let mut entity = Entity::new("Arch");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Collider".into(), json!(Collider::default()));
    let (sid, eid) = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    let part = ColliderPart {
        id: new_id(),
        translation: [1., 0., 0.],
        rotation: [0., 0., 0., 1.],
        shape: PrimitiveColliderShape::Sphere { radius: 0.5 },
    };
    let mut collider = json!(Collider {
        shape: ColliderShape::Compound {
            parts: vec![part.clone()]
        },
        ..Default::default()
    });
    let path = std::env::temp_dir().join(format!("incant-compound-{}.jsonl", new_id()));
    let set = |value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: "Collider".into(),
        value,
    };
    let mut bus = CommandBus::persistent(&path, project.clone()).unwrap();
    let actor = Actor::agent("compound-test", "compound-session");
    bus.execute(
        vec![set(collider.clone())],
        actor.clone(),
        "Create compound",
        None,
    )
    .unwrap();
    let original = bus.project().clone();
    let mut invalids = Vec::new();
    for parts in [
        json!([]),
        json!(vec![part.clone(); 65]),
        json!([part.clone(), part.clone()]),
    ] {
        let mut bad = collider.clone();
        bad["shape"]["parts"] = parts;
        invalids.push(bad);
    }
    for (field, value) in [
        ("id", json!("invalid")),
        ("translation", json!([10001, 0, 0])),
        ("rotation", json!([0, 0, 0, 0])),
        ("shape", json!({"type":"compound","parts":[]})),
        ("shape", json!({"type":"sphere","radius":-1})),
    ] {
        let mut bad = collider.clone();
        bad["shape"]["parts"][0][field] = value;
        invalids.push(bad);
    }
    for bad in invalids {
        assert!(
            bus.execute(
                vec![
                    Command::RenameEntity {
                        scene_id: sid.clone(),
                        entity_id: eid.clone(),
                        name: "must roll back".into()
                    },
                    set(bad)
                ],
                actor.clone(),
                "Reject invalid part",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project(), &original);
        assert_eq!(bus.revision(), 1);
    }
    collider["shape"]["parts"][0]["translation"] = json!([2, 0, 0]);
    bus.execute(vec![set(collider.clone())], actor, "Move part", None)
        .unwrap();
    assert_eq!(
        bus.project().scenes[&sid].entities[&eid].components["Collider"]["shape"]["parts"][0]["id"],
        part.id
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().scenes, original.scenes);
    drop(bus);
    let mut reopened = CommandBus::persistent(&path, project).unwrap();
    assert_eq!(reopened.project().scenes, original.scenes);
    reopened.redo().unwrap();
    assert_eq!(
        reopened.project().scenes[&sid].entities[&eid].components["Collider"],
        collider
    );
    assert_eq!(
        reopened.project().scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("compound-test")
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
