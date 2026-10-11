use super::*;
use crate::{SaveError, TimerError};
use incant_doc::Project;
use serde_json::{Value, json};
use std::time::Duration;

#[test]
fn malformed_schedules_and_handler_removal_are_rejected_without_losing_pending_work() {
    let project = Project::empty("Timer validation");
    let source =
        r#"exports.default={update(api){api.setTimer({id:'one',delay_ticks:5})},onTimer(){}}"#;
    // This case checks save validation and pending-work preservation, not CPU
    // performance. Production constructors and the separate deadline tests keep
    // their 50 ms allowance. Hot reload inherits this explicit fixture budget.
    let mut play =
        PlaySession::with_host_factory(&project, incant_core::NavigationResources::new(), || {
            ScriptHost::with_budget(source, Duration::from_secs(1))
        })
        .unwrap();
    play.tick().unwrap();
    let before = play.save_text().unwrap();
    assert!(matches!(
        play.host.hot_reload("exports.default={update(){}}"),
        Err(ScriptError::TimerHandler)
    ));
    assert_eq!(play.save_text().unwrap(), before);
    let valid: Value = serde_json::from_str(&before).unwrap();
    for (pointer, value) in [
        ("/schedule", json!(null)),
        ("/schedule/clock/tick", json!(0)),
        ("/schedule/timers/one/due_tick", json!(1)),
        ("/schedule/timers/one/interval_ticks", json!(0)),
        ("/schedule/timers/one/payload", json!("x".repeat(17000))),
        ("/schedule/timers/one/due_tick", json!(9007199254740991_u64)),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        // A decoder/VM startup failure must not masquerade as rejection of the
        // malformed schedule. Each fixture must hit its intended typed check.
        assert!(
            matches!(
                (
                    pointer,
                    PlaySession::from_save(&project, source, &bad.to_string())
                ),
                ("/schedule", Err(SaveError::Version))
                    | ("/schedule/clock/tick", Err(SaveError::Clock))
                    | (
                        "/schedule/timers/one/due_tick",
                        Err(SaveError::Timer(TimerError::Clock))
                    )
                    | (
                        "/schedule/timers/one/interval_ticks",
                        Err(SaveError::Timer(TimerError::Range))
                    )
                    | (
                        "/schedule/timers/one/payload",
                        Err(SaveError::Timer(TimerError::Limit))
                    )
            ),
            "{pointer}"
        );
    }
    assert_eq!(play.save_text().unwrap(), before);
}
