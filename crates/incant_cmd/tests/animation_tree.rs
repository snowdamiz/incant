use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use serde_json::json;

#[test]
fn stable_tree_references_reject_bad_graphs_atomically_and_preserve_undo_provenance() {
    let mut p = Project::empty("Tree");
    let mut scene = Scene::new("Main");
    let e = Entity::new("Rig");
    let sid = scene.id.clone();
    let eid = e.id.clone();
    let aid = new_id();
    scene.entities.insert(eid.clone(), e);
    p.scenes.insert(sid.clone(), scene);
    p.assets.insert(
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
    let parameter = new_id();
    let state = new_id();
    let a = new_id();
    let b = new_id();
    let root = new_id();
    let graph = json!({"initial":state,"parameters":{parameter.clone():{"initial":0.25,"min":0,"max":1}},"states":{state.clone():{"motion":{"kind":"tree","cycle_seconds":2,"tree":{"root":root,"nodes":{a.clone():{"kind":"clip","clip":0},b.clone():{"kind":"clip","clip":null},root.clone():{"kind":"blend1d","parameter":parameter,"points":[{"threshold":0,"node":a},{"threshold":1,"node":b}]}}}},"looping":true}},"transitions":{}});
    let set = |name: &str, value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: name.into(),
        value,
    };
    let cmds = |graph| {
        vec![
            set("AnimationGraph", graph),
            set(
                "Animation",
                json!({"clip":null,"seconds":0,"speed":1,"playing":true,"looping":true}),
            ),
            set(
                "MeshRenderer",
                json!({"mesh":aid,"materials":[],"cast_shadows":false}),
            ),
        ]
    };
    let mut bus = CommandBus::new(p).unwrap();
    let before = bus.project().canonical_text().unwrap();
    for variant in 0..9 {
        let mut bad = graph.clone();
        let tree = &mut bad["states"][&state]["motion"]["tree"];
        match variant {
            0 => tree["root"] = json!(new_id()),
            1 => tree["nodes"][&root]["points"][0]["node"] = json!(root),
            2 => tree["nodes"][&root]["points"][0]["node"] = json!(new_id()),
            3 => tree["nodes"][&root]["points"][1]["threshold"] = json!(0),
            4 => tree["nodes"][&root]["parameter"] = json!(new_id()),
            5 => tree["nodes"][&a]["clip"] = json!(256),
            6 => tree["nodes"][new_id()] = json!({"kind":"clip","clip":0}),
            7 => tree["nodes"][&a]["unknown"] = json!(true),
            _ => {
                let node = tree["nodes"].as_object_mut().unwrap().remove(&a).unwrap();
                tree["nodes"][&parameter] = node;
                tree["nodes"][&root]["points"][0]["node"] = json!(parameter);
            }
        }
        assert!(
            bus.execute(
                cmds(bad),
                Actor::agent("tree-test", "test"),
                "bad tree",
                None
            )
            .is_err(),
            "{variant}"
        );
        assert_eq!(bus.revision(), 0);
        assert_eq!(bus.project().canonical_text().unwrap(), before);
    }
    bus.execute(
        cmds(graph),
        Actor::agent("tree-test", "test"),
        "add tree",
        None,
    )
    .unwrap();
    let after = bus.project().canonical_text().unwrap();
    assert_eq!(Project::from_text(&after).unwrap(), *bus.project());
    assert_eq!(
        bus.project().scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("tree-test")
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
}
