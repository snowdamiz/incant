use incant_core::Engine;
use incant_doc::{Asset, Entity, MeshRenderer, Project, Scene, Transform, new_id};
use serde_json::json;

fn positioned(name: &str, translation: [f64; 3]) -> Entity {
    let mut entity = Entity::new(name);
    entity.components.insert(
        "Transform".into(),
        json!(Transform {
            translation,
            ..Default::default()
        }),
    );
    entity
}
fn near(actual: [f64; 3], expected: [f64; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }
}
fn origin(matrix: [[f64; 4]; 4]) -> [f64; 3] {
    [matrix[3][0], matrix[3][1], matrix[3][2]]
}

#[test]
fn hierarchy_composes_rotation_scale_and_parent_local_motion_before_snapshot() {
    let mut project = Project::empty("Hierarchy");
    let mut scene = Scene::new("First");
    let mut parent = positioned("Parent", [10., 0., 0.]);
    parent.id = "00000000000000000000000002".into();
    parent.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [10., 0., 0.],
            rotation: [
                0.,
                0.,
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2
            ],
            scale: [2., 3., 1.],
        }),
    );
    parent
        .components
        .insert("Velocity".into(), json!({"linear":[0.,1.,0.]}));
    let mut child = positioned("Child sorted before parent", [1., 0., 0.]);
    child.id = "00000000000000000000000001".into();
    child.parent = Some(parent.id.clone());
    child
        .components
        .insert("Velocity".into(), json!({"linear":[1.,0.,0.]}));
    let mut grandchild = positioned("Grandchild", [0., 1., 0.]);
    grandchild.parent = Some(child.id.clone());
    let (parent_id, child_id, grandchild_id) =
        (parent.id.clone(), child.id.clone(), grandchild.id.clone());
    for entity in [parent, child, grandchild] {
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut other = Scene::new("Second");
    let entity = positioned("Independent", [-5., 4., 0.]);
    let other_id = entity.id.clone();
    other.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(other.id.clone(), other);
    let authored = project.clone();
    let mut engine = Engine::new(&project).unwrap();
    let initial = engine.snapshot();
    near(
        origin(initial.entities[&child_id].world_transform),
        [10., 2., 0.],
    );
    near(
        origin(initial.entities[&grandchild_id].world_transform),
        [7., 2., 0.],
    );
    assert_eq!(
        initial.entities[&child_id].parent.as_ref(),
        Some(&parent_id)
    );
    engine.run_ticks(60).unwrap();
    let current = engine.snapshot();
    near(current.entities[&child_id].translation, [2., 0., 0.]);
    near(
        origin(current.entities[&child_id].world_transform),
        [10., 5., 0.],
    );
    near(
        origin(current.entities[&grandchild_id].world_transform),
        [7., 5., 0.],
    );
    near(
        origin(current.entities[&other_id].world_transform),
        [-5., 4., 0.],
    );
    assert_eq!(project, authored);
}

#[test]
fn sync_reparents_and_replaces_bindings_atomically_without_rewinding_time() {
    let mut project = Project::empty("Bindings");
    let asset = Asset {
        id: new_id(),
        name: "Model".into(),
        path: "model.glb".into(),
        kind: "model".into(),
        sha256: "ab".repeat(32),
        import_settings: None,
    };
    let binding = MeshRenderer {
        mesh: asset.id.clone(),
        materials: vec![],
        cast_shadows: true,
    };
    project.assets.insert(asset.id.clone(), asset);
    let mut scene = Scene::new("Scene");
    let parent = positioned("Parent", [5., 0., 0.]);
    let second = positioned("Second", [-8., 0., 0.]);
    let mut child = Entity::new("Identity transform mesh");
    child.parent = Some(parent.id.clone());
    child
        .components
        .insert("MeshRenderer".into(), json!(binding));
    let (parent_id, second_id, child_id, scene_id) = (
        parent.id.clone(),
        second.id.clone(),
        child.id.clone(),
        scene.id.clone(),
    );
    for entity in [parent, second, child] {
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut engine = Engine::new(&project).unwrap();
    engine.run_ticks(30).unwrap();
    let before = engine.snapshot();
    assert_eq!(before.entities[&child_id].mesh.as_ref(), Some(&binding));
    near(
        origin(before.entities[&child_id].world_transform),
        [5., 0., 0.],
    );

    let mut invalid = project.clone();
    invalid
        .scenes
        .get_mut(&scene_id)
        .unwrap()
        .entities
        .get_mut(&parent_id)
        .unwrap()
        .parent = Some(child_id.clone());
    assert!(engine.sync(&invalid).is_err());
    assert_eq!(
        serde_json::to_value(engine.snapshot()).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    let entities = &mut project.scenes.get_mut(&scene_id).unwrap().entities;
    let child = entities.get_mut(&child_id).unwrap();
    child.parent = Some(second_id);
    child.components.remove("MeshRenderer");
    project.settings.tick_rate = 120;
    engine.sync(&project).unwrap();
    let synced = engine.snapshot();
    assert!(synced.entities[&child_id].mesh.is_none());
    near(
        origin(synced.entities[&child_id].world_transform),
        [-8., 0., 0.],
    );
    assert_eq!(synced.tick, 30);
    assert!((synced.elapsed_seconds - 0.5).abs() < 1e-12);
    engine.run_ticks(120).unwrap();
    assert!((engine.snapshot().elapsed_seconds - 1.5).abs() < 1e-12);
}

#[test]
fn deep_hierarchy_is_iterative_and_has_initial_world_transforms() {
    let mut project = Project::empty("Deep");
    let mut scene = Scene::new("Scene");
    let mut parent = None;
    for _ in 0..512 {
        let mut entity = positioned("Node", [1., 0., 0.]);
        entity.parent = parent;
        parent = Some(entity.id.clone());
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let state = Engine::new(&project).unwrap().snapshot();
    near(
        origin(state.entities[&parent.unwrap()].world_transform),
        [512., 0., 0.],
    );
}

#[test]
fn composed_transform_overflow_rejects_sync_without_changing_live_state() {
    let mut project = Project::empty("Overflow");
    let mut scene = Scene::new("Scene");
    let parent = positioned("Parent", [0.; 3]);
    let mut child = positioned("Child", [0.; 3]);
    child.parent = Some(parent.id.clone());
    for entity in [parent, child] {
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut engine = Engine::new(&project).unwrap();
    let before = serde_json::to_value(engine.snapshot()).unwrap();
    for entity in project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
    {
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                scale: [1e200; 3],
                ..Default::default()
            }),
        );
    }
    project.validate().unwrap(); // Each local transform is individually finite.
    let error = engine.sync(&project).unwrap_err();
    assert!(error.to_string().contains("composed world transform"));
    assert_eq!(serde_json::to_value(engine.snapshot()).unwrap(), before);
}

#[test]
fn sync_removes_deleted_entities_and_resets_components_while_moving_scenes() {
    let mut project = Project::empty("Sync");
    let mut scene = Scene::new("Original");
    let mut moving = positioned("Moving", [3., 0., 0.]);
    moving
        .components
        .insert("Velocity".into(), json!({"linear":[1.,0.,0.]}));
    let deleted = Entity::new("Delete me");
    let (moving_id, deleted_id) = (moving.id.clone(), deleted.id.clone());
    scene.entities.insert(moving.id.clone(), moving.clone());
    scene.entities.insert(deleted.id.clone(), deleted);
    project.scenes.insert(scene.id.clone(), scene);
    let mut engine = Engine::new(&project).unwrap();
    engine.run_ticks(60).unwrap();
    near(
        engine.snapshot().entities[&moving_id].translation,
        [4., 0., 0.],
    );
    moving.components.clear();
    let mut replacement = Scene::new("Replacement");
    let added = positioned("Added", [8., 0., 0.]);
    let added_id = added.id.clone();
    replacement.entities.insert(added.id.clone(), added);
    replacement.entities.insert(moving.id.clone(), moving);
    let scene_id = replacement.id.clone();
    project.scenes.clear();
    project.scenes.insert(scene_id.clone(), replacement);
    engine.sync(&project).unwrap();
    engine.step().unwrap();
    let state = engine.snapshot();
    assert_eq!(state.entities.len(), 2);
    assert!(!state.entities.contains_key(&deleted_id));
    assert_eq!(state.entities[&moving_id].scene_id, scene_id);
    near(state.entities[&moving_id].translation, [0.; 3]);
    near(state.entities[&moving_id].velocity, [0.; 3]);
    near(origin(state.entities[&moving_id].world_transform), [0.; 3]);
    near(
        origin(state.entities[&added_id].world_transform),
        [8., 0., 0.],
    );
    assert_eq!(state.tick, 61);
}
