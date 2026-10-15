use super::*;
use std::{
    hint::black_box,
    sync::{Arc, Barrier},
};

fn work(source: [u8; 16]) {
    let _phase = crate::scope(Phase::ScriptTick);
    let _source = crate::script_scope(Some(source));
    for n in 0..100 {
        black_box(n * n);
    }
}
fn zone(report: &Report, phase: Phase) -> &Zone {
    report
        .zones
        .iter()
        .find(|z| z.name == phase.name())
        .unwrap()
}
#[test]
fn only_owner_tick_collects_real_phases_and_source_identities() {
    let mut capture = Capture::new().unwrap();
    work([1; 16]); // Same thread, outside the owning closure.
    capture
        .frame(7, || {
            work([2; 16]);
            std::thread::spawn(|| work([3; 16])).join().unwrap();
            Ok::<_, ()>(())
        })
        .unwrap()
        .unwrap();
    work([4; 16]);
    let report = capture.finish().unwrap().read(1).unwrap();
    assert_eq!(report.sources.len(), 1);
    assert_eq!(report.sources[0].source, Some([2; 16]));
    assert_eq!(report.frames[0].tick, 7);
    assert_eq!(zone(&report, Phase::Frame).totals.calls, 1);
    assert_eq!(zone(&report, Phase::ScriptTick).totals.calls, 1);
    assert!(report.frames[0].wall_ns.is_some());
    assert!(zone(&report, Phase::ScriptTick).p95_wall_ns.is_some());
    assert_eq!(zone(&report, Phase::Presentation).missing_frames, 1);
    assert!(zone(&report, Phase::Presentation).p95_wall_ns.is_none());
}
#[test]
fn simultaneous_thread_captures_do_not_share_sources_or_frames() {
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (1..=2)
        .map(|id| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut capture = Capture::new().unwrap();
                capture
                    .frame(u64::from(id), || {
                        barrier.wait();
                        work([id; 16]);
                        barrier.wait();
                        Ok::<_, ()>(())
                    })
                    .unwrap()
                    .unwrap();
                capture.finish().unwrap().read(1).unwrap()
            })
        })
        .collect();
    for (index, handle) in handles.into_iter().enumerate() {
        let report = handle.join().unwrap();
        assert_eq!(report.attempted_frames, 1);
        assert_eq!(report.sources.len(), 1);
        assert_eq!(report.sources[0].source, Some([index as u8 + 1; 16]));
    }
}
#[test]
fn bounded_ring_reads_exact_suffix_and_exposes_eviction() {
    let mut capture = Capture::new().unwrap();
    for tick in 1..=300 {
        capture.frame(tick, || Ok::<_, ()>(())).unwrap().unwrap();
    }
    let recording = capture.finish().unwrap();
    assert!(recording.read(0).is_err());
    assert!(recording.read(MAX_FRAMES + 1).is_err());
    let report = recording.read(MAX_FRAMES).unwrap();
    assert_eq!(report.evicted_frames, 44);
    assert_eq!(report.frames.first().unwrap().tick, 45);
    assert_eq!(report.frames.last().unwrap().tick, 300);
    assert_eq!(zone(&report, Phase::Frame).totals.calls, MAX_FRAMES as u64);
    let suffix = recording.read(2).unwrap();
    assert_eq!(
        suffix.frames.iter().map(|r| r.tick).collect::<Vec<_>>(),
        [299, 300]
    );
    assert_eq!(zone(&suffix, Phase::Frame).totals.calls, 2);
    assert!(
        size_of::<Row>() * recording.0.rows.capacity() + size_of::<Storage>() <= MAX_STORAGE_BYTES
    );
}
#[test]
fn nested_capture_refuses_before_running_and_does_not_replace_owner() {
    let mut outer = Capture::new().unwrap();
    let mut inner = Capture::new().unwrap();
    outer
        .frame(1, || {
            assert_eq!(
                inner.frame(8, || -> Result<(), ()> { panic!("must not execute") }),
                Err(Error::SessionBusy)
            );
            work([5; 16]);
            Ok::<_, ()>(())
        })
        .unwrap()
        .unwrap();
    assert!(inner.finish().unwrap().read(1).is_err());
    assert_eq!(
        outer.finish().unwrap().read(1).unwrap().sources[0].source,
        Some([5; 16])
    );
}
#[test]
fn failure_and_unwind_remain_invalid_and_retire_tls() {
    let mut capture = Capture::new().unwrap();
    assert_eq!(
        capture.frame(1, || Err::<(), _>("game error")).unwrap(),
        Err("game error")
    );
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = capture.frame(2, || -> Result<(), ()> {
            let _scope = crate::scope(Phase::ScriptTick);
            panic!("guest panic")
        });
    }));
    assert!(panic.is_err());
    capture
        .frame(3, || {
            work([6; 16]);
            Ok::<_, ()>(())
        })
        .unwrap()
        .unwrap();
    let recording = capture.finish().unwrap();
    let report = recording.read(3).unwrap();
    assert_eq!(report.failed_frames, 2);
    assert_eq!(zone(&report, Phase::ScriptTick).totals.unwound, 1);
    assert!(zone(&report, Phase::Frame).p95_wall_ns.is_none());
    assert_eq!(recording.read(1).unwrap().failed_frames, 0);
}
#[test]
fn escaped_scope_invalidates_its_frame_and_cannot_write_reused_ring_slot() {
    let mut capture = Capture::new().unwrap();
    let escaped = capture
        .frame(1, || Ok::<_, ()>(crate::scope(Phase::ScriptTick)))
        .unwrap()
        .unwrap();
    assert!(capture.0.borrow().rows[0].phases[Phase::ScriptTick as usize].invalid);
    for tick in 2..=257 {
        capture.frame(tick, || Ok::<_, ()>(())).unwrap().unwrap();
    }
    drop(escaped);
    let report = capture.finish().unwrap().read(1).unwrap();
    assert_eq!(report.frames[0].attempt, 257);
    assert_eq!(zone(&report, Phase::ScriptTick).totals.calls, 0);
    assert_eq!(zone(&report, Phase::ScriptTick).missing_frames, 1);
}
#[test]
fn unfinished_scope_is_visible_in_final_record_and_cannot_mutate_it_later() {
    let mut capture = Capture::new().unwrap();
    let scope = capture
        .frame(1, || Ok::<_, ()>(crate::scope(Phase::Physics3d)))
        .unwrap()
        .unwrap();
    let recording = capture.finish().unwrap();
    let before = recording.read(1).unwrap();
    assert_eq!(before.invalid_frames, 1);
    assert_eq!(before.frames[0].invalid_phases, 1);
    assert_eq!(zone(&before, Phase::Physics3d).invalid_frames, 1);
    assert!(zone(&before, Phase::Physics3d).p95_wall_ns.is_none());
    drop(scope);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(recording.read(1).unwrap()).unwrap()
    );
}
#[test]
fn backward_clock_and_overflow_are_not_valid_zero_samples() {
    let start = Instant::now();
    assert_eq!(duration(start + Duration::from_secs(1), start), None);
    let mut metric = Metric::default();
    metric.start();
    metric.end(None, None, false);
    assert!(metric.invalid);
    assert_eq!(metric.totals.unavailable_cpu_samples, 1);
    metric.totals.wall_ns = u64::MAX;
    metric.invalid = false;
    metric.start();
    metric.end(Some(1), Some(1), false);
    assert!(metric.invalid);
}
#[test]
fn source_exhaustion_is_reported_without_unbounded_storage() {
    let mut capture = Capture::new().unwrap();
    capture
        .frame(1, || {
            for id in 0..=MAX_SOURCES {
                work([id as u8; 16]);
            }
            Ok::<_, ()>(())
        })
        .unwrap()
        .unwrap();
    let report = capture.finish().unwrap().read(1).unwrap();
    assert_eq!(report.sources.len(), MAX_SOURCES);
    assert_eq!(report.dropped_source_samples, 1);
    assert_eq!(
        zone(&report, Phase::ScriptSource).totals.calls,
        MAX_SOURCES as u64 + 1
    );
}

#[test]
fn largest_recording_has_a_bounded_response_separate_from_retained_storage() {
    let mut capture = Capture::new().unwrap();
    for tick in 1..=MAX_FRAMES as u64 {
        capture.frame(tick, || Ok::<_, ()>(())).unwrap().unwrap();
    }
    let mut recording = capture.finish().unwrap();
    recording.0.sources = std::array::from_fn(|i| Some(Some([i as u8; 16])));
    for row in &mut recording.0.rows {
        for metric in row.phases.iter_mut().chain(row.sources.iter_mut()) {
            metric.totals = Totals {
                calls: u64::MAX,
                wall_ns: u64::MAX,
                thread_cpu_ns: u64::MAX,
                max_wall_ns: u64::MAX,
                unavailable_cpu_samples: u64::MAX,
                unwound: u64::MAX,
            };
            metric.invalid = true;
        }
        row.dropped_sources = u64::MAX;
    }
    let report = recording.read(MAX_FRAMES).unwrap();
    assert_eq!(report.invalid_frames, MAX_FRAMES);
    assert!(report.dropped_source_count_overflow);
    assert!(report.zones.iter().all(|zone| zone.p95_wall_ns.is_none()));
    // Leaves 64 KiB for the host's fixed identity/limitations envelope. The
    // runtime independently refuses any complete response above 256 KiB.
    assert!(serde_json::to_vec(&report).unwrap().len() < 192 * 1024);
}
