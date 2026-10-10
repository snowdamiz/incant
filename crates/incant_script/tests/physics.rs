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

fn character_project() -> (Project, String, String) {
    let (mut project, scene, _, player) = project(false);
    let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
    let character = entities.get_mut(&player).unwrap();
    character.components.get_mut("RigidBody").unwrap()["motion"] = json!("kinematic");
    character.components.get_mut("Transform").unwrap()["translation"] = json!([0., 0.91, 0.]);
    character.components.get_mut("Collider").unwrap()["shape"] =
        json!({"type":"capsule","half_height":0.6,"radius":0.3});
    let mut wall = Entity::new("wall");
    wall.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [2., 2., 0.],
            ..Default::default()
        }),
    );
    wall.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [0.1, 2., 5.]
            },
            ..Default::default()
        }),
    );
    entities.insert(wall.id.clone(), wall);
    (project, scene, player)
}

#[test]
fn scripts_apply_character_movement_through_commands_and_keep_it_after_hot_reload() {
    let (project, scene, player) = character_project();
    let authored = project.canonical_text().unwrap();
    let source = format!(
        r#"exports.default={{initialState:{{ticks:0,grounded:false}},update(api,dt,state){{
      const movement = api.computeCharacterMotion({{scene_id:{scene:?},entity_id:{player:?},translation:[2*dt,-0.01,0]}});
      state.ticks++; state.grounded=movement.grounded;
      api.command({{op:'set_component',scene_id:{scene:?},entity_id:{player:?},component:'Velocity',value:{{linear:movement.translation.map(v=>v/dt)}}}});
    }}}};"#
    );
    let mut session = PlaySession::new(&project, &source).unwrap();
    for tick in 0..120 {
        if tick == 60 {
            session.host.hot_reload(&source).unwrap();
        }
        assert_eq!(session.tick().unwrap(), 1);
    }
    let position = session.snapshot().entities[&player].translation;
    assert!((1.55..1.61).contains(&position[0]), "{position:?}");
    assert!((0.9..0.93).contains(&position[1]), "{position:?}");
    assert_eq!(session.host.state(), &json!({"ticks":120,"grounded":true}));
    assert_eq!(project.canonical_text().unwrap(), authored);
}

#[test]
fn character_and_ray_queries_share_a_budget_and_require_play_capabilities() {
    let (project, scene, player) = character_project();
    let query = format!(r#"{{scene_id:{scene:?},entity_id:{player:?},translation:[0,0,0]}}"#);
    let ray = format!(
        r#"{{scene_id:{scene:?},origin:[0,10,0],direction:[0,-1,0],max_distance:20,include_sensors:false,exclude_entity:null,memberships:4294967295,filter:4294967295}}"#
    );
    let success = format!(
        "exports.default={{initialState:{{ok:false}},update(api,dt,state){{for(let i=0;i<16;i++)api.computeCharacterMotion({query});state.ok=true;}}}};"
    );
    let mut play = PlaySession::new(&project, &success).unwrap();
    play.tick().unwrap();
    assert_eq!(play.host.state()["ok"], true);
    for statement in [
        format!("for(let i=0;i<17;i++)api.computeCharacterMotion({query});"),
        format!("api.raycast({ray});for(let i=0;i<16;i++)api.computeCharacterMotion({query});"),
        "api.computeCharacterMotion({unknown:true});".into(),
    ] {
        let source = format!(
            "exports.default={{initialState:{{ok:false}},update(api,dt,state){{{statement}state.ok=true;}}}};"
        );
        let mut play = PlaySession::new(&project, &source).unwrap();
        assert!(play.tick().is_err());
        assert_eq!(play.host.state()["ok"], false);
    }
    let mut host = ScriptHost::new(&success).unwrap();
    let mut bus = incant_cmd::CommandBus::simulation(project).unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
    assert_eq!(host.state()["ok"], false);
}
