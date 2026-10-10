//! Portable engine-grid queries; no renderer or scripting VM is implied.
use incant_core::{Engine, GridNavigationQuery, GridPathRequest};
use incant_doc::{Entity, NavigationGrid, Project, Scene};
use serde_json::{Value, json};

pub(super) fn check() -> Result<Value, String> {
    let mut project = Project::empty("Grid platform smoke");
    let mut scene = Scene::new("Cells");
    let mut grid = Entity::new("Weighted cells");
    let mut corners = Entity::new("Blocked corner");
    grid.components.insert(
        "NavigationGrid".into(),
        json!(NavigationGrid {
            dimensions: [3, 3],
            costs: vec![1, 1, 1, 1, 100, 1, 1, 1, 1],
        }),
    );
    corners.components.insert(
        "NavigationGrid".into(),
        json!(NavigationGrid {
            dimensions: [2, 2],
            costs: vec![1, 0, 0, 1],
        }),
    );
    let scene_id = scene.id.clone();
    let grid_id = grid.id.clone();
    let corner_id = corners.id.clone();
    scene.entities.insert(grid_id.clone(), grid);
    scene.entities.insert(corner_id.clone(), corners);
    project.scenes.insert(scene_id.clone(), scene);
    let request = |id: &str, start, end| GridNavigationQuery {
        scene_id: scene_id.clone(),
        grid_entity: id.into(),
        path: GridPathRequest {
            start,
            end,
            diagonal: true,
            max_expansions: 16,
        },
    };
    let query = request(&grid_id, [0, 1], [2, 1]);
    let corner_query = request(&corner_id, [0, 0], [1, 1]);
    let mut engine = Engine::new(&project).map_err(|e| e.to_string())?;
    let find = engine.grid_navigator();
    let first = find(query.clone())
        .map_err(|e| e.to_string())?
        .ok_or("grid detour absent")?;
    if first.cost != 2828 || first.cells.contains(&[1, 1]) {
        return Err("weighted grid detour is not optimal".into());
    }
    if find(corner_query.clone())
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("grid diagonal cut a blocked corner".into());
    }
    let entities = &mut project.scenes.get_mut(&scene_id).unwrap().entities;
    entities
        .get_mut(&grid_id)
        .unwrap()
        .components
        .get_mut("NavigationGrid")
        .unwrap()["costs"] = json!(vec![1; 9]);
    entities
        .get_mut(&corner_id)
        .unwrap()
        .components
        .get_mut("NavigationGrid")
        .unwrap()["costs"] = json!([1, 1, 0, 1]);
    engine.sync(&project).map_err(|e| e.to_string())?;
    let next = find(query.clone())
        .map_err(|e| e.to_string())?
        .ok_or("grid shortcut absent")?;
    let corner = find(corner_query)
        .map_err(|e| e.to_string())?
        .ok_or("grid orthogonal detour absent")?;
    if next.cost != 2000
        || next.generation <= first.generation
        || corner.cost != 2000
        || corner.cells != [[0, 0], [1, 0], [1, 1]]
    {
        return Err("grid runtime edit failed".into());
    }
    let text = project.canonical_text().map_err(|e| e.to_string())?;
    let reopened = Project::from_text(&text).map_err(|e| e.to_string())?;
    let fresh = Engine::new(&reopened).map_err(|e| e.to_string())?;
    let restored = fresh.grid_navigator()(query)
        .map_err(|e| e.to_string())?
        .ok_or("restored grid path absent")?;
    if restored.cells != next.cells || restored.cost != next.cost {
        return Err("grid document restart changed the route".into());
    }
    Ok(
        json!({"weighted_cost":first.cost,"edited_cost":next.cost,"corner_cost":corner.cost,
        "snapshot_changed":true,"restart_route_equal":true,"visual_gate":false,"script_vm":false}),
    )
}
