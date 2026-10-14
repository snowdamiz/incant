#![cfg(feature = "scene")]
#[path = "../../incant_cmd/tests/support/prefab.rs"]
mod support;
use incant_cmd::Command;
use incant_cook::{cook_scene, stable_id};
use incant_runtime::NativeWorld;
use serde_json::json;
#[test]
fn linked_instances_cook_as_ordinary_entities_and_run_without_authoring() {
    let mut f = support::Fixture::new();
    // Match the selected native test solver without changing prefab semantics.
    f.run(vec![Command::SetPhysicsMode {
        mode: incant_runtime::PHYSICS_VARIANT.mode(),
    }])
    .unwrap();
    f.create();
    let a = f.instance();
    let b = f.instance();
    let node = f.nodes[&f.source].clone();
    let ar = a[&node].clone();
    let br = b[&node].clone();
    f.run(vec![Command::SetComponent {
        scene_id: f.scene.clone(),
        entity_id: br.clone(),
        component: "Velocity".into(),
        value: json!({"linear":[3,0,0]}),
    }])
    .unwrap();
    let linked = cook_scene(f.bus.project(), &f.scene).unwrap();
    f.run(vec![
        Command::DetachPrefab {
            scene_id: f.scene.clone(),
            root_entity_id: ar.clone(),
        },
        Command::DetachPrefab {
            scene_id: f.scene.clone(),
            root_entity_id: br.clone(),
        },
        Command::DeletePrefab {
            prefab_id: f.prefab.clone(),
        },
    ])
    .unwrap();
    assert_eq!(cook_scene(f.bus.project(), &f.scene).unwrap(), linked);
    drop(f);
    let mut world = NativeWorld::from_bytes(&linked).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    assert!(
        (world
            .inspect(stable_id(&ar).unwrap())
            .unwrap()
            .transform
            .translation[0]
            - 1.)
            .abs()
            < 1e-12
    );
    assert!(
        (world
            .inspect(stable_id(&br).unwrap())
            .unwrap()
            .transform
            .translation[0]
            - 3.)
            .abs()
            < 1e-12
    );
}
