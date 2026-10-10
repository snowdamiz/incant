use incant_input::*;
use serde_json::{Value, json};

fn clip() -> Value {
    json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":100,"ticks":60,"frames":[
        {"tick":1,"events":[{"type":"key","code":"KeyW","down":true},
            {"type":"touch","id":1,"phase":"down","position":[5,6]}]},
        {"tick":50,"events":[{"type":"key","code":"KeyW","down":false},
            {"type":"touch","id":1,"phase":"up","position":[5,6]}]}
    ]})
}

#[test]
fn seeking_preserves_held_controls_and_gesture_age_without_replaying_the_game() {
    let text = clip().to_string();
    let whole = InputReplay::from_text(&text, 100, 60).unwrap();
    let resumed = InputReplay::from_text(&text, 119, 60).unwrap();
    let mut control = whole.initial_state();
    let mut restored = resumed.initial_state();
    for tick in 101..=119 {
        control
            .advance(whole.events_at(tick).unwrap(), 1. / 60.)
            .unwrap();
    }
    assert_eq!(control.frame(), restored.frame());
    assert!(restored.frame().keyboard.held.contains(&KeyCode::KeyW));
    let mut holds = 0;
    for tick in 120..=160 {
        control
            .advance(whole.events_at(tick).unwrap(), 1. / 60.)
            .unwrap();
        restored
            .advance(resumed.events_at(tick).unwrap(), 1. / 60.)
            .unwrap();
        assert_eq!(control.frame(), restored.frame(), "tick {tick}");
        holds += restored
            .frame()
            .gestures
            .iter()
            .filter(|g| matches!(g, Gesture::LongPress { .. }))
            .count();
    }
    assert_eq!(holds, 1);
    assert!(resumed.events_at(100).is_err());
    assert!(resumed.events_at(161).is_err());
    let end = InputReplay::from_text(&text, 160, 60).unwrap();
    assert!(end.initial_state().frame().keyboard.held.is_empty());
}

#[test]
fn the_entire_clip_is_validated_before_play_with_order_limits_and_compatibility() {
    for (path, value) in [
        ("/format", json!("other")),
        ("/version", json!(2)),
        ("/tick_rate", json!(120)),
        ("/ticks", json!(10001)),
        ("/start_tick", json!(u64::MAX)),
        ("/frames/0/tick", json!(0)),
        ("/frames/1/tick", json!(1)),
        ("/frames/1/tick", json!(61)),
    ] {
        let mut c = clip();
        *c.pointer_mut(path).unwrap() = value;
        assert!(
            InputReplay::from_text(&c.to_string(), 100, 60).is_err(),
            "{path}"
        );
    }
    let mut c = clip();
    c["frames"][1]["events"] = json!([{"type":"gamepad_disconnected","id":5}]);
    assert!(matches!(
        InputReplay::from_text(&c.to_string(), 100, 60),
        Err(RecordingError::Input { tick: 50, .. })
    ));
    for tick in [99, 161] {
        assert!(matches!(
            InputReplay::from_text(&clip().to_string(), tick, 60),
            Err(RecordingError::Range)
        ));
    }
    c = clip();
    c["frames"][0]["events"] = json!(vec![
        json!({"type":"focus","focused":true});
        MAX_EVENTS_PER_TICK + 1
    ]);
    assert!(matches!(
        InputReplay::from_text(&c.to_string(), 100, 60),
        Err(RecordingError::Size)
    ));
    assert!(matches!(
        InputReplay::from_text(&" ".repeat(MAX_RECORDING_BYTES + 1), 100, 60),
        Err(RecordingError::Size)
    ));
    c = clip();
    c["unexpected"] = json!(true);
    assert!(InputReplay::from_text(&c.to_string(), 100, 60).is_err());
}

#[test]
fn zero_duration_clips_can_describe_a_checkpoint_but_cannot_supply_another_tick() {
    let text=json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":50,"ticks":0,"frames":[]}).to_string();
    let replay = InputReplay::from_text(&text, 50, 60).unwrap();
    assert_eq!(replay.end_tick(), 50);
    assert!(replay.events_at(51).is_err());
}
