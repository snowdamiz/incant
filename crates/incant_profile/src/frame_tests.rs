use super::*;
fn sample(f: &mut Frames, ns: u64) {
    let id = f.begin();
    let scope = f.begin_scope(Phase::ScriptTick);
    f.end_scope(scope, Phase::ScriptTick, ns, false);
    f.finish(id, true);
}
fn script(f: &Frames) -> FramePhaseReport {
    f.report().phases.remove(Phase::ScriptTick as usize)
}
#[test]
fn nearest_rank_intervals_distinguish_two_ms_boundary_without_rounding_down() {
    let mut f = Frames::allocated().unwrap();
    for _ in 0..94 {
        sample(&mut f, 1);
    }
    sample(&mut f, 2_000_000);
    for _ in 0..5 {
        sample(&mut f, 2_000_001);
    }
    let r = script(&f);
    assert_eq!(r.samples, 100);
    assert_eq!(
        (r.p95_lower_ns, r.p95_upper_ns),
        (Some(1_990_001), Some(2_000_000))
    );
    sample(&mut f, 2_000_001); // ceil(101*.95) is96, now above the budget.
    let r = script(&f);
    assert_eq!(
        (r.p95_lower_ns, r.p95_upper_ns),
        (Some(2_000_001), Some(2_010_000))
    );
    assert_eq!(r.counts.iter().sum::<u64>(), 101);
}
#[test]
fn zero_missing_overflow_failure_and_active_frames_do_not_invent_quantiles() {
    let mut f = Frames::allocated().unwrap();
    assert_eq!(script(&f).p95_upper_ns, None);
    sample(&mut f, 0);
    assert_eq!(
        (script(&f).p95_lower_ns, script(&f).p95_upper_ns),
        (Some(0), Some(0))
    );
    let id = f.begin();
    assert_eq!(script(&f).p95_upper_ns, None);
    f.finish(id, true);
    assert_eq!(script(&f).missing_frames, 1);
    assert_eq!(script(&f).p95_upper_ns, None);
    let mut f = Frames::allocated().unwrap();
    sample(&mut f, FRAME_MAX_NS + 1);
    assert_eq!(script(&f).overflow_samples, 1);
    assert_eq!(script(&f).p95_upper_ns, None);
    let mut f = Frames::allocated().unwrap();
    sample(&mut f, FRAME_MAX_NS);
    assert_eq!(script(&f).p95_upper_ns, Some(FRAME_MAX_NS));
    let id = f.begin();
    f.finish(id, false);
    assert_eq!(script(&f).p95_upper_ns, None);
    assert_eq!(f.report().failed_frames, 1);
}
#[test]
fn sequential_phase_cost_is_one_sample_and_nested_sources_are_not_added() {
    let mut f = Frames::allocated().unwrap();
    let frame = f.begin();
    for _ in 0..2 {
        let tick = f.begin_scope(Phase::ScriptTick);
        for _ in 0..4 {
            let source = f.begin_scope(Phase::ScriptSource);
            f.end_scope(source, Phase::ScriptSource, 50_000, false);
        }
        f.end_scope(tick, Phase::ScriptTick, 500_000, false);
    }
    f.finish(frame, true);
    assert_eq!(script(&f).samples, 1);
    assert_eq!(script(&f).p95_upper_ns, Some(1_000_000));
    assert_eq!(
        f.report().phases[Phase::ScriptSource as usize].p95_upper_ns,
        Some(400_000)
    );
}
#[test]
fn unfinished_recursive_and_preexisting_scopes_refuse_instead_of_undercounting() {
    for kind in 0..3 {
        let mut f = Frames::allocated().unwrap();
        let pre = (kind == 2).then(|| f.begin_scope(Phase::ScriptTick));
        let frame = f.begin();
        let a = f.begin_scope(Phase::ScriptTick);
        if kind == 1 {
            let b = f.begin_scope(Phase::ScriptTick);
            f.end_scope(b, Phase::ScriptTick, 10, false);
        }
        if kind != 0 {
            f.end_scope(a, Phase::ScriptTick, 10, false);
        }
        f.finish(frame, true);
        assert_eq!(script(&f).invalid_frames, 1);
        assert_eq!(script(&f).p95_upper_ns, None);
        if let Some(pre) = pre {
            f.end_scope(pre, Phase::ScriptTick, 10, false);
        }
    }
}
#[test]
fn concurrency_and_saturation_fail_closed() {
    let mut f = Frames::allocated().unwrap();
    let frame = f.begin();
    std::thread::scope(|s| {
        s.spawn(|| {
            let tick = f.begin_scope(Phase::ScriptTick);
            assert_eq!(tick, None);
            f.end_scope(tick, Phase::ScriptTick, 1, false);
        })
        .join()
        .unwrap();
    });
    f.finish(frame, true);
    assert_eq!(script(&f).invalid_frames, 1);
    let mut f = Frames::allocated().unwrap();
    let a = f.begin();
    let b = f.begin();
    f.finish(b, true);
    f.finish(a, true);
    assert_eq!(f.report().invalid_frames, 2);
    let mut f = Frames::allocated().unwrap();
    f.rows[Phase::ScriptTick as usize].samples = u64::MAX;
    sample(&mut f, 7);
    assert!(f.report().saturated);
    assert_eq!(script(&f).p95_upper_ns, None);
    assert_eq!(percentile(&[u64::MAX], u64::MAX), Some((0, 0)));
}
