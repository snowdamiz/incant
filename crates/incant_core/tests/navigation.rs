use incant_core::{Engine, NavigationQuery, PathRequest};
use incant_doc::{
    Collider, ColliderShape, Entity, NavigationMesh, NavigationSource, NavigationSourceKind,
    Project, Scene, Transform,
};
use serde_json::json;

#[test]
fn portal_search_avoids_the_rendered_room_tile_boundary_detour() {
    let mut project = Project::empty("Doorway detour regression");
    let mut scene = Scene::new("Room");
    let mut sources = vec![];
    for (name, at, half) in [
        ("Floor", [0., -0.1, 0.], [8., 0.1, 5.]),
        ("North", [0., 0.5, -4.7], [0.15, 0.5, 0.3]),
        ("Middle", [0., 0.5, -1.2], [0.15, 0.5, 1.6]),
        ("South", [0., 0.5, 3.5], [0.15, 0.5, 1.5]),
        ("Screen", [3.6, 0.5, -1.6], [0.15, 0.5, 1.8]),
        ("Pocket west", [5.625, 0.5, 3.], [0.625, 0.5, 0.15]),
        ("Pocket east", [7.375, 0.5, 3.], [0.625, 0.5, 0.15]),
        ("Pocket side", [5., 0.5, 3.925], [0.15, 0.5, 1.075]),
        ("Ridge", [1.3, 0.075, 0.], [0.3, 0.075, 5.]),
        ("Barrier", [0., 0.5, 1.2], [0.15, 0.5, 0.8]),
        ("Pillar", [-3.6, 0.7, 1.], [0., 0., 0.]),
    ] {
        let mut entity = Entity::new(name);
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: at,
                ..Default::default()
            }),
        );
        entity.components.insert(
            "Collider".into(),
            json!(Collider {
                shape: if name == "Pillar" {
                    ColliderShape::Capsule {
                        radius: 0.45,
                        half_height: 0.7,
                    }
                } else {
                    ColliderShape::Box { half_extents: half }
                },
                ..Default::default()
            }),
        );
        sources.push(NavigationSource {
            entity: entity.id.clone(),
            geometry: NavigationSourceKind::Collider,
        });
        scene.entities.insert(entity.id.clone(), entity);
    }
    let mut nav = Entity::new("Navigation");
    let mut component = NavigationMesh {
        settings: Default::default(),
        sources,
    };
    component.settings.min = [-8., -1., -5.];
    component.settings.max = [8., 2.5, 5.];
    component.settings.cell_size = 0.1;
    component.settings.cell_height = 0.05;
    component.settings.tile_cells = 32;
    component.settings.agent_radius = 0.4;
    component.settings.agent_height = 1.7;
    component.settings.max_climb = 0.25;
    nav.components
        .insert("NavigationMesh".into(), json!(component));
    let query = incant_core::NavigationQuery {
        scene_id: scene.id.clone(),
        mesh_entity: nav.id.clone(),
        path: incant_core::PathRequest {
            start: [-3.44, 0., 0.],
            end: [6.6, 0., 1.2],
            snap_distance: 1.,
            max_visited: 4000,
        },
    };
    scene.entities.insert(nav.id.clone(), nav);
    project.scenes.insert(scene.id.clone(), scene);
    let engine = incant_core::Engine::new(&project).unwrap();
    for x in [-2.3, -2.1, -1.9, -1.7] {
        for z in [0.7, 0.9, 1.1, 1.3] {
            let mut floor_query = query.clone();
            floor_query.path.start = [x, 0.05, z];
            floor_query.path.end = [x + 0.01, 0.05, z];
            floor_query.path.snap_distance = 0.06;
            assert!(
                engine.navigator()(floor_query).unwrap().is_some(),
                "clear floor missing at {x},{z}"
            );
        }
    }
    let path = engine.navigator()(query).unwrap().unwrap();
    let length = path
        .points
        .windows(2)
        .map(|p| (p[1][0] - p[0][0]).hypot(p[1][2] - p[0][2]))
        .sum::<f32>();
    assert!(
        length < 14.5,
        "portal route takes {length}m; centroid routing took 15.27m"
    );
    assert!(
        !path
            .points
            .iter()
            .any(|p| (p[0] + 3.3).abs() < 0.01 && (p[2] + 1.8).abs() < 0.01),
        "arbitrary tile-corner detour remains"
    );
}

fn fixture() -> (Project, String, String, String) {
    let mut project = Project::empty("Navigation");
    let mut scene = Scene::new("World");
    let mut floor = Entity::new("Floor");
    floor.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., -0.5, 0.],
            ..Default::default()
        }),
    );
    floor.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [10., 0.5, 6.]
            },
            ..Default::default()
        }),
    );
    let mut obstacle = Entity::new("Room wall");
    obstacle.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 1.5, 0.],
            ..Default::default()
        }),
    );
    obstacle.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [1., 1.5, 2.]
            },
            ..Default::default()
        }),
    );
    let mut nav = Entity::new("Walkable");
    nav.components.insert(
        "NavigationMesh".into(),
        json!(NavigationMesh {
            settings: incant_nav::NavigationSettings {
                min: [-10., -2., -6.],
                max: [10., 5., 6.],
                cell_size: 0.25,
                tile_cells: 16,
                agent_radius: 0.5,
                ..Default::default()
            },
            sources: [&floor, &obstacle]
                .map(|e| NavigationSource {
                    entity: e.id.clone(),
                    geometry: NavigationSourceKind::Collider
                })
                .to_vec(),
        }),
    );
    let ids = (scene.id.clone(), nav.id.clone(), obstacle.id.clone());
    for entity in [floor, obstacle, nav] {
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1, ids.2)
}
fn query(scene: &str, mesh: &str) -> NavigationQuery {
    NavigationQuery {
        scene_id: scene.into(),
        mesh_entity: mesh.into(),
        path: PathRequest {
            start: [-8., 0., 0.],
            end: [8., 0., 0.],
            snap_distance: 1.,
            max_visited: 1000,
        },
    }
}
#[test]
fn authored_sources_rebuild_changed_tiles_and_reject_partial_engine_updates() {
    let (mut project, scene, nav, wall) = fixture();
    let mut engine = Engine::new(&project).unwrap();
    let navigator = engine.navigator();
    let first = navigator(query(&scene, &nav)).unwrap().unwrap();
    assert!(first.points.len() > 2);
    assert_eq!(engine.snapshot().navigation[&nav].rebuilt.len(), 15);
    engine.sync(&project).unwrap();
    assert!(engine.snapshot().navigation[&nav].rebuilt.is_empty());
    assert_eq!(navigator(query(&scene, &nav)).unwrap().unwrap(), first);
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&wall)
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["translation"] = json!([0., 1.5, 20.]);
    engine.sync(&project).unwrap();
    let report = engine.snapshot().navigation[&nav].clone();
    assert!(!report.rebuilt.is_empty() && report.rebuilt.len() < 15);
    let second = navigator(query(&scene, &nav)).unwrap().unwrap();
    assert_eq!(second.points.len(), 2);
    assert!(second.generation > first.generation);
    let before = serde_json::to_value(engine.snapshot()).unwrap();
    // Document accepts physics positions up to 1000 km, but navigation rejects
    // this source beyond its own 100 km bound. No engine subsystem changes.
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&wall)
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["translation"] = json!([200000., 1.5, 0.]);
    assert!(project.validate().is_ok());
    assert!(engine.sync(&project).is_err());
    assert_eq!(serde_json::to_value(engine.snapshot()).unwrap(), before);
    assert_eq!(navigator(query(&scene, &nav)).unwrap().unwrap(), second);
    assert!(navigator(query(&incant_doc::new_id(), &nav)).is_err());
}
#[test]
fn removing_and_readding_mesh_does_not_reuse_old_generation() {
    let (mut project, scene, nav, _) = fixture();
    let mut engine = Engine::new(&project).unwrap();
    let navigator = engine.navigator();
    let first = navigator(query(&scene, &nav)).unwrap().unwrap();
    let component = project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&nav)
        .unwrap()
        .components
        .remove("NavigationMesh")
        .unwrap();
    engine.sync(&project).unwrap();
    assert!(navigator(query(&scene, &nav)).is_err());
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&nav)
        .unwrap()
        .components
        .insert("NavigationMesh".into(), component);
    engine.sync(&project).unwrap();
    assert!(navigator(query(&scene, &nav)).unwrap().unwrap().generation > first.generation);
}
#[test]
fn navigation_rejects_dynamic_sensor_duplicate_and_cross_scene_sources() {
    let (project, scene, nav, wall) = fixture();
    for mode in 0..4 {
        let mut bad = project.clone();
        let entities = &mut bad.scenes.get_mut(&scene).unwrap().entities;
        match mode {
            0 => {
                entities
                    .get_mut(&wall)
                    .unwrap()
                    .components
                    .insert("RigidBody".into(), json!(incant_doc::RigidBody::default()));
            }
            1 => {
                entities
                    .get_mut(&wall)
                    .unwrap()
                    .components
                    .get_mut("Collider")
                    .unwrap()["sensor"] = json!(true)
            }
            2 => {
                let sources = entities
                    .get_mut(&nav)
                    .unwrap()
                    .components
                    .get_mut("NavigationMesh")
                    .unwrap()["sources"]
                    .as_array_mut()
                    .unwrap();
                sources.push(sources[0].clone());
            }
            _ => {
                let entity = entities.remove(&wall).unwrap();
                let mut other = Scene::new("Other");
                other.entities.insert(wall.clone(), entity);
                bad.scenes.insert(other.id.clone(), other);
            }
        }
        assert!(bad.validate().is_err(), "accepted mode {mode}");
    }
    let mut engine = Engine::new(&project).unwrap();
    assert_eq!(
        serde_json::to_value(incant_core::project_entities(&project).unwrap()).unwrap(),
        serde_json::to_value(engine.snapshot().entities).unwrap()
    );
}

#[test]
fn cooked_model_geometry_uses_hierarchy_and_requires_matching_resources() {
    use incant_doc::{Asset, MeshRenderer};
    use incant_nav::{NavigationAsset, NavigationGeometry, NavigationResources};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let (mut project, scene, nav, wall) = fixture();
    let ground = project.scenes[&scene]
        .entities
        .values()
        .find(|e| e.name == "Floor")
        .unwrap()
        .id
        .clone();
    let asset = Asset {
        id: incant_doc::new_id(),
        name: "Floor model".into(),
        path: "floor.glb".into(),
        kind: "model".into(),
        sha256: "a".repeat(64),
        import_settings: None,
    };
    let mut parent = Entity::new("Elevated room");
    parent.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 2., 0.],
            scale: [-1., 1., 1.],
            ..Default::default()
        }),
    );
    let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
    let floor = entities.get_mut(&ground).unwrap();
    floor.parent = Some(parent.id.clone());
    floor.components.remove("Collider");
    floor
        .components
        .insert("Transform".into(), json!(Transform::default()));
    floor.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: asset.id.clone(),
            materials: vec![],
            cast_shadows: true
        }),
    );
    entities.remove(&wall);
    entities
        .get_mut(&nav)
        .unwrap()
        .components
        .get_mut("NavigationMesh")
        .unwrap()["sources"] = json!([{ "entity":ground,"geometry":"mesh" }]);
    entities.insert(parent.id.clone(), parent.clone());
    project.assets.insert(asset.id.clone(), asset.clone());
    assert!(project.validate().is_ok());
    assert!(incant_core::project_entities(&project).is_ok());
    assert!(
        Engine::new(&project).is_err(),
        "missing model must not silently bake an empty floor"
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let tracked = calls.clone();
    let resources = NavigationResources::from([(
        asset.id.clone(),
        NavigationAsset::lazy(asset.sha256.clone(), move || {
            tracked.fetch_add(1, Ordering::Relaxed);
            Ok(NavigationGeometry {
                vertices: vec![
                    [-10., 0., -6.],
                    [10., 0., -6.],
                    [10., 0., 6.],
                    [-10., 0., 6.],
                ],
                triangles: vec![[0, 2, 1], [0, 3, 2]],
            })
        }),
    )]);
    let mut engine = Engine::with_navigation_resources(&project, resources).unwrap();
    let navigator = engine.navigator();
    let mut q = query(&scene, &nav);
    q.path.start[1] = 2.;
    q.path.end[1] = 2.;
    let path = navigator(q.clone()).unwrap().unwrap();
    assert_eq!(path.points.len(), 2);
    assert!(path.points.iter().all(|p| (p[1] - 2.).abs() < 0.2));
    engine.sync(&project).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    project.assets.get_mut(&asset.id).unwrap().sha256 = "b".repeat(64);
    assert!(
        engine.sync(&project).is_err(),
        "stale resources must fail even if source transforms are unchanged"
    );
    assert_eq!(navigator(q).unwrap().unwrap(), path);
    project.assets.get_mut(&asset.id).unwrap().sha256 = asset.sha256;
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&parent.id)
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[1.,0.,0.]}));
    assert!(
        project.validate().is_err(),
        "moving ancestor cannot masquerade as static geometry"
    );
}
