use incant_doc::Project;
use incant_script::PlaySession;
use serde_json::json;

#[test]
fn action_queries_and_runtime_rebinding_resume_with_saved_state_and_ordered_input() {
    let mut project = Project::empty("Rebinding");
    project.settings.input_actions = serde_json::from_value(
        json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}),
    )
    .unwrap();
    let authored = project.canonical_text().unwrap();
    let source = r#"exports.default={initialState:{events:[],active:[],isolated:true},update(api,dt,s){
      const clock=api.clock(),a=api.input().actions.jump;
      if(a.pressed)s.events.push(clock.tick+'+');if(a.released)s.events.push(clock.tick+'-');
      s.active.push(a.active);a.active=!a.active;
      if(api.input().actions.jump.active===a.active)s.isolated=false;
      if(clock.tick===3)api.command({op:'set_input_actions',actions:{jump:{kind:'button',bindings:[{type:'key',code:'KeyE'}]}}});
      api.log(JSON.stringify([clock.tick,s.events,s.active]));
    }};"#;
    let recording =
        json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":8,
        "frames":[{"tick":1,"events":[{"type":"key","code":"Space","down":true}]},
        {"tick":2,"events":[{"type":"key","code":"KeyE","down":true}]},
        {"tick":5,"events":[{"type":"key","code":"KeyE","down":false}]},
        {"tick":6,"events":[{"type":"key","code":"Space","down":false}]},
        {"tick":7,"events":[{"type":"key","code":"KeyE","down":true}]}]})
        .to_string();
    let mut whole = PlaySession::new(&project, source).unwrap();
    assert!(!whole.input().actions["jump"].active);
    whole.replay_input(&recording).unwrap();
    for _ in 0..3 {
        whole.tick().unwrap();
        whole.host.take_logs();
    }
    let save = whole.save_text().unwrap();
    let mut restored = PlaySession::from_save(&project, source, &save).unwrap();
    assert!(
        !restored.input().actions["jump"].active,
        "cold load resets physical controls"
    );
    restored.replay_input(&recording).unwrap();
    assert!(
        restored.input().actions["jump"].active,
        "replay seek seeds the held new binding"
    );
    assert!(!restored.input().actions["jump"].pressed);
    for _ in 3..8 {
        whole.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(whole.host.state(), restored.host.state());
        assert_eq!(whole.save_text().unwrap(), restored.save_text().unwrap());
        assert_eq!(
            json!(whole.host.take_logs()),
            json!(restored.host.take_logs())
        );
        assert_eq!(whole.input(), restored.input());
    }
    assert_eq!(
        whole.host.state(),
        &json!({"events":["1+","5-","7+"],"active":[true,true,true,true,false,false,true,true],"isolated":true})
    );
    assert_eq!(project.canonical_text().unwrap(), authored);
}
