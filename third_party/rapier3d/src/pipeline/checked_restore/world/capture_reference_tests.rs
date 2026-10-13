// The old serde-based one-way observer is independent of every new direct
// record extractor. Both only observe these harness-owned actual native steps.
#[test]
fn direct_capture_matches_independent_full_native_observer_during_churn() {
    use crate::pipeline::checked_restore::{
        AdmissionError,
        capture::{CaptureError, observe_for_reference},
    };
    let (mut admitted, mut events) = (0, 0);
    let mut refused = std::collections::BTreeSet::new();
    let mut mass_refused = std::collections::BTreeSet::new();
    narrow_run(|tick, w, _, _, rows| {
        events += rows.len();
        match capture_gate(w) {
            Ok(()) => {
                let expected = with_world(w, |v| {
                    admit_numeric_frame(v, identity(), &mut WorkBudget::new(u64::MAX)).map(|_| ())
                });
                let direct = observe_for_reference(w, identity());
                match expected {
                    Ok(()) => {
                        assert_eq!(
                            world_bytes(w),
                            direct.unwrap(),
                            "all 22 sections tick {tick}"
                        );
                        admitted += 1;
                    }
                    Err(AdmissionError::LocalMass) => {
                        // Native body disable retains historic mass while attached
                        // colliders become disabled. The numeric domain deliberately
                        // refuses this, independently of either capture mapping.
                        assert_eq!(
                            direct,
                            Err(CaptureError::Numeric(AdmissionError::LocalMass))
                        );
                        mass_refused.insert(tick);
                    }
                    other => panic!("unexpected independent numeric outcome at {tick}: {other:?}"),
                }
            }
            Err(CaptureRefusal::PendingMutation) => {
                assert_eq!(
                    observe_for_reference(w, identity()),
                    Err(CaptureError::Record(Error::PendingMutation))
                );
                refused.insert(tick);
            }
            other => panic!("unexpected observer gate at tick {tick}: {other:?}"),
        }
    });
    assert_eq!(admitted, 60);
    assert_eq!(refused, [7].into_iter().collect());
    assert_eq!(mass_refused, [21, 22, 23].into_iter().collect());
    assert!(events > 0);
    eprintln!(
        "DIRECT_CAPTURE_REFERENCE dimension={} admitted={admitted} journal_refused={refused:?} mass_refused={mass_refused:?} complete_event_records={events}",
        crate::math::DIM
    );
}
