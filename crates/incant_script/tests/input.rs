use incant_doc::{Entity, Project, Scene, Transform};
use incant_input::{InputEvent, KeyCode};
use incant_script::{PlaySession, ScriptError};
use serde_json::{Value, json};

const SOURCE: &str = r#"exports.default={initialState:{ticks:0,presses:0,holds:0,isolated:true},update(api,dt,s){
    s.ticks++;const input=api.input();
    if(input.keyboard.pressed.includes('KeyW'))s.presses++;
    for(const gesture of input.gestures)if(gesture.type==='long_press')s.holds++;
    const speed=(input.keyboard.held.includes('KeyW')?2:0)+(input.gamepads['3']?.left_stick[0]??0);
    for(const e of api.query('Transform'))api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,
        component:'Velocity',value:{linear:[speed,0,0]}});
    input.keyboard.held.push('KeyQ');
    if(api.input().keyboard.held.includes('KeyQ'))s.isolated=false;
    api.log('tick '+s.ticks+' held '+api.input().keyboard.held.includes('KeyW'));
}};"#;
fn setup() -> (Project, String) {
    let mut project = Project::empty("Input game");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Player");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let id = entity.id.clone();
    scene.entities.insert(id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    (project, id)
}
fn clip() -> Value {
    json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":60,"frames":[
        {"tick":1,"events":[{"type":"key","code":"KeyW","down":true},{"type":"gamepad_connected","id":3},
            {"type":"gamepad_axis","id":3,"axis":"left_x","value":0.5},
            {"type":"touch","id":1,"phase":"down","position":[0,0]}]},
        {"tick":40,"events":[{"type":"key","code":"KeyW","down":false},{"type":"gamepad_disconnected","id":3},
            {"type":"touch","id":1,"phase":"up","position":[0,0]}]}
    ]})
}
#[test]
fn live_packets_drive_commands_and_bad_input_does_not_advance_or_poison_play() {
    let (project, id) = setup();
    let authored = project.canonical_text().unwrap();
    let mut play = PlaySession::new(&project, SOURCE).unwrap();
    let before = play.save_text().unwrap();
    assert!(
        play.tick_with_input(&[InputEvent::GamepadDisconnected { id: 99 }])
            .is_err()
    );
    assert_eq!(play.save_text().unwrap(), before);
    play.tick_with_input(&[InputEvent::Key {
        code: KeyCode::KeyW,
        down: true,
    }])
    .unwrap();
    assert_eq!(play.snapshot().entities[&id].velocity, [2., 0., 0.]);
    play.tick().unwrap();
    assert!((play.snapshot().entities[&id].translation[0] - 2. / 60.).abs() < 1e-12);
    play.tick_with_input(&[InputEvent::Focus { focused: false }])
        .unwrap();
    assert_eq!(play.snapshot().entities[&id].velocity, [0., 0., 0.]);
    let state = play.host.state();
    assert_eq!(state["presses"], 1);
    assert_eq!(state["isolated"], true);
    assert_eq!(project.canonical_text().unwrap(), authored);
}

#[test]
fn replay_resumes_saved_game_and_input_history_with_exact_future_state_and_logs() {
    let (project, _) = setup();
    let mut whole = PlaySession::new(&project, SOURCE).unwrap();
    let text = clip().to_string();
    whole.replay_input(&text).unwrap();
    for _ in 0..19 {
        whole.tick().unwrap();
        whole.host.take_logs();
    }
    let save = whole.save_text().unwrap();
    let mut restored = PlaySession::from_save(&project, SOURCE, &save).unwrap();
    assert!(
        restored.input().keyboard.held.is_empty(),
        "ordinary save loads reset physical devices"
    );
    restored.replay_input(&text).unwrap();
    assert_eq!(whole.input(), restored.input());
    assert!(matches!(
        restored.tick_with_input(&[]),
        Err(ScriptError::InputReplayConflict)
    ));
    for tick in 20..=60 {
        whole.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(
            json!(whole.snapshot()),
            json!(restored.snapshot()),
            "tick {tick}"
        );
        assert_eq!(whole.input(), restored.input());
        assert_eq!(whole.host.state(), restored.host.state());
        assert_eq!(
            json!(whole.host.take_logs()),
            json!(restored.host.take_logs())
        );
    }
    assert_eq!(whole.host.state()["presses"], 1);
    assert_eq!(whole.host.state()["holds"], 1);
    let end = whole.save_text().unwrap();
    assert!(whole.tick().is_err());
    assert_eq!(whole.save_text().unwrap(), end);
}

#[test]
fn rejected_future_input_keeps_an_existing_replay_and_current_game_unchanged() {
    let (project, _) = setup();
    let mut play = PlaySession::new(&project, SOURCE).unwrap();
    play.replay_input(&clip().to_string()).unwrap();
    play.tick().unwrap();
    let before = play.save_text().unwrap();
    let input = play.input().clone();
    let mut bad = clip();
    bad["frames"][1]["events"] = json!([{"type":"touch","id":7,"phase":"up","position":[0,0]}]);
    assert!(play.replay_input(&bad.to_string()).is_err());
    assert_eq!(play.save_text().unwrap(), before);
    assert_eq!(play.input(), &input);
    play.tick().unwrap();
    assert_eq!(play.host.state()["ticks"], 2);
}
