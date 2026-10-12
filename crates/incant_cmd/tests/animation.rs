use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use serde_json::json;
#[test]
fn animation_authoring_is_schema_checked_atomic_reversible_and_provenanced() {
    let mut project = Project::empty("Animate");
    let mut scene = Scene::new("Main");
    let entity = Entity::new("Rig");
    let sid = scene.id.clone();
    let eid = entity.id.clone();
    let aid = new_id();
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    project.assets.insert(
        aid.clone(),
        Asset {
            id: aid.clone(),
            name: "Rig".into(),
            path: "rig.gltf".into(),
            kind: "model".into(),
            sha256: "ab".repeat(32),
            import_settings: None,
        },
    );
    let mut bus = CommandBus::new(project).unwrap();
    let before = bus.project().canonical_text().unwrap();
    let set = |name: &str, value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: name.into(),
        value,
    };
    let valid = json!({"clip":0,"seconds":0.25,"speed":1,"playing":true,"looping":true});
    assert!(
        bus.execute(
            vec![set("Animation", valid.clone())],
            Actor::user("test"),
            "missing mesh",
            None
        )
        .is_err()
    );
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    let mesh = json!({"mesh":aid,"materials":[],"cast_shadows":true});
    for (key, value) in [
        ("clip", json!(256)),
        ("seconds", json!(-1)),
        ("speed", json!(16.1)),
        ("playing", json!(1)),
        ("unknown", json!(true)),
    ] {
        let mut bad = valid.clone();
        bad[key] = value;
        assert!(
            bus.execute(
                vec![set("MeshRenderer", mesh.clone()), set("Animation", bad)],
                Actor::agent("animation-test", "test"),
                "bad animation",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), before);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set("Animation", valid), set("MeshRenderer", mesh)],
        Actor::agent("animation-test", "test"),
        "animate rig",
        None,
    )
    .unwrap();
    let authored = bus.project().canonical_text().unwrap();
    assert_eq!(Project::from_text(&authored).unwrap(), *bus.project());
    assert_eq!(
        bus.project().scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("animation-test")
    );
    assert!(
        bus.execute(
            vec![Command::RemoveComponent {
                scene_id: sid,
                entity_id: eid,
                component: "MeshRenderer".into()
            }],
            Actor::user("test"),
            "remove dependency",
            None
        )
        .is_err()
    );
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
}
