use incant_doc::{Collider, ColliderShape, Entity, Project, RigidBody, Scene, Transform};
use incant_script::{MAX_SAVE_BYTES, PlaySession, SaveError, ScriptError};
use serde_json::{Value, json};

const SOURCE: &str = r#"exports.default={
  initialState:{ticks:0,inventory:{coins:0,items:['map']},flags:[false,true]},
  update(api,dt,state){
    state.ticks++;state.inventory.coins+=2;
    for(const e of api.query('Transform')){
      const t=e.components.Transform;t.translation[1]+=dt;
      api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'Transform',value:t});
    }
    api.log('tick '+state.ticks);
  }
};"#;

fn project() -> (Project, String, String) {
    let mut project = Project::empty("Save test");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Moving entity");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[2,0,0]}));
    let (sid, eid) = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    (project, sid, eid)
}
fn advance(play: &mut PlaySession, ticks: usize) {
    for _ in 0..ticks {
        play.tick().unwrap();
        play.host.take_logs();
    }
}

#[test]
fn save_restores_nested_state_scene_and_clock_then_continues_without_authoring_writes() {
    let (authored, _, _) = project();
    let original = authored.canonical_text().unwrap();
    let mut uninterrupted = PlaySession::new(&authored, SOURCE).unwrap();
    advance(&mut uninterrupted, 23);
    uninterrupted.tick().unwrap();
    let text = uninterrupted.save_text().unwrap();
    assert_eq!(
        uninterrupted.host.take_logs().len(),
        1,
        "save must not consume output"
    );
    let mut restored = PlaySession::from_save(&authored, SOURCE, &text).unwrap();
    assert!(
        restored.host.take_logs().is_empty(),
        "old logs are not replayed"
    );
    assert_eq!(restored.project(), uninterrupted.project());
    assert_eq!(restored.host.state(), uninterrupted.host.state());
    assert_eq!(json!(restored.snapshot()), json!(uninterrupted.snapshot()));
    advance(&mut uninterrupted, 37);
    advance(&mut restored, 37);
    assert_eq!(json!(restored.snapshot()), json!(uninterrupted.snapshot()));
    assert_eq!(restored.host.state(), uninterrupted.host.state());
    assert_eq!(restored.host.state()["inventory"]["coins"], 122);
    let second = restored.save_text().unwrap();
    assert!(PlaySession::from_save(&authored, SOURCE, &second).is_ok());
    assert_eq!(authored.canonical_text().unwrap(), original);
}

#[test]
fn restored_physics_keeps_pose_and_velocities_and_reinstalls_queries() {
    let (mut authored, sid, eid) = project();
    let entity = authored
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap();
    entity
        .components
        .insert("RigidBody".into(), json!(RigidBody::default()));
    entity.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [0.2, 0.3, 0.4]
            },
            ..Default::default()
        }),
    );
    entity
        .components
        .insert("AngularVelocity".into(), json!({"angular":[0.3,0.7,0.2]}));
    entity.components.get_mut("Transform").unwrap()["translation"] = json!([0, 10, 0]);
    let source = format!(
        r#"exports.default={{initialState:{{hits:0}},update(api,dt,s){{
      const e=api.query('Collider')[0];const p=e.components.Transform.translation;
      if(api.raycast({{scene_id:{sid:?},origin:[p[0],p[1]+2,p[2]],direction:[0,-1,0],
        max_distance:4,include_sensors:false,exclude_entity:null,memberships:4294967295,filter:4294967295}}))s.hits++;
    }}}};"#
    );
    let mut original = PlaySession::new(&authored, &source).unwrap();
    advance(&mut original, 12);
    let mut restored =
        PlaySession::from_save(&authored, &source, &original.save_text().unwrap()).unwrap();
    assert_eq!(json!(original.snapshot()), json!(restored.snapshot()));
    advance(&mut original, 12);
    advance(&mut restored, 12);
    let a = original.snapshot();
    let b = restored.snapshot();
    // Logical reload reconstructs the solver. Float roundoff is bounded here;
    // contact/warm-start equality is deliberately not a save-format promise.
    for (a, b) in a.entities[&eid]
        .translation
        .iter()
        .zip(b.entities[&eid].translation)
    {
        assert!((a - b).abs() < 1e-5);
    }
    assert_eq!(restored.host.state()["hits"], 24);
    assert!(
        b.entities[&eid]
            .angular_velocity
            .iter()
            .any(|v| v.abs() > 0.1)
    );
}

#[test]
fn invalid_or_incompatible_saves_fail_without_mutating_a_running_session() {
    let (authored, sid, eid) = project();
    let mut running = PlaySession::new(&authored, SOURCE).unwrap();
    advance(&mut running, 5);
    let before = running.save_text().unwrap();
    let base: Value = serde_json::from_str(&before).unwrap();
    for (pointer, value) in [
        ("/version".into(), json!(3)),
        ("/format".into(), json!("project")),
        ("/authored_sha256".into(), json!("bad")),
        ("/script_sha256".into(), json!("bad")),
        ("/tick".into(), json!(u64::MAX)),
        ("/elapsed_seconds".into(), json!(-1)),
        ("/elapsed_seconds".into(), json!(9000)),
        ("/project/id".into(), json!(incant_doc::new_id())),
        ("/project/settings/tick_rate".into(), json!(120)),
        (
            format!("/project/scenes/{sid}/entities/{eid}/components/Transform/scale/0"),
            json!(0),
        ),
    ] {
        let mut invalid = base.clone();
        *invalid.pointer_mut(&pointer).unwrap() = value;
        assert!(
            PlaySession::from_save(&authored, SOURCE, &invalid.to_string()).is_err(),
            "{pointer}"
        );
    }
    let mut extra = base.clone();
    extra["unknown"] = json!(true);
    assert!(PlaySession::from_save(&authored, SOURCE, &extra.to_string()).is_err());
    let mut manifest = base.clone();
    manifest["project"]["scripts"] = json!({incant_doc::new_id():{}});
    assert!(PlaySession::from_save(&authored, SOURCE, &manifest.to_string()).is_err());
    assert!(PlaySession::from_save(&authored, SOURCE, "{").is_err());
    assert!(matches!(
        PlaySession::from_save(&authored, SOURCE, &" ".repeat(MAX_SAVE_BYTES + 1)),
        Err(SaveError::Size)
    ));
    let mut huge_state = base.clone();
    huge_state["script_state"] = json!("x".repeat(1024 * 1024));
    assert!(matches!(
        PlaySession::from_save(&authored, SOURCE, &huge_state.to_string()),
        Err(SaveError::Size)
    ));
    let mut changed = authored.clone();
    changed.name.push('!');
    assert!(matches!(
        PlaySession::from_save(&changed, SOURCE, &before),
        Err(SaveError::ProjectMismatch)
    ));
    assert!(matches!(
        PlaySession::from_save(&authored, "while(true){}", &before),
        Err(SaveError::ScriptMismatch)
    ));
    assert_eq!(running.save_text().unwrap(), before);
    running.tick().unwrap();
    assert_eq!(running.host.state()["ticks"], 6);
}

#[test]
fn a_failed_tick_cannot_be_saved_or_retried_as_committed_state() {
    let (authored, _, _) = project();
    let source = "exports.default={initialState:{ticks:0},update(api,dt,s){s.ticks++;if(s.ticks===2)throw Error('fail');}};";
    let mut play = PlaySession::new(&authored, source).unwrap();
    play.tick().unwrap();
    let good = play.save_text().unwrap();
    assert!(play.tick().is_err());
    let failed = json!(play.snapshot());
    assert!(matches!(
        play.save_text(),
        Err(SaveError::Script(ScriptError::FailedSession))
    ));
    play.host.hot_reload(SOURCE).unwrap();
    assert!(matches!(play.tick(), Err(ScriptError::FailedSession)));
    assert_eq!(json!(play.snapshot()), failed);
    assert!(PlaySession::from_save(&authored, source, &good).is_ok());
}

#[test]
fn save_tracks_the_successfully_hot_reloaded_script_revision() {
    let (authored, _, _) = project();
    let mut play = PlaySession::new(&authored, SOURCE).unwrap();
    advance(&mut play, 3);
    assert!(play.host.hot_reload("invalid !!!").is_err());
    assert!(PlaySession::from_save(&authored, SOURCE, &play.save_text().unwrap()).is_ok());
    let next = SOURCE.replace("coins+=2", "coins+=3");
    play.host.hot_reload(&next).unwrap();
    play.tick().unwrap();
    let text = play.save_text().unwrap();
    assert!(matches!(
        PlaySession::from_save(&authored, SOURCE, &text),
        Err(SaveError::ScriptMismatch)
    ));
    let mut restored = PlaySession::from_save(&authored, &next, &text).unwrap();
    restored.tick().unwrap();
    assert_eq!(restored.host.state()["inventory"]["coins"], 12);
}

#[test]
fn save_keeps_runtime_spawns_deletions_hierarchy_and_custom_game_data() {
    let (authored, sid, eid) = project();
    let parent = incant_doc::new_id();
    let child = incant_doc::new_id();
    let source = format!(
        r#"exports.default={{initialState:{{ticks:0,collected:[]}},update(api,dt,s){{
        if(!s.ticks){{
          api.command({{op:'delete_entity',scene_id:{sid:?},entity_id:{eid:?}}});
          for(const [id,parent,position] of [[{parent:?},null,[5,0,0]],[{child:?},{parent:?},[1,0,0]]]){{
            api.command({{op:'create_entity',scene_id:{sid:?},entity:{{id,name:'Spawned',parent,provenance:null,
              components:{{Transform:{{translation:position,rotation:[0,0,0,1],scale:[1,1,1]}},Velocity:{{linear:[1,0,0]}}}}}}}});
          }}
          s.collected.push({eid:?});
        }}
        s.ticks++;
      }}}};"#
    );
    let mut original = PlaySession::new(&authored, &source).unwrap();
    advance(&mut original, 8);
    let text = original.save_text().unwrap();
    let mut restored = PlaySession::from_save(&authored, &source, &text).unwrap();
    assert!(!restored.snapshot().entities.contains_key(&eid));
    assert_eq!(restored.snapshot().entities[&child].parent, Some(parent));
    assert_eq!(restored.host.state()["collected"], json!([eid]));
    advance(&mut original, 9);
    advance(&mut restored, 9);
    assert_eq!(json!(original.snapshot()), json!(restored.snapshot()));
    assert_eq!(original.host.state(), restored.host.state());
    assert_eq!(authored.scenes[&sid].entities.len(), 1);
}
