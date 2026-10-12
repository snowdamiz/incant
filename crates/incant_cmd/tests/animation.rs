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

#[test]
fn graph_atomic_dependencies_references_bounds_provenance_and_undo() {
    let mut project = Project::empty("Graph");
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
    let set = |component: &str, value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: component.into(),
        value,
    };
    let parameter = new_id();
    let state = new_id();
    let second = new_id();
    let transition = new_id();
    let graph = json!({"initial":state,"parameters":{parameter.clone():{"initial":0,"min":0,"max":1}},"states":{state.clone():{"motion":{"kind":"clip","clip":0},"looping":true},second.clone():{"motion":{"kind":"blend1d","parameter":parameter,"cycle_seconds":2,"points":[{"threshold":0,"clip":0},{"threshold":1,"clip":1}]},"looping":true}},"transitions":{transition.clone():{"from":state,"to":second,"priority":0,"fade_seconds":0,"after_seconds":null,"conditions":[{"parameter":parameter,"comparison":"greater_equal","value":0.5}]}}});
    let animation = json!({"clip":0,"seconds":0,"speed":1,"playing":true,"looping":true});
    let mesh = json!({"mesh":aid,"materials":[],"cast_shadows":false});
    assert!(
        bus.execute(
            vec![set("AnimationGraph", graph.clone())],
            Actor::user("test"),
            "missing playback",
            None
        )
        .is_err()
    );
    for variant in 0..7 {
        let mut bad = graph.clone();
        match variant {
            0 => bad["initial"] = json!(new_id()),
            1 => bad["parameters"][&parameter]["initial"] = json!(2),
            2 => bad["transitions"][&transition]["fade_seconds"] = json!(0.5),
            3 => bad["states"][&second]["motion"]["points"][1]["threshold"] = json!(0),
            4 => bad["states"][&second]["motion"]["kind"] = json!("blend2d"),
            5 => bad["parameters"][&parameter]["unknown"] = json!(true),
            _ => bad["transitions"][&transition]["conditions"][0]["parameter"] = json!(new_id()),
        }
        assert!(
            bus.execute(
                vec![
                    set("AnimationGraph", bad),
                    set("Animation", animation.clone()),
                    set("MeshRenderer", mesh.clone())
                ],
                Actor::agent("graph-test", "test"),
                "bad graph",
                None
            )
            .is_err()
        );
        assert_eq!(bus.revision(), 0);
        assert_eq!(bus.project().canonical_text().unwrap(), before);
    }
    bus.execute(
        vec![
            set("AnimationGraph", graph),
            set("Animation", animation),
            set("MeshRenderer", mesh),
        ],
        Actor::agent("graph-test", "test"),
        "authored graph",
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
        Some("graph-test")
    );
    assert!(
        bus.execute(
            vec![Command::RemoveComponent {
                scene_id: sid,
                entity_id: eid,
                component: "Animation".into()
            }],
            Actor::user("test"),
            "remove graph dependency",
            None
        )
        .is_err()
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
}
