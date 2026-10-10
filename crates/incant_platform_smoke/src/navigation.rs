//! Actual collider bake, connected tiles and incremental runtime rebuild on each target.
use incant_core::{Engine, NavigationQuery, PathRequest};
use incant_doc::{
    Collider, ColliderShape, Entity, NavigationMesh, NavigationSource, NavigationSourceKind,
    Project, Scene, Transform,
};
use serde_json::{Value, json};
pub(super) fn check() -> Result<Value, String> {
    let mut project = Project::empty("Navigation smoke");
    let mut scene = Scene::new("Room");
    let mut floor = Entity::new("Floor");
    let mut wall = Entity::new("Wall");
    for (entity, position, size) in [
        (&mut floor, [0., -0.5, 0.], [10., 0.5, 6.]),
        (&mut wall, [0., 1.5, 0.], [1., 1.5, 2.]),
    ] {
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: position,
                ..Default::default()
            }),
        );
        entity.components.insert(
            "Collider".into(),
            json!(Collider {
                shape: ColliderShape::Box { half_extents: size },
                ..Default::default()
            }),
        );
    }
    let mut nav = Entity::new("Navigation");
    let mut component = NavigationMesh {
        settings: Default::default(),
        sources: [&floor, &wall]
            .map(|e| NavigationSource {
                entity: e.id.clone(),
                geometry: NavigationSourceKind::Collider,
            })
            .to_vec(),
    };
    component.settings.min = [-10., -2., -6.];
    component.settings.max = [10., 5., 6.];
    component.settings.cell_size = 0.25;
    component.settings.tile_cells = 16;
    nav.components
        .insert("NavigationMesh".into(), json!(component));
    let scene_id = scene.id.clone();
    let wall_id = wall.id.clone();
    let nav_id = nav.id.clone();
    for entity in [floor, wall, nav] {
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene_id.clone(), scene);
    let mut engine = Engine::new(&project).map_err(|e| e.to_string())?;
    let navigator = engine.navigator();
    let query = NavigationQuery {
        scene_id: scene_id.clone(),
        mesh_entity: nav_id.clone(),
        path: PathRequest {
            start: [-8., 0., 0.],
            end: [8., 0., 0.],
            snap_distance: 1.,
            max_visited: 1000,
        },
    };
    let first = navigator(query.clone())
        .map_err(|e| e.to_string())?
        .ok_or("no initial path")?;
    if first.points.len() <= 2 {
        return Err("path did not detour around collider".into());
    }
    project
        .scenes
        .get_mut(&scene_id)
        .unwrap()
        .entities
        .get_mut(&wall_id)
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["translation"] = json!([0., 1.5, 20.]);
    engine.sync(&project).map_err(|e| e.to_string())?;
    let second = navigator(query)
        .map_err(|e| e.to_string())?
        .ok_or("no rebuilt path")?;
    let report = engine.snapshot().navigation.remove(&nav_id).unwrap();
    if second.points.len() != 2
        || second.generation <= first.generation
        || report.rebuilt.is_empty()
        || report.reused.is_empty()
    {
        return Err("incremental navigation rebuild failed".into());
    }
    Ok(
        json!({"recast_tiled":true,"first_corners":first.points.len(),"second_corners":second.points.len(),"changed_tiles":report.rebuilt.len(),"retained_tiles":report.reused.len(),"visual_gate":false}),
    )
}
