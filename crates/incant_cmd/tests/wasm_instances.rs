use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::CoroutineValue as V;
use incant_doc::{Asset, Entity, Project, Scene, SceneProgram, new_id};
use std::collections::BTreeMap;

fn fixture() -> (CommandBus, String, String, String) {
    let mut project = Project::empty("Wasm parameters");
    let mut scene = Scene::new("Main");
    let owner = Entity::new("Mover");
    let owner_id = owner.id.clone();
    scene.entities.insert(owner.id.clone(), owner);
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let module = new_id();
    project.assets.insert(
        module.clone(),
        Asset {
            id: module.clone(),
            name: "Behavior".into(),
            path: "behavior.wasm".into(),
            kind: "wasm".into(),
            sha256: "1".repeat(64),
            import_settings: None,
        },
    );
    (CommandBus::new(project).unwrap(), sid, owner_id, module)
}
#[test]
fn authored_parameters_share_set_scene_program_provenance_and_undo_redo() {
    let (mut bus, scene, owner, module) = fixture();
    let before = bus.project().clone();
    let props = BTreeMap::from([
        ("speed".into(), V::Number(10.)),
        (
            "options".into(),
            V::Array(vec![
                V::Bool(true),
                V::Null,
                V::Object(BTreeMap::from([("name".into(), V::String("語".into()))])),
            ]),
        ),
    ]);
    bus.execute(
        vec![Command::SetSceneProgram {
            scene_id: scene.clone(),
            program: Some(SceneProgram::Wasm {
                module,
                instances: BTreeMap::from([(owner, props)]),
            }),
        }],
        Actor::agent("test-model", "catalog-turn"),
        "Configure module",
        Some(0),
    )
    .unwrap();
    let after = bus.project().clone();
    assert_eq!(bus.history().len(), 1);
    assert_eq!(
        bus.history()[0].actor,
        Actor::agent("test-model", "catalog-turn")
    );
    let text = after.canonical_text().unwrap();
    assert_eq!(Project::from_text(&text).unwrap(), after);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &after);
}
#[test]
fn invalid_owners_properties_and_stale_revisions_preserve_the_entire_transaction() {
    for invalid in [
        V::Number(f64::NAN),
        V::String("x".repeat(1024 * 1024)),
        (0..17).fold(V::Null, |v, _| V::Array(vec![v])),
    ] {
        let (mut bus, scene, owner, module) = fixture();
        let before = bus.project().clone();
        assert!(
            bus.execute(
                vec![
                    Command::SetMemory {
                        section: "must_not_publish".into(),
                        text: "staged".into()
                    },
                    Command::SetSceneProgram {
                        scene_id: scene,
                        program: Some(SceneProgram::Wasm {
                            module,
                            instances: BTreeMap::from([(
                                owner,
                                BTreeMap::from([("bad".into(), invalid)])
                            )])
                        })
                    }
                ],
                Actor::user("test"),
                "Invalid catalog",
                Some(0)
            )
            .is_err()
        );
        assert_eq!(bus.project(), &before);
        assert!(bus.history().is_empty());
    }
    let (mut bus, scene, owner, module) = fixture();
    let before = bus.project().clone();
    for (id, revision) in [(new_id(), 0), (owner, 1)] {
        assert!(
            bus.execute(
                vec![Command::SetSceneProgram {
                    scene_id: scene.clone(),
                    program: Some(SceneProgram::Wasm {
                        module: module.clone(),
                        instances: BTreeMap::from([(id, BTreeMap::new())])
                    })
                }],
                Actor::user("test"),
                "Refuse",
                Some(revision)
            )
            .is_err()
        );
        assert_eq!(bus.project(), &before);
        assert!(bus.history().is_empty());
    }
}
#[test]
fn legacy_module_selector_round_trips_without_new_fields() {
    let (mut bus, scene, _, module) = fixture();
    let json = serde_json::json!({"kind":"wasm","module":module});
    let program: SceneProgram = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&program).unwrap(), json);
    bus.execute(
        vec![Command::SetSceneProgram {
            scene_id: scene,
            program: Some(program),
        }],
        Actor::user("test"),
        "Legacy module",
        Some(0),
    )
    .unwrap();
    assert!(bus.project().validate().is_ok());
}
