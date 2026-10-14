#![cfg(feature = "summary")]
use incant_profile::*;
use std::sync::Mutex;
#[global_allocator]
static ALLOCATOR: CountingAllocator<std::alloc::System> = CountingAllocator(std::alloc::System);
static SERIAL: Mutex<()> = Mutex::new(());
fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}
fn count(report: &Report, name: &str) -> u64 {
    report
        .zones
        .iter()
        .find(|z| z.name == name)
        .map_or(0, |z| z.totals.calls)
}
#[test]
fn nested_thread_zones_and_failure_cleanup_are_observable() {
    let _lock = lock();
    let capture = Capture::start().unwrap();
    assert!(matches!(Capture::start(), Err(Error::Busy)));
    {
        let _outer = scope(Phase::Frame);
        {
            let _inner = scope(Phase::Input);
            std::hint::black_box((0..1000).sum::<u64>());
        }
    }
    std::thread::spawn(|| {
        let _zone = scope(Phase::Physics3d);
    })
    .join()
    .unwrap();
    assert!(
        std::panic::catch_unwind(|| {
            let _zone = scope(Phase::AudioSync);
            panic!("test unwind");
        })
        .is_err()
    );
    frame_outcome(true);
    frame_outcome(false);
    let report = capture.report();
    for name in [
        "player.frame",
        "native.input",
        "physics.3d",
        "game.audio_sync",
    ] {
        assert_eq!(count(&report, name), 1);
    }
    assert_eq!(
        report
            .zones
            .iter()
            .find(|z| z.name == "game.audio_sync")
            .unwrap()
            .totals
            .unwound,
        1
    );
    assert_eq!((report.completed_frames, report.failed_frames), (1, 1));
    assert!(
        report
            .zones
            .iter()
            .all(|z| z.totals.unavailable_cpu_samples == 0)
    );
    drop(capture);
    assert!(!enabled());
    let capture = Capture::start().unwrap();
    assert!(capture.report().zones.is_empty());
}
#[test]
fn stale_scope_and_old_worker_cannot_write_into_replacement_capture() {
    let _lock = lock();
    let first = Capture::start().unwrap();
    let stale = scope(Phase::WorldStep);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (drop_tx, drop_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _zone = scope(Phase::Physics2d);
        ready_tx.send(()).unwrap();
        drop_rx.recv().unwrap();
    });
    ready_rx.recv().unwrap();
    drop(first);
    let next = Capture::start().unwrap();
    drop(stale);
    drop_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(next.report().zones.is_empty());
    {
        let _zone = scope(Phase::WorldStep);
    }
    assert_eq!(count(&next.report(), "world.step"), 1);
}
#[test]
fn numeric_source_bound_drops_excess_without_unbounded_registry_or_hot_allocations() {
    let _lock = lock();
    let capture = Capture::start().unwrap();
    for n in 0..MAX_SOURCES + 10 {
        let _zone = script_scope(Some((n as u128).to_be_bytes()));
    }
    for _ in 0..1000 {
        let _zone = script_scope(Some(0_u128.to_be_bytes()));
    }
    let report = capture.report();
    assert_eq!(report.scripts.len(), MAX_SOURCES);
    assert_eq!(report.dropped_source_samples, 10);
    assert_eq!(count(&report, "script.source"), 1074);
    assert_eq!(report.scripts[0].totals.calls, 1001);
}
#[test]
fn heap_metrics_are_vm_samples_and_rust_requested_sizes_are_separate() {
    let _lock = lock();
    let capture = Capture::start().unwrap();
    quickjs_heap(HeapSample {
        used_bytes: 17,
        allocated_bytes: 32,
        allocation_count: 3,
    });
    quickjs_heap(HeapSample {
        used_bytes: 12,
        allocated_bytes: 24,
        allocation_count: 2,
    });
    let report = capture.report();
    assert_eq!(report.quickjs_heap.samples, 2);
    assert_eq!(report.quickjs_heap.latest.used_bytes, 12);
    assert_eq!(report.quickjs_heap.peak_used_bytes, 17);
    assert!(report.rust_allocations.installed);
}

#[test]
fn guarded_frames_track_success_failure_unwind_and_capture_generation() {
    let _lock = lock();
    let capture = Capture::start().unwrap();
    frame(|| {
        let _tick = scope(Phase::ScriptTick);
        Ok::<_, ()>(())
    })
    .unwrap();
    let report = capture.report();
    assert_eq!(report.version, 2);
    assert_eq!(report.frame_wall.successful_frames, 1);
    assert_eq!(
        report.frame_wall.phases[Phase::ScriptTick as usize].samples,
        1
    );
    assert!(
        report.frame_wall.phases[Phase::ScriptTick as usize]
            .p95_upper_ns
            .is_some()
    );
    assert!(
        std::panic::catch_unwind(|| frame(|| -> Result<(), ()> { panic!("frame unwind") }))
            .is_err()
    );
    let report = capture.report();
    assert_eq!(report.failed_frames, 1);
    assert_eq!(report.frame_wall.failed_frames, 1);
    assert!(
        report
            .frame_wall
            .phases
            .iter()
            .all(|r| r.p95_upper_ns.is_none())
    );
    let mut replacement = None;
    frame(|| {
        drop(capture);
        replacement = Some(Capture::start().unwrap());
        Ok::<_, ()>(())
    })
    .unwrap();
    let report = replacement.unwrap().report();
    assert_eq!(report.frame_wall.attempted_frames, 0);
    assert_eq!(report.completed_frames, 0);
}
