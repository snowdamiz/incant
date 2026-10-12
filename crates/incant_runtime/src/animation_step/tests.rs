use super::*;
use crate::*;
use incant_types::{Animation, PoseInterpolation};
use std::sync::Arc;

fn clip() -> Playback {
    let definition = Arc::new(NativeAnimation {
        mesh_fingerprint: [1; 32],
        clip_durations: Arc::from([2.0, 4.0]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.0,
            speed: 1.0,
            playing: true,
            looping: true,
        },
        graph: None,
    });
    Playback {
        state: definition.initial_state(),
        definition,
        graph: None,
    }
}
fn graph() -> Playback {
    let id = |n: u8| StableId([n; 16]);
    let mut playback = clip();
    let definition = Arc::make_mut(&mut playback.definition);
    let graph = NativeAnimationGraph::new(
        0,
        vec![NativeAnimationParameter {
            id: id(9),
            initial: 0.25,
            min: 0.0,
            max: 1.0,
        }],
        vec![
            NativeAnimationGraphState {
                id: id(1),
                motion: NativeAnimationMotion::Clip(Some(0)),
                looping: true,
            },
            NativeAnimationGraphState {
                id: id(2),
                motion: NativeAnimationMotion::Blend1d {
                    parameter: 0,
                    cycle_seconds: 4.0,
                    points: Arc::from([
                        NativeAnimationBlendPoint {
                            threshold: 0.0,
                            clip: 0,
                        },
                        NativeAnimationBlendPoint {
                            threshold: 1.0,
                            clip: 1,
                        },
                    ]),
                },
                looping: true,
            },
        ],
        vec![NativeAnimationGraphTransition {
            id: id(3),
            from: 0,
            to: 1,
            priority: 0,
            fade_seconds: 0.3,
            after_seconds: Some(0.1),
            conditions: Arc::from([]),
        }],
        &[2.0, 4.0],
    )
    .unwrap();
    definition.graph = Some(Arc::new(graph));
    playback.graph = Some(definition.graph.as_ref().unwrap().initial_state(true));
    playback
}
fn publish(playback: &mut Playback, next: &PreparedAnimationStep) {
    playback.state = next.state;
    playback.graph = next.graph;
}

#[test]
fn exact_loop_receipts_match_accepted_clock_without_float_integer_rounding() {
    for (before, speed, dt, expected) in [
        (1.75, 1.0, 0.5, 1),
        (0.25, -1.0, 0.5, -1),
        (0.0, 1.0, 2.0, 1),
        (0.0, -1.0, 2.0, -1),
        (0.0, 1.0, 12.5, 6),
        (0.0, -1.0, 12.5, -7),
        (2.0, 0.0, 1.0, 0),
        (2.0, 1.0, 0.5, 1),
    ] {
        let (after, wraps, _) = clock::advance(before, 2.0, speed, true, true, dt).unwrap();
        let mut native = before;
        crate::animation::advance_clock(&mut native, 2.0, speed, true, dt);
        assert_eq!(after.to_bits(), native.to_bits());
        assert_eq!(wraps, expected);
    }
    // Exact rational quotient from the binary f64 values is 578685596,
    // whereas floating division rounds it to the next integer.
    let next: f64 = 1.255212948491412e-144;
    let duration: f64 = 2.1690758418710256e-153;
    assert_eq!((next / duration).floor() as u64, 578_685_597);
    assert_eq!(
        clock::advance(0.0, duration, 1.0, true, true, next)
            .unwrap()
            .1,
        578_685_596
    );
    assert_eq!(
        clock::advance(0.0, duration, -1.0, true, true, next)
            .unwrap()
            .1,
        -578_685_597
    );
    // Native rem_euclid rounds this near-zero negative position to duration,
    // then canonicalizes its phase to zero. It must not create a cycle jump.
    assert_eq!(
        clock::advance(0.0, 2.0, -1.0, true, true, f64::from_bits(1)).unwrap(),
        (0.0, 0, false)
    );
}

#[test]
fn bounds_pause_and_nonlooping_completion_are_exact() {
    for sign in [-1.0, 1.0] {
        let (_, wraps, _) =
            clock::advance(0.0, 1.0, sign, true, true, (1u64 << 32) as f64).unwrap();
        assert_eq!(wraps, sign as i64 * (1i64 << 32));
        assert!(clock::advance(0.0, 1.0, sign, true, true, ((1u64 << 32) + 1) as f64).is_err());
    }
    assert!(clock::advance(0.0, f64::from_bits(1), 64.0, true, true, 0.125).is_err());
    assert_eq!(
        clock::advance(0.25, 2.0, -1.0, false, true, 1.0).unwrap(),
        (0.0, 0, true)
    );
    assert_eq!(
        clock::advance(1.75, 2.0, 1.0, false, true, 1.0).unwrap(),
        (2.0, 0, true)
    );
    assert_eq!(
        clock::advance(1.75, 2.0, 1.0, false, false, 1.0).unwrap(),
        (1.75, 0, false)
    );
    for bad in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        assert!(clock::advance(0.0, 2.0, 1.0, true, true, bad).is_err());
    }
}

#[test]
fn automatic_edge_owns_the_pre_edge_interval_and_compound_clocks_remain_independent() {
    let mut p = graph();
    let first = PreparedAnimationStep::new(&p, 0.125).unwrap();
    assert_eq!(first.current.owner, ClockOwner::State(0));
    assert_eq!(first.current.after, 0.125);
    assert!(first.outgoing.is_none());
    assert_eq!(first.graph.unwrap().current, 1);
    assert_eq!(p.graph.unwrap().current, 0); // Preparation has no side effect.
    publish(&mut p, &first);
    p.state.speed = -2.0;
    let second = PreparedAnimationStep::new(&p, 0.125).unwrap();
    assert_eq!(second.current.owner, ClockOwner::State(1));
    assert_eq!(second.current.after, 3.75);
    assert_eq!(second.current.wraps, -1);
    let source = second.outgoing.unwrap();
    assert_eq!(source.owner, ClockOwner::State(0));
    assert_eq!(source.before, 0.125);
    assert_eq!(source.after, 0.25);
    let group = second.current.after_pose.group(0).unwrap();
    assert_eq!(group.interpolation, PoseInterpolation::SphericalPair);
    assert_eq!(group.samples[0].sample.seconds, 1.875);
    assert_eq!(group.samples[1].sample.seconds, 3.75);
    assert_eq!(group.samples[1].weight, 0.25);
    assert!((second.fade.unwrap().mean_weight - 0.0625 / 0.3).abs() < 1e-15);
    publish(&mut p, &second);
    p.graph.as_mut().unwrap().paused = true;
    p.state.playing = false;
    let paused = PreparedAnimationStep::new(&p, 0.125).unwrap();
    assert_eq!(paused.state, p.state);
    assert_eq!(paused.graph, p.graph);
    assert_eq!(paused.current.before, paused.current.after);
    assert_eq!(
        paused.outgoing.unwrap().before,
        paused.outgoing.unwrap().after
    );
    assert_eq!(paused.current.wraps, 0);
}

#[test]
fn linear_fade_mean_includes_saturated_tail_and_refusal_preserves_every_candidate() {
    let fade = FadeSpan::new(0.25, 0.3, true, 0.125).unwrap();
    let expected = ((0.25 / 0.3 + 1.0) * 0.5 * 0.05 + 0.075) / 0.125;
    assert!((fade.mean_weight - expected).abs() < 1e-15);
    let mut p = clip();
    Arc::make_mut(&mut p.definition).clip_durations = Arc::from([f32::from_bits(1), 4.0]);
    let old = p.state;
    assert!(PreparedAnimationStep::new(&p, 0.125).is_err());
    assert_eq!(p.state, old);
    assert!(p.graph.is_none());
    p.state.playing = false;
    assert!(PreparedAnimationStep::new(&p, 0.125).is_ok());
}
