use incant_doc::{Collider, ColliderShape, Entity, Project, RigidBody, Scene, Transform};
use incant_script::{PlaySession, ScriptHost};
use serde_json::json;
fn project(sensor: bool) -> (Project, String, String, String) {
    let mut project = Project::empty("Play physics");
    let mut scene = Scene::new("World");
    let mut ground = Entity::new("ground");
    ground.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., -0.5, 0.],
            ..Default::default()
        }),
    );
    ground.components.insert(
        "Collider".into(),
        json!(Collider {
            sensor,
            shape: ColliderShape::Box {
                half_extents: [10., 0.5, 10.]
            },
            ..Default::default()
        }),
    );
    let mut body = Entity::new("body");
    body.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 4., 0.],
            ..Default::default()
        }),
    );
    body.components
        .insert("Collider".into(), json!(Collider::default()));
    body.components
        .insert("RigidBody".into(), json!(RigidBody::default()));
    let ids = (scene.id.clone(), ground.id.clone(), body.id.clone());
    scene.entities.insert(ground.id.clone(), ground);
    scene.entities.insert(body.id.clone(), body);
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1, ids.2)
}
#[test]
fn play_echoes_physics_queries_commands_and_hot_reload_without_editing_author() {
    let (project, scene, ground, body) = project(false);
    let original = project.canonical_text().unwrap();
    let source = format!(
        r#"exports.default={{initialState:{{tick:0,hit:'',speed:0}},update(api,dt,state){{
      state.tick++;
      const body = api.query('RigidBody')[0];
      state.speed = body.components.Velocity.linear[1];
      const hit = api.raycast({{scene_id:{scene:?},origin:[0,10,0],direction:[0,-1,0],max_distance:20,include_sensors:false,exclude_entity:body.id,memberships:4294967295,filter:4294967295}});
      state.hit = hit.entity_id;
      if (state.tick === 121) api.command({{op:'set_component',scene_id:body.scene_id,entity_id:body.id,component:'Velocity',value:{{linear:[0,5,0]}}}});
    }}}};"#
    );
    let mut session = PlaySession::new(&project, &source).unwrap();
    for _ in 0..120 {
        session.tick().unwrap();
    }
    assert_eq!(session.host.state()["hit"], ground);
    assert!((session.snapshot().entities[&body].translation[1] - 0.5).abs() < 0.02);
    session.host.hot_reload(&source).unwrap();
    session.tick().unwrap();
    session.tick().unwrap();
    assert!(session.host.state()["speed"].as_f64().unwrap() > 4.);
    assert!(session.snapshot().entities[&body].translation[1] > 0.55);
    assert_eq!(project.canonical_text().unwrap(), original);
    assert_eq!(session.host.state()["tick"], 122);
    let velocity = session.snapshot().entities[&body].velocity[1];
    assert_eq!(
        session.project().scenes[&scene].entities[&body].components["Velocity"]["linear"][1],
        json!(velocity)
    );
}
#[test]
fn scripts_observe_sensor_transitions_once_and_queries_have_limits() {
    let (project, _, _, _) = project(true);
    let source = "exports.default={initialState:{enter:0,exit:0},update(api,dt,state){for(const event of api.triggerEvents()){ if(event.entered)state.enter++;else state.exit++; }}};";
    let mut session = PlaySession::new(&project, source).unwrap();
    for _ in 0..180 {
        session.tick().unwrap();
    }
    assert_eq!(session.host.state(), &json!({"enter":1,"exit":1}));
    let scene = project.scenes.keys().next().unwrap();
    let query = format!(
        r#"{{scene_id:{scene:?},origin:[0,10,0],direction:[0,-1,0],max_distance:20,include_sensors:false,exclude_entity:null,memberships:4294967295,filter:4294967295}}"#
    );
    for statement in [
        format!("for(let i=0;i<257;i++)api.raycast({query});"),
        "api.raycast({unknown:true});".into(),
    ] {
        let mut play = PlaySession::new(&project, &format!("exports.default={{initialState:{{ok:false}},update(api,dt,state){{{statement} state.ok=true;}}}};")).unwrap();
        assert!(play.tick().is_err());
        assert_eq!(play.host.state()["ok"], false);
    }
    let mut host = ScriptHost::new(&format!(
        "exports.default={{update(api){{api.raycast({query});}}}};"
    ))
    .unwrap();
    let mut bus = incant_cmd::CommandBus::simulation(project).unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
}
