#![cfg(feature = "scene")]
#[path = "../../incant_cmd/tests/support/subtree.rs"]
mod support;
use incant_cmd::Command;
use incant_cook::{cook_scene, stable_id};
use incant_runtime::NativeWorld;
use serde_json::json;

#[test]
fn duplicated_entities_cook_and_simulate_without_a_copy_runtime_path() {
    let mut f = support::Fixture::new();
    f.run(vec![
        Command::SetPhysicsMode {
            mode: incant_runtime::PHYSICS_VARIANT.mode(),
        },
        f.command(),
    ])
    .unwrap();
    let copied = f.mapping[&f.root].clone();
    f.run(vec![f.set(&copied, "Velocity", json!({"linear":[3,0,0]}))])
        .unwrap();
    let bytes = cook_scene(f.bus.project(), &f.scene).unwrap();
    let original = f.root.clone();
    let child = f.mapping[&f.child].clone();
    let leaf = f.mapping[&f.leaf].clone();
    drop(f);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    for (id, x) in [(&original, 1.), (&copied, 3.)] {
        let state = world.inspect(stable_id(id).unwrap()).unwrap();
        assert!((state.transform.translation[0] - x).abs() < 1e-12);
    }
    assert!(world.inspect(stable_id(&child).unwrap()).is_some());
    assert!(world.inspect(stable_id(&leaf).unwrap()).is_some());
}
