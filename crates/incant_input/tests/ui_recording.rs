use incant_input::{InputReplay, RecordingError};
use serde_json::{Value, json};
fn clip() -> Value {
    json!({"format":"incant-input","version":2,"tick_rate":60,"start_tick":0,"ticks":3,"frames":[{"tick":1,"events":[{"type":"key","code":"Tab","down":true}],"ui":[{"type":"focus","entity_id":"00000000000000000000000001"}]},{"tick":2,"events":[],"ui":[{"type":"preedit","entity_id":"00000000000000000000000001","text":"日本","cursor":[0,6]}],"viewport":[640,480]},{"tick":3,"events":[],"ui":[{"type":"commit","entity_id":"00000000000000000000000001","text":"日本"}]}]})
}
#[test]
fn versioned_ui_replay_preserves_physical_lifetimes_and_refuses_unrestored_widget_seek() {
    let source = clip();
    let replay = InputReplay::from_text(&source.to_string(), 0, 60).unwrap();
    assert!(replay.has_ui());
    assert_eq!(replay.events_at(1).unwrap().len(), 1);
    assert_eq!(replay.ui_at(2).unwrap().len(), 1);
    assert_eq!(replay.viewport_at(2).unwrap(), Some([640., 480.]));
    assert!(matches!(
        InputReplay::from_text(&source.to_string(), 1, 60),
        Err(RecordingError::UiSeek)
    ));
    let mut legacy = source.clone();
    legacy["version"] = json!(1);
    assert!(matches!(
        InputReplay::from_text(&legacy.to_string(), 0, 60),
        Err(RecordingError::Version)
    ));
    legacy["frames"] = json!([{"tick":1,"events":[{"type":"key","code":"Tab","down":true}]}]);
    let replay = InputReplay::from_text(&legacy.to_string(), 1, 60).unwrap();
    assert!(!replay.has_ui());
    assert!(
        replay
            .initial_state()
            .frame()
            .keyboard
            .held
            .contains(&incant_input::KeyCode::Tab)
    );
}
#[test]
fn all_ui_record_bounds_are_checked_before_any_replay() {
    let cases: [fn(&mut Value); 7] = [
        |v| v["frames"][1]["ui"][0]["cursor"] = json!([1, 6]),
        |v| v["frames"][1]["ui"][0]["text"] = json!("x".repeat(16 * 1024 + 1)),
        |v| v["frames"][1]["ui"][0]["entity_id"] = json!("not-an-id"),
        |v| v["frames"][1]["viewport"] = json!([0, 480]),
        |v| v["frames"][1]["ui"] = json!(vec![json!({"type":"activate"}); 257]),
        |v| {
            v["frames"][1]["ui"] = json!(vec![
                json!({"type":"commit","entity_id":"00000000000000000000000001","text":"x".repeat(16*1024)});
                5
            ])
        },
        |v| v["frames"][1]["ui"][0]["shell"] = json!("forbidden"),
    ];
    for case in cases {
        let mut source = clip();
        case(&mut source);
        assert!(InputReplay::from_text(&source.to_string(), 0, 60).is_err());
    }
}
