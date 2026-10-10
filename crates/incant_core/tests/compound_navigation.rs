use incant_core::{Engine, NavigationQuery, PathRequest};
use incant_doc::{
    Collider, ColliderPart, ColliderShape, Entity, NavigationMesh, NavigationSource,
    NavigationSourceKind, PrimitiveColliderShape, Project, Scene, Transform,
};
use serde_json::json;

#[test]
fn compound_navigation_preserves_rotated_child_openings_and_rebuilds_after_edits() {
    let mut p = Project::empty("Compound navigation");
    let mut scene = Scene::new("Arch");
    let mut floor = Entity::new("Floor");
    floor.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., -0.25, 0.],
            ..Default::default()
        }),
    );
    floor.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [8., 0.25, 6.]
            },
            ..Default::default()
        }),
    );
    let mut arch = Entity::new("Hollow arch");
    arch.components
        .insert("Transform".into(), json!(Transform::default()));
    // Rotation converts the long local X dimension into a vertical pillar.
    let q = std::f64::consts::FRAC_PI_4;
    let mut parts = vec![
        ColliderPart {
            id: format!("{:026}", 1),
            translation: [0., 1.5, -2.],
            rotation: [0., 0., q.sin(), q.cos()],
            shape: PrimitiveColliderShape::Box {
                half_extents: [1.5, 0.5, 0.5],
            },
        },
        ColliderPart {
            id: format!("{:026}", 2),
            translation: [0., 1.5, 2.],
            rotation: [0., 0., 0., 1.],
            shape: PrimitiveColliderShape::Capsule {
                half_height: 1.,
                radius: 0.5,
            },
        },
        ColliderPart {
            id: format!("{:026}", 3),
            translation: [0., 3.5, 0.],
            rotation: [0., 0., 0., 1.],
            shape: PrimitiveColliderShape::Box {
                half_extents: [0.5, 0.5, 2.5],
            },
        },
    ];
    arch.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Compound {
                parts: parts.clone()
            },
            ..Default::default()
        }),
    );
    let mut nav = Entity::new("Navigation");
    nav.components.insert(
        "NavigationMesh".into(),
        json!(NavigationMesh {
            links: vec![],
            settings: incant_nav::NavigationSettings {
                min: [-8., -1., -6.],
                max: [8., 5., 6.],
                cell_size: 0.2,
                cell_height: 0.05,
                tile_cells: 32,
                agent_radius: 0.3,
                agent_height: 1.8,
                ..Default::default()
            },
            sources: [&floor, &arch]
                .map(|e| NavigationSource {
                    entity: e.id.clone(),
                    geometry: NavigationSourceKind::Collider
                })
                .to_vec()
        }),
    );
    let (sid, aid, nid) = (scene.id.clone(), arch.id.clone(), nav.id.clone());
    for e in [floor, arch, nav] {
        scene.entities.insert(e.id.clone(), e);
    }
    p.scenes.insert(sid.clone(), scene);
    let mut engine = Engine::new(&p).unwrap();
    let query = NavigationQuery {
        scene_id: sid.clone(),
        mesh_entity: nid,
        path: PathRequest {
            start: [-5., 0., 0.],
            end: [5., 0., 0.],
            snap_distance: 0.2,
            max_visited: 1000,
        },
    };
    let original = engine.navigator()(query.clone()).unwrap().unwrap();
    assert_eq!(
        original.points.len(),
        2,
        "the arch opening must stay hollow"
    );
    // Presentation order cannot change geometry or the resulting route.
    parts.reverse();
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&aid)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["shape"] = json!(ColliderShape::Compound {
        parts: parts.clone()
    });
    engine.sync(&p).unwrap();
    assert_eq!(
        engine.navigator()(query.clone()).unwrap().unwrap().points,
        original.points
    );
    // A new sphere closes the opening and must invalidate affected bake tiles.
    parts.push(ColliderPart {
        id: format!("{:026}", 4),
        translation: [0., 1.5, 0.],
        rotation: [0., 0., 0., 1.],
        shape: PrimitiveColliderShape::Sphere { radius: 1.7 },
    });
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&aid)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["shape"] = json!(ColliderShape::Compound { parts });
    engine.sync(&p).unwrap();
    let blocked = engine.navigator()(query).unwrap().unwrap();
    assert!(
        blocked.points.iter().any(|point| point[2].abs() > 2.5),
        "route must go around the closed arch: {:?}",
        blocked.points
    );
    assert!(blocked.generation > original.generation);
}
