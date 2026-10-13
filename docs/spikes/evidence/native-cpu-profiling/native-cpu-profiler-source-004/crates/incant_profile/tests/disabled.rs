#![cfg(not(feature = "summary"))]
use incant_profile::*;
#[test]
fn default_has_no_capture_clock_or_dynamic_state() {
    assert_eq!(std::mem::size_of::<Scope>(), 0);
    assert!(!enabled());
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
