use incant_doc::{Entity, Project, Scene, Transform};
use incant_script::{PlaySession, ScriptHost};
use serde_json::json;

fn fixture() -> (Project, String, Vec<String>) {
    let mut p = Project::empty("Local avoidance");
    let mut scene = Scene::new("Flat plane");
    let mut ids = vec![];
    for x in [-1., 1.] {
        let mut e = Entity::new("Walker");
        e.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, 0., 0.],
                ..Default::default()
            }),
        );
        e.components
            .insert("Velocity".into(), json!({"linear":[0.,0.,0.]}));
        ids.push(e.id.clone());
        scene.entities.insert(e.id.clone(), e);
    }
    let id = scene.id.clone();
    p.scenes.insert(id.clone(), scene);
    (p, id, ids)
}
fn query(ids: &[String]) -> serde_json::Value {
    json!({"agents":ids.iter().enumerate().map(|(i,id)|json!({"id":id,"position":[if i==0 {-1.} else {1.},0.,0.],"velocity":[0.,0.],"preferred_velocity":[if i==0 {1.} else {-1.},0.],"radius":0.25,"height":1.8,"max_speed":2.,"responsibility":1.})).collect::<Vec<_>>(),"time_horizon":2.,"obstacle_time_horizon":2.})
}
#[test]
fn proposed_velocities_use_shared_commands_and_survive_reload_and_save() {
    let (project, scene, ids) = fixture();
    let q = query(&ids);
    let source = format!(
        r#"exports.default={{initialState:{{tick:0,last:[]}},update(api,dt,state){{
      const q={q};
      for(const a of q.agents){{const e=api.query().find(e=>e.id===a.id);a.position=e.components.Transform.translation;a.velocity=[e.components.Velocity.linear[0],e.components.Velocity.linear[2]];}}
      state.last=api.steerAgents(q);state.tick++;
      for(const v of state.last)api.command({{op:'set_component',scene_id:{scene:?},entity_id:v.id,component:'Velocity',value:{{linear:[v.velocity[0],0,v.velocity[1]]}}}});
    }}}};"#
    );
    let original = project.canonical_text().unwrap();
    let mut play = PlaySession::new(&project, &source).unwrap();
    play.tick().unwrap();
    assert_eq!(play.snapshot().entities[&ids[0]].translation, [-1., 0., 0.]);
    assert!(
        play.project().scenes[&scene].entities[&ids[0]].components["Velocity"]["linear"][0]
            .as_f64()
            .unwrap()
            > 0.
    );
    for _ in 1..30 {
        play.tick().unwrap();
    }
    let save = play.save_text().unwrap();
    play.host.hot_reload(&source).unwrap();
    let mut resumed = PlaySession::from_save(&project, &source, &save).unwrap();
    for _ in 0..30 {
        play.tick().unwrap();
        resumed.tick().unwrap();
    }
    assert_eq!(play.host.state(), resumed.host.state());
    assert_eq!(
        serde_json::to_value(play.snapshot()).unwrap(),
        serde_json::to_value(resumed.snapshot()).unwrap()
    );
    let entities = play.snapshot().entities;
    assert!(entities[&ids[0]].translation[0] > -1.);
    let a = entities[&ids[0]].translation;
    let b = entities[&ids[1]].translation;
    assert!((a[0] - b[0]).hypot(a[2] - b[2]) >= 0.5);
    assert_eq!(project.canonical_text().unwrap(), original);
}
#[test]
fn query_budget_bad_input_and_missing_play_session_fail_without_committing() {
    let (project, scene, ids) = fixture();
    let q = query(&ids);
    for statement in [
        format!("for(let i=0;i<3;i++)api.steerAgents({q});"),
        format!("const q={q};q.agents[0].radius=0;api.steerAgents(q);"),
        format!("const q={q};q.agents[0].extra='x'.repeat(65536);api.steerAgents(q);"),
        format!("const q={q};q.agents[0].extra=true;api.steerAgents(q);"),
        format!(
            "api.steerAgents({q});api.steerAgents({q});api.raycast({{origin:[0,0,0],direction:[1,0,0],max_distance:1}});"
        ),
    ] {
        let source = format!(
            r#"exports.default={{initialState:{{ok:false}},update(api,dt,state){{state.ok=true;api.command({{op:'set_component',scene_id:{scene:?},entity_id:{:?},component:'Velocity',value:{{linear:[9,0,0]}}}});{statement}}}}};"#,
            ids[0]
        );
        let mut play = PlaySession::new(&project, &source).unwrap();
        assert!(play.tick().is_err());
        assert_eq!(play.host.state(), &json!({"ok":false}));
        assert_eq!(play.project(), &project);
        assert!(play.save_text().is_err());
    }
    let mut host = ScriptHost::new(&format!(
        "exports.default={{update(api){{api.steerAgents({q});}}}};"
    ))
    .unwrap();
    let mut bus = incant_cmd::CommandBus::simulation(project).unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
}
#[test]
fn overlap_recovery_uses_the_sessions_fixed_timestep() {
    let (mut project, _, ids) = fixture();
    let mut q = query(&ids);
    q["margin"] = json!(0.);
    for i in 0..2 {
        q["agents"][i]["position"] = json!([i as f32 * 0.09, 0., 0.]);
        q["agents"][i]["radius"] = json!(0.05);
        q["agents"][i]["preferred_velocity"] = json!([0., 0.]);
    }
    let source = format!(
        "exports.default={{initialState:{{velocities:[]}},update(api,dt,state){{state.velocities=api.steerAgents({q});}}}};"
    );
    let mut speeds = vec![];
    for rate in [30, 120] {
        project.settings.tick_rate = rate;
        let mut play = PlaySession::new(&project, &source).unwrap();
        play.tick().unwrap();
        speeds.push(
            play.host.state()["velocities"][0]["velocity"][0]
                .as_f64()
                .unwrap()
                .abs(),
        );
    }
    assert!((speeds[1] / speeds[0] - 4.).abs() < 0.001, "{speeds:?}");
}
