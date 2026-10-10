use incant_doc::{Collider, ColliderShape, Entity, Project, Scene, Transform};
use incant_script::PlaySession;
use serde_json::json;

fn fixture() -> (Project, String, String, String, serde_json::Value) {
    let mut project = Project::empty("Linked islands");
    let mut scene = Scene::new("World");
    let mut sources = vec![];
    for x in [-4., 4.] {
        let mut floor = Entity::new("Island");
        floor.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, -0.1, 0.],
                ..Default::default()
            }),
        );
        floor.components.insert(
            "Collider".into(),
            json!(Collider {
                shape: ColliderShape::Box {
                    half_extents: [2., 0.1, 2.]
                },
                ..Default::default()
            }),
        );
        sources.push(json!({"entity":floor.id,"geometry":"collider"}));
        scene.entities.insert(floor.id.clone(), floor);
    }
    let mut nav = Entity::new("Navigation");
    let component = json!({"settings":{"min":[-6.,-1.,-2.],"max":[6.,3.,2.],"cell_size":0.1,"cell_height":0.05,
        "tile_cells":32,"agent_radius":0.2,"agent_height":1.7,"max_climb":0.2,"max_slope_degrees":45.},
        "sources":sources,"links":[{"id":"00000000000000000000000001","start":[-2.6,0.,0.],"end":[2.6,0.,0.],
        "snap_distance":0.2,"enabled":true,"bidirectional":false,"extra_cost":0.}]});
    nav.components
        .insert("NavigationMesh".into(), component.clone());
    let mut walker = Entity::new("Walker");
    walker.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-5., 0., 0.],
            ..Default::default()
        }),
    );
    let ids = (scene.id.clone(), nav.id.clone(), walker.id.clone());
    for e in [nav, walker] {
        scene.entities.insert(e.id.clone(), e);
    }
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1, ids.2, component)
}
#[test]
fn authored_links_toggle_through_shared_commands_and_saved_traversals_survive_reload() {
    let (project, scene, nav, walker, component) = fixture();
    let source = format!(
        r#"exports.default={{initialState:{{tick:0,paths:[]}},update(api,dt,state){{
      const path=api.findPath({{scene_id:{scene:?},mesh_entity:{nav:?},path:{{start:[-5,0,0],end:[5,0,0],snap_distance:0.2,max_visited:1000}}}});
      state.tick++;state.paths.push(path?path.traversals:[]);
      if(state.tick===1){{
        const link=path.traversals[0];
        api.command({{op:'set_component',scene_id:{scene:?},entity_id:{walker:?},component:'Transform',value:{{translation:path.points[link.to_index],rotation:[0,0,0,1],scale:[1,1,1]}}}});
      }}
      if(state.tick<=2){{const value={component};value.links[0].enabled=state.tick===2;
        api.command({{op:'set_component',scene_id:{scene:?},entity_id:{nav:?},component:'NavigationMesh',value}});}}
    }}}};"#
    );
    let mut play = PlaySession::new(&project, &source).unwrap();
    play.tick().unwrap();
    assert_eq!(
        play.host.state()["paths"][0][0]["link_id"],
        "00000000000000000000000001"
    );
    let position =
        &play.project().scenes[&scene].entities[&walker].components["Transform"]["translation"];
    assert!((position[0].as_f64().unwrap() - 2.6).abs() < 1e-5);
    let saved = play.save_text().unwrap();
    let mut resumed = PlaySession::from_save(&project, &source, &saved).unwrap();
    resumed.host.hot_reload(&source).unwrap();
    for _ in 0..2 {
        play.tick().unwrap();
        resumed.tick().unwrap();
    }
    assert_eq!(play.host.state(), resumed.host.state());
    // Transaction ULIDs record each independent execution; gameplay content
    // and state must match, while independently created provenance differs.
    for (scene_id, scene) in &play.project().scenes {
        for (id, entity) in &scene.entities {
            assert_eq!(
                entity.components,
                resumed.project().scenes[scene_id].entities[id].components
            );
        }
    }
    assert_eq!(play.host.state()["paths"][1], json!([]));
    assert_eq!(play.host.state()["paths"][2], play.host.state()["paths"][0]);
}
#[test]
fn invalid_enabled_endpoint_cannot_publish_commands_state_or_a_partial_rebuild() {
    let (project, scene, nav, walker, mut component) = fixture();
    component["links"][0]["end"] = json!([50., 0., 0.]);
    let source = format!(
        r#"exports.default={{initialState:{{changed:false}},update(api,dt,state){{
      state.changed=true;api.log("uncommitted");api.setTimer({{id:"pending",delay_ticks:1}});
      api.command({{op:'set_component',scene_id:{scene:?},entity_id:{walker:?},component:'Transform',value:{{translation:[0,0,0],rotation:[0,0,0,1],scale:[1,1,1]}}}});
      api.command({{op:'set_component',scene_id:{scene:?},entity_id:{nav:?},component:'NavigationMesh',value:{component}}});
    }},onTimer(){{}}}};"#
    );
    let mut play = PlaySession::new(&project, &source).unwrap();
    let error = play.tick().unwrap_err();
    assert!(
        matches!(error, incant_script::ScriptError::Document(_)),
        "{error}"
    );
    assert_eq!(play.project(), &project);
    assert_eq!(play.host.state(), &json!({"changed":false}));
    assert!(play.host.take_logs().is_empty());
    assert_eq!(play.host.clock().tick, 0);
    assert!(play.save_text().is_err());
}
