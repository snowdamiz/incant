#![cfg(feature = "scene")]
use incant_cook::{CookError, cook_scene, cook_scene_with_navigation, stable_id};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use incant_nav::{
    GridPathRequest, NavigationAsset, NavigationGeometry, NavigationResources, NavigationSettings,
    PathRequest,
};
use incant_runtime::NativeWorld;
use serde_json::json;
use std::sync::Arc;
fn fixture(mesh: bool) -> (Project, String, String, String) {
    let mut p = Project::empty("Cook navigation");
    p.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut s = Scene::new("World");
    let mut source = Entity::new("Floor");
    source.components.insert(
        "Transform".into(),
        serde_json::to_value(incant_doc::Transform::default()).unwrap(),
    );
    let model = new_id();
    if mesh {
        p.assets.insert(
            model.clone(),
            Asset {
                id: model.clone(),
                name: "Floor".into(),
                path: "absent/floor.glb".into(),
                kind: "model".into(),
                sha256: "08".repeat(32),
                import_settings: None,
            },
        );
        source.components.insert(
            "MeshRenderer".into(),
            json!({"mesh":model,"materials":[],"cast_shadows":false}),
        );
    } else {
        source.components.insert(
            "Collider".into(),
            serde_json::to_value(incant_doc::Collider {
                shape: incant_doc::ColliderShape::Box {
                    half_extents: [10., 0.1, 6.],
                },
                ..Default::default()
            })
            .unwrap(),
        );
    }
    let mut owner = Entity::new("Paths");
    owner.components.insert(
        "NavigationGrid".into(),
        json!({"dimensions":[2,2],"costs":[1,0,1,1]}),
    );
    owner.components.insert("NavigationMesh".into(),json!({"settings":NavigationSettings{min:[-10.,-2.,-6.],max:[10.,5.,6.],cell_size:0.25,cell_height:0.1,tile_cells:16,agent_radius:0.5,agent_height:1.8,max_climb:0.3,max_slope_degrees:45.},"sources":[{"entity":source.id,"geometry":if mesh{"mesh"}else{"collider"}}]}));
    let owner_id = owner.id.clone();
    s.entities.insert(owner.id.clone(), owner);
    s.entities.insert(source.id.clone(), source);
    let sid = s.id.clone();
    p.scenes.insert(sid.clone(), s);
    (p, sid, owner_id, model)
}
fn query() -> PathRequest {
    PathRequest {
        start: [-8., 0.1, 0.],
        end: [8., 0.1, 0.],
        snap_distance: 1.,
        max_visited: 1000,
    }
}
#[test]
fn collider_and_grid_cook_runs_after_project_drop_without_resource_io() {
    let (p, sid, nav, _) = fixture(false);
    let before = serde_json::to_vec(&p).unwrap();
    let bytes = cook_scene(&p, &sid).unwrap();
    assert_eq!(serde_json::to_vec(&p).unwrap(), before);
    assert_eq!(cook_scene(&p, &sid).unwrap(), bytes);
    drop(p);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let first = world
        .navigation_path(stable_id(&nav).unwrap(), &query())
        .unwrap()
        .unwrap();
    for _ in 0..8 {
        world.step().unwrap();
    }
    assert_eq!(
        world
            .navigation_path(stable_id(&nav).unwrap(), &query())
            .unwrap()
            .unwrap(),
        first
    );
    assert_eq!(
        world
            .grid_path(
                stable_id(&nav).unwrap(),
                &GridPathRequest {
                    start: [0, 0],
                    end: [1, 1],
                    diagonal: true,
                    max_expansions: 10
                }
            )
            .unwrap()
            .unwrap()
            .cells,
        vec![[0, 0], [0, 1], [1, 1]]
    );
}
#[test]
fn missing_stale_invalid_and_valid_mesh_resources_are_explicit() {
    let (p, sid, nav, model) = fixture(true);
    assert!(matches!(cook_scene(&p,&sid),Err(CookError::NavigationResource(id)) if id==model));
    let geometry = Arc::new(NavigationGeometry {
        vertices: vec![
            [-10., 0., -6.],
            [10., 0., -6.],
            [10., 0., 6.],
            [-10., 0., 6.],
        ],
        triangles: vec![[0, 2, 1], [0, 3, 2]],
    });
    let mut resources = NavigationResources::from([(
        model.clone(),
        NavigationAsset::new("00".repeat(32), geometry.clone()),
    )]);
    assert!(matches!(
        cook_scene_with_navigation(&p, &sid, &resources),
        Err(CookError::NavigationResource(_))
    ));
    resources.insert(
        model.clone(),
        NavigationAsset::new(
            "08".repeat(32),
            Arc::new(NavigationGeometry {
                vertices: vec![],
                triangles: vec![[0, 1, 2]],
            }),
        ),
    );
    assert!(cook_scene_with_navigation(&p, &sid, &resources).is_err());
    resources.insert(model, NavigationAsset::new("08".repeat(32), geometry));
    let bytes = cook_scene_with_navigation(&p, &sid, &resources).unwrap();
    drop(resources);
    drop(p);
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    let path = world
        .navigation_path(stable_id(&nav).unwrap(), &query())
        .unwrap()
        .unwrap();
    assert_eq!(path.points.len(), 2);
}

#[cfg(feature = "script")]
#[test]
fn actual_foreign_tick_restores_numeric_writes_and_navigation_on_late_rebuild_rejection() {
    use incant_script::{CompiledScript, ScriptHost};
    let (p, sid, nav, model) = fixture(true);
    let resources = NavigationResources::from([(
        model,
        NavigationAsset::new(
            "08".repeat(32),
            Arc::new(NavigationGeometry {
                vertices: vec![
                    [-10., 0., -6.],
                    [10., 0., -6.],
                    [10., 0., 6.],
                    [-10., 0., 6.],
                ],
                triangles: vec![[0, 2, 1], [0, 3, 2]],
            }),
        ),
    )]);
    let bytes = cook_scene_with_navigation(&p, &sid, &resources).unwrap();
    let nav_id = stable_id(&nav).unwrap();
    for tail in ["", "throw Error('late');", "new Promise(()=>{});"] {
        let mut world = NativeWorld::from_bytes(&bytes).unwrap();
        let before = world.snapshot();
        let path = world.navigation_path(nav_id, &query()).unwrap();
        let source = format!(
            "let retained; exports.default=defineBehavior({{update({{chunks}}){{if(retained && retained.byteLength!==0)throw Error('retained loan');for(const c of chunks){{retained=c.transforms;for(let i=0;i<c.transforms.length;i+=c.transformStride)c.transforms[i]=1e9;}}{tail}}}}});"
        );
        let mut host = ScriptHost::new(CompiledScript::compile(&source).unwrap()).unwrap();
        for _ in 0..2 {
            assert!(host.tick(&mut world, 1. / 60.).is_err());
            assert_eq!(world.snapshot(), before);
            assert_eq!(world.navigation_path(nav_id, &query()).unwrap(), path);
        }
        world.step().unwrap();
    }
}

#[cfg(feature = "script")]
#[test]
fn authored_program_and_mesh_geometry_share_one_checked_source_free_package() {
    use incant_cook::{
        ProjectScriptError, prepare_script_scene, prepare_script_scene_with_navigation,
    };
    let (mut project, sid, nav, model) = fixture(true);
    let source = new_id();
    project.scripts.insert(
        source.clone(),
        incant_doc::ScriptSource {
            id: source.clone(),
            name: "Move floor".into(),
            path: "absent/move.ts".into(),
        },
    );
    let floor_id = project.scenes[&sid]
        .entities
        .values()
        .find(|e| e.components.contains_key("MeshRenderer"))
        .unwrap()
        .id
        .clone();
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&floor_id)
        .unwrap()
        .components
        .insert(
            "Script".into(),
            json!({"source":source,"props":{"speed":0.1}}),
        );
    assert!(matches!(
        prepare_script_scene(&project, &sid),
        Err(ProjectScriptError::Cook(CookError::NavigationResource(_)))
    ));
    let resources = NavigationResources::from([(
        model,
        NavigationAsset::new(
            "08".repeat(32),
            Arc::new(NavigationGeometry {
                vertices: vec![
                    [-10., 0., -6.],
                    [10., 0., -6.],
                    [10., 0., 6.],
                    [-10., 0., 6.],
                ],
                triangles: vec![[0, 2, 1], [0, 3, 2]],
            }),
        ),
    )]);
    let before = project.canonical_text().unwrap();
    let prepared = prepare_script_scene_with_navigation(&project, &sid, &resources).unwrap();
    assert_eq!(
        prepared.source_catalog(),
        &json!([{"id":source,"path":"absent/move.ts"}])
    );
    let code = format!(
        "exports.default={{sources:[{{id:{source:?},behavior:{{update({{chunks,instances,dt}}){{for(const chunk of chunks)for(let row=0;row<chunk.ids.length;row++)if(chunk.ids[row]===instances[0].entity)chunk.transforms[row*10+2]+=instances[0].props.speed*dt;}}}}}}]}};"
    );
    let (scene, script) = prepared.compile(&code).unwrap().into_parts();
    assert_eq!(project.canonical_text().unwrap(), before);
    drop(project);
    drop(resources);
    let mut session = incant_script::NativeSession::new(scene, script, Default::default()).unwrap();
    let generation = session
        .world()
        .unwrap()
        .navigation_report(stable_id(&nav).unwrap())
        .unwrap()
        .generation;
    for _ in 0..6 {
        session.tick(&[]).unwrap();
    }
    let world = session.world().unwrap();
    assert!(
        (world
            .inspect(stable_id(&floor_id).unwrap())
            .unwrap()
            .transform
            .translation[2]
            - 0.01)
            .abs()
            < 1e-12
    );
    assert!(
        world
            .navigation_path(stable_id(&nav).unwrap(), &query())
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .navigation_report(stable_id(&nav).unwrap())
            .unwrap()
            .generation
            > generation
    );
}
