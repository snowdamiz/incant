#![cfg(feature = "typescript")]
use incant_cmd::{Actor, Command, CommandBus};
use incant_cook::{cook_scene, prepare_wasm_instances};
use incant_doc::{Asset, Entity, Project, Scene, SceneProgram, new_id};
use incant_runtime::CookedScene;
use incant_types::CoroutineValue as V;
use incant_wasm_abi::instances::{Snapshot, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[test]
fn ordinary_authoring_cooks_bound_nested_initial_data_without_sources_or_runtime_json() {
    let mut project = Project::empty("Cook instances");
    let mut scene = Scene::new("Main");
    let sid = scene.id.clone();
    let a = Entity::new("A");
    let b = Entity::new("B");
    let a_id = a.id.clone();
    let b_id = b.id.clone();
    scene.entities.insert(a.id.clone(), a);
    scene.entities.insert(b.id.clone(), b);
    project.scenes.insert(sid.clone(), scene);
    let module = new_id();
    project.assets.insert(
        module.clone(),
        Asset {
            id: module.clone(),
            name: "Behavior".into(),
            path: "deleted.wasm".into(),
            kind: "wasm".into(),
            sha256: "1".repeat(64),
            import_settings: None,
        },
    );
    let mut bus = CommandBus::new(project).unwrap();
    bus.execute(
        vec![Command::SetSceneProgram {
            scene_id: sid.clone(),
            program: Some(SceneProgram::Wasm {
                module,
                instances: BTreeMap::from([
                    (
                        a_id,
                        BTreeMap::from([
                            ("speed".into(), V::Number(-0.)),
                            (
                                "nested".into(),
                                V::Object(BTreeMap::from([(
                                    "語".into(),
                                    V::Array(vec![V::Bool(true), V::Null]),
                                )])),
                            ),
                        ]),
                    ),
                    (b_id, BTreeMap::from([("speed".into(), V::Number(20.))])),
                ]),
            }),
        }],
        Actor::user("test"),
        "Parameters",
        None,
    )
    .unwrap();
    let bytes = cook_scene(bus.project(), &sid).unwrap();
    let scene = CookedScene::from_bytes(&bytes).unwrap();
    let blob = prepare_wasm_instances(bus.project(), &sid, &scene)
        .unwrap()
        .unwrap();
    assert_eq!(&blob[..32], &Sha256::digest(&bytes)[..]);
    let snapshot = Snapshot::parse(&blob[32..]).unwrap();
    assert_eq!(snapshot.len(), 2);
    let rows: Vec<_> = snapshot.iter().collect();
    assert!(rows[0].entity < rows[1].entity);
    let zero = rows
        .iter()
        .find_map(|r| match r.props.get("speed") {
            Some(Value::Number(n)) if n == 0. => Some(n),
            _ => None,
        })
        .unwrap();
    assert_eq!(zero.to_bits(), (-0.0f64).to_bits());
    let nested = rows
        .iter()
        .find_map(|r| match r.props.get("nested") {
            Some(Value::Object(v)) => Some(v),
            _ => None,
        })
        .unwrap();
    assert!(matches!(nested.get("語"), Some(Value::Array(_))));
    assert_eq!(
        prepare_wasm_instances(bus.project(), &sid, &scene)
            .unwrap()
            .unwrap(),
        blob
    );
}
