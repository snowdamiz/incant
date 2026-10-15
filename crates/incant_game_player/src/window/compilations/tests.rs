use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn scope(observer: &Observer, stats: &Rc<RefCell<Stats>>, phase: Phase, step: u64) -> Scope {
    let native = observer
        .scope(Context {
            session_generation: 1,
            surface_generation: 1,
            segment: 1,
            host_step: step,
            phase,
        })
        .unwrap();
    let index = phase_index(phase);
    stats.borrow_mut().scopes[index].started += 1;
    Scope {
        native: Some(native),
        stats: Rc::clone(stats),
        index,
        committed: false,
    }
}
#[test]
fn create_only_output_and_early_failure_preserve_truthful_unbound_status() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    {
        let _capture = Capture::new(path.to_str().unwrap(), 1, 1).unwrap();
        assert!(Capture::new(path.to_str().unwrap(), 1, 1).is_err());
    }
    let bytes = std::fs::read(&path).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["snapshot"], serde_json::Value::Null);
    assert_eq!(
        report["finalization"]["stage"],
        "abandoned_before_finalization"
    );
    assert_eq!(report["shader_stall_gate"], false);
    assert!(Capture::new(path.to_str().unwrap(), 1, 1).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
#[test]
fn invalid_capture_bounds_have_no_file_effect() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    for (capacity, steps) in [(0, 1), (4097, 1), (1, 0), (1, 36001)] {
        assert!(Capture::new(path.to_str().unwrap(), capacity, steps).is_err());
        assert!(!path.exists());
    }
}
#[test]
fn only_explicit_commit_marks_success_and_unwind_clears_native_scope() {
    let observer = Observer::default();
    observer.start(1).unwrap();
    let stats = Rc::new(RefCell::new(Stats::default()));
    {
        let mut first = scope(&observer, &stats, Phase::Gameplay, 1);
        first.commit();
        first.commit();
    }
    assert!(!observer.snapshot().scope_active);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _failed = scope(&observer, &stats, Phase::Gameplay, 2);
        panic!("host failure");
    }));
    assert!(result.is_err());
    assert!(!observer.snapshot().scope_active);
    let c = stats.borrow().scopes[phase_index(Phase::Gameplay)];
    assert_eq!((c.started, c.committed, c.abandoned), (2, 1, 1));
}
#[test]
fn boundary_overflow_is_visible_and_refuses_further_scopes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    let mut capture = Capture::new(path.to_str().unwrap(), 1, 1).unwrap();
    capture.boundary(Boundary::Focus);
    assert_eq!(capture.segment.get(), 2);
    capture.segment.set(u64::MAX);
    capture.boundary(Boundary::Focus);
    assert!(capture.boundary_overflow.get());
    assert!(capture.scope(Phase::Gameplay, 1, 1, 1).is_err());
    let result: session::Result<serde_json::Value> = Err("original host failure".into());
    capture.finish(0, 1, 1, &result).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(report["segment_overflow"], true);
    assert_eq!(
        report["finalization"]["window_error"],
        "original host failure"
    );
    assert_eq!(report["finalization"]["returned_report"], false);
}
#[test]
fn output_limit_and_counter_saturation_remain_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let file = File::create(dir.path().join("limited")).unwrap();
    let mut writer = Limited { file, remaining: 3 };
    writer.write_all(b"abc").unwrap();
    assert!(writer.write_all(b"d").is_err());
    assert_eq!(std::fs::read(dir.path().join("limited")).unwrap(), b"abc");
    let mut n = u64::MAX;
    let mut saturated = false;
    bump(&mut n, &mut saturated);
    assert_eq!(n, u64::MAX);
    assert!(saturated);
}
#[test]
fn absent_capture_is_inert_and_snapshot_never_claims_driver_or_display_gate() {
    assert!(enter(&None, Phase::Gameplay, 1, 1, 1).unwrap().is_none());
    let observer = Observer::default();
    observer.start(1).unwrap();
    let snapshot = snapshot_json(&observer);
    assert_eq!(snapshot["counts"][0]["attempts"], 0);
    assert_eq!(snapshot["capture_started"], true);
    assert_eq!(snapshot["rows"], serde_json::json!([]));
    assert_eq!(
        snapshot["clock_anomalies"],
        serde_json::json!({"entry":0,"completion":0,"snapshot":0})
    );
}

// Same qualified collector bytes; no upstream HAL dev/example graph required.
#[allow(
    dead_code,
    reason = "The pure collector does not call native creation APIs"
)]
#[path = "../../../../../third_party/wgpu-hal/src/metal/compilation.rs"]
mod collector_model;

#[test]
fn final_sidecar_failure_preserves_original_error_and_refuses_false_success() {
    let original: session::Result<u32> = Err("game failed".into());
    let result = preserve_result(original, Err("sidecar failed".into()));
    assert_eq!(result.unwrap_err().to_string(), "game failed");
    assert_eq!(
        preserve_result(Ok(7), Err("sidecar failed".into()))
            .unwrap_err()
            .to_string(),
        "sidecar failed"
    );
    assert_eq!(preserve_result(Ok(7), Ok(())).unwrap(), 7);
}

#[test]
fn reservation_boundaries_keep_defer_error_and_surface_changes_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.json");
    let capture = Capture::new(path.to_str().unwrap(), 1, 1).unwrap();
    capture.reservation([0, 0, 0], [0, 0, 0], Some(true));
    assert_eq!(capture.segment.get(), 1);
    capture.reservation([0, 0, 0], [1, 0, 1], Some(false));
    capture.reservation([1, 0, 1], [1, 1, 1], None);
    assert_eq!(capture.segment.get(), 5);
    let counts = capture.boundaries.get();
    assert_eq!(&counts[5..], &[1, 1, 1, 1]);
    assert_eq!(capture.surface_counters.get(), Some([1, 1, 1]));
}
