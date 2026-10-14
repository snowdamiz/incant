#![cfg(not(feature = "summary"))]
use incant_profile::*;
#[test]
fn default_has_no_capture_clock_or_dynamic_state() {
    assert_eq!(std::mem::size_of::<Scope>(), 0);
    assert!(!enabled());
    assert_eq!(frame(|| Ok::<_, ()>(17)), Ok(17));
    assert_eq!(frame(|| Err::<(), _>("unchanged")), Err("unchanged"));
    for _ in 0..1000 {
        let _zone = scope(Phase::Frame);
        let _script = script_scope(Some([8; 16]));
    }
    assert!(matches!(Capture::start(), Err(Error::Disabled)));
    assert!(matches!(
        Capture::start_tracy_process(),
        Err(Error::TracyUnavailable)
    ));
}

#[test]
fn heap_observation_and_detail_mode_stay_disabled() {
    assert!(incant_profile::quickjs_observation().is_none());
    assert!(matches!(
        incant_profile::Capture::start_with_heap_mode(incant_profile::HeapMode::Detailed),
        Err(incant_profile::Error::Disabled)
    ));
}
