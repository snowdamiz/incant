use incant_doc::{
    Collider, ColliderShape, Entity, NavigationMesh, NavigationSource, NavigationSourceKind,
    Project, Scene, Transform,
};
use incant_script::{PlaySession, ScriptHost};
use serde_json::json;
fn fixture() -> (Project, String, String, String) {
    let mut p = Project::empty("Scripted rooms");
    let mut scene = Scene::new("World");
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
    nav.components.insert(
        "NavigationMesh".into(),
        json!(NavigationMesh {
            settings: Default::default(),
            sources: [&floor, &wall]
                .map(|e| NavigationSource {
                    entity: e.id.clone(),
                    geometry: NavigationSourceKind::Collider
                })
                .to_vec(),
        }),
    );
    // Small public component settings keep this test about live gameplay state.
    let settings = &mut nav.components.get_mut("NavigationMesh").unwrap()["settings"];
    settings["min"] = json!([-10., -2., -6.]);
    settings["max"] = json!([10., 5., 6.]);
    settings["cell_size"] = json!(0.25);
    settings["tile_cells"] = json!(16);
    let ids = (scene.id.clone(), nav.id.clone(), wall.id.clone());
    for e in [floor, wall, nav] {
        scene.entities.insert(e.id.clone(), e);
    }
    p.scenes.insert(scene.id.clone(), scene);
    (p, ids.0, ids.1, ids.2)
}
fn query(scene: &str, nav: &str) -> String {
    json!({"scene_id":scene,"mesh_entity":nav,"path":{"start":[-8.,0.,0.],"end":[8.,0.,0.],"snap_distance":1.,"max_visited":1000}}).to_string()
}
#[test]
fn script_room_edits_rebuild_next_tick_and_paths_survive_reload_and_save() {
    let (project, scene, nav, wall) = fixture();
    let original = project.canonical_text().unwrap();
    let query = query(&scene, &nav);
    let source = format!(
        r#"exports.default={{initialState:{{tick:0,lengths:[],generations:[]}},update(api,dt,state){{
        const p=api.findPath({query});state.tick++;state.lengths.push(p.points.length);state.generations.push(p.generation);
        if(state.tick===1)api.command({{op:'set_component',scene_id:{scene:?},entity_id:{wall:?},component:'Transform',value:{{translation:[0,1.5,20],rotation:[0,0,0,1],scale:[1,1,1]}}}});
    }}}};"#
    );
    let mut play = PlaySession::new(&project, &source).unwrap();
    play.tick().unwrap();
    play.tick().unwrap();
    let state = play.host.state();
    assert!(state["lengths"][0].as_u64().unwrap() > 2);
    assert_eq!(state["lengths"][1], 2);
    assert!(state["generations"][1].as_u64() > state["generations"][0].as_u64());
    play.host.hot_reload(&source).unwrap();
    play.tick().unwrap();
    assert_eq!(
        play.host.state()["generations"][1],
        play.host.state()["generations"][2]
    );
    let text = play.save_text().unwrap();
    let mut restored = PlaySession::from_save(&project, &source, &text).unwrap();
    restored.tick().unwrap();
    assert_eq!(restored.host.state()["lengths"][3], 2);
    assert_eq!(restored.host.state()["tick"], 4);
    assert_eq!(project.canonical_text().unwrap(), original);
}
#[test]
fn shared_query_limits_and_bad_inputs_reject_script_state_and_commands() {
    let (project, scene, nav, wall) = fixture();
    let query = query(&scene, &nav);
    for statement in [
        format!("for(let i=0;i<5;i++)api.findPath({query});"),
        format!("const q={query};q.path.max_visited=0;api.findPath(q);"),
        format!("const q={query};q.mesh_entity='missing';api.findPath(q);"),
    ] {
        let source = format!(
            r#"exports.default={{initialState:{{ok:false}},update(api,dt,state){{
          state.ok=true;api.command({{op:'set_component',scene_id:{scene:?},entity_id:{wall:?},component:'Transform',value:{{translation:[9,9,9],rotation:[0,0,0,1],scale:[1,1,1]}}}});{statement}
        }}}};"#
        );
        let mut play = PlaySession::new(&project, &source).unwrap();
        assert!(play.tick().is_err());
        assert_eq!(play.host.state(), &json!({"ok":false}));
        assert_eq!(play.project(), &project);
        assert!(play.save_text().is_err());
    }
    let mut host = ScriptHost::new(&format!(
        "exports.default={{initialState:{{}},update(api){{api.findPath({query});}}}};"
    ))
    .unwrap();
    let mut bus = incant_cmd::CommandBus::simulation(project).unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
}
