#[path = "skeletal/alloc.rs"]
mod alloc;
#[path = "root_motion/fixture.rs"]
mod fixture;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;
use glam::{DMat4, DQuat, DVec3};
use incant_assets_runtime::animation::{
    AnimationPlayer, MAX_ROOT_MOTION_CYCLES, MAX_ROOT_MOTION_DURATION, RootMotionDelta,
    RootMotionError, RootMotionSampler, RootMotionTime,
};

fn time(cycle: i64, seconds: f64) -> RootMotionTime {
    RootMotionTime { cycle, seconds }
}
fn matrix(delta: RootMotionDelta) -> DMat4 {
    DMat4::from_rotation_translation(
        DQuat::from_array(delta.rotation),
        DVec3::from_array(delta.translation),
    )
}
fn near(actual: DMat4, expected: DMat4) {
    assert!(
        actual.abs_diff_eq(expected, 2e-6),
        "{actual:?} != {expected:?}"
    );
}
fn rigid(x: f64, y: f64, angle: f64) -> DMat4 {
    DMat4::from_rotation_translation(DQuat::from_rotation_z(angle), DVec3::new(x, y, 0.))
}

#[test]
fn captured_linear_tracks_clamp_reverse_and_match_existing_sampler_arithmetic() {
    let model = fixture::cooked(|_, _| {});
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    assert_eq!(root.duration(), 2.);
    for (from, to, x) in [(0.5, 1.5, 1.), (1.5, 0.5, -1.), (-10., 10., 2.)] {
        near(
            matrix(root.clamped_delta(from, to).unwrap()),
            rigid(x, 0., 0.),
        );
    }
    assert_eq!(
        root.clamped_delta(3., f64::MAX).unwrap(),
        RootMotionDelta::IDENTITY
    );
    let rest = RootMotionSampler::new(&model, 0, 0).unwrap();
    assert_eq!(
        rest.clamped_delta(0., 2.).unwrap(),
        RootMotionDelta::IDENTITY
    );
    let mut player = AnimationPlayer::new(&model).unwrap();
    for clip in [0, 1] {
        let node = clip + 1;
        let root = RootMotionSampler::new(&model, node, clip).unwrap();
        for (from, to) in [(0., 1.234567), (1.7, 0.125), (-3., 6.)] {
            player.sample(Some(clip), from).unwrap();
            let a = player.locals()[node];
            player.sample(Some(clip), to).unwrap();
            near(
                matrix(root.clamped_delta(from, to).unwrap()),
                a.inverse() * player.locals()[node],
            );
        }
    }
}

#[test]
fn turning_cycles_rotate_translation_and_reverse_is_the_rigid_inverse() {
    let model = fixture::cooked(fixture::turn);
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    for (cycles, x, y, turns) in [
        (1, 2., 0., 1.),
        (2, 2., 2., 2.),
        (3, 0., 2., 3.),
        (4, 0., 0., 4.),
        (-1, 0., 2., -1.),
    ] {
        near(
            matrix(root.loop_delta(time(0, 0.), time(cycles, 0.)).unwrap()),
            rigid(x, y, turns * std::f64::consts::FRAC_PI_2),
        );
    }
    near(
        matrix(root.loop_delta(time(0, 1.), time(1, 1.)).unwrap()),
        rigid(2f64.sqrt(), 0., std::f64::consts::FRAC_PI_2),
    );
    let forward = matrix(root.loop_delta(time(-3, 0.25), time(4, 1.75)).unwrap());
    let reverse = matrix(root.loop_delta(time(4, 1.75), time(-3, 0.25)).unwrap());
    near(forward * reverse, DMat4::IDENTITY);
    let large = root
        .loop_delta(time(i64::MAX - 2, 0.), time(i64::MAX, 0.))
        .unwrap();
    assert_eq!(large, root.loop_delta(time(0, 0.), time(2, 0.)).unwrap());
}

#[test]
fn intervals_match_an_independent_repeated_cycle_matrix_oracle() {
    let model = fixture::cooked(fixture::turn);
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    let cycle = rigid(2., 0., std::f64::consts::FRAC_PI_2);
    let absolute = |n: i32, phase: f64| {
        let step = if n < 0 { cycle.inverse() } else { cycle };
        let mut value = DMat4::IDENTITY;
        for _ in 0..n.unsigned_abs() {
            value *= step;
        }
        value * rigid(phase, 0., phase * std::f64::consts::FRAC_PI_4)
    };
    for a in -4..=4 {
        for b in -4..=4 {
            for (from, to) in [(0., 0.25), (1.75, 0.5), (0.25, 1.75)] {
                near(
                    matrix(
                        root.loop_delta(time(i64::from(a), from), time(i64::from(b), to))
                            .unwrap(),
                    ),
                    absolute(a, from).inverse() * absolute(b, to),
                );
            }
        }
    }
}

#[test]
fn step_discontinuity_and_missing_channels_keep_canonical_endpoint_rules() {
    let model = fixture::cooked(|doc, _| {
        doc["animations"][0]["samplers"][0]["interpolation"] = "STEP".into()
    });
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    assert_eq!(
        root.clamped_delta(0., 1.999).unwrap(),
        RootMotionDelta::IDENTITY
    );
    near(
        matrix(root.clamped_delta(1.999, 2.).unwrap()),
        rigid(2., 0., 0.),
    );
    near(
        matrix(root.loop_delta(time(0, 1.9), time(1, 0.1)).unwrap()),
        rigid(2., 0., 0.),
    );
    fixture::cubic_refusal();
}

#[test]
fn quaternion_signs_and_angle_wrap_preserve_shortest_arc_motion() {
    let mut results = Vec::new();
    for sign in [1., -1.] {
        let model = fixture::cooked(|doc, bytes| {
            let a = DQuat::from_rotation_z(170f64.to_radians())
                .to_array()
                .map(|v| v as f32);
            let b = DQuat::from_rotation_z((-170f64).to_radians())
                .to_array()
                .map(|v| v as f32 * sign);
            fixture::rotation_keys(doc, bytes, [a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]]);
        });
        let root = RootMotionSampler::new(&model, 2, 1).unwrap();
        let delta = matrix(root.clamped_delta(0., 2.).unwrap());
        near(delta, rigid(0., 0., 20f64.to_radians()));
        near(
            matrix(root.loop_delta(time(-1, 1.), time(1, 1.)).unwrap()),
            rigid(0., 0., 40f64.to_radians()),
        );
        results.push(delta);
    }
    near(results[0], results[1]);
}

#[test]
fn exact_cycle_bounds_and_invalid_times_refuse_without_affecting_later_reads() {
    let model = fixture::cooked(|_, _| {});
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    let max = MAX_ROOT_MOTION_CYCLES as i64;
    near(
        matrix(root.loop_delta(time(0, 0.), time(max, 0.)).unwrap()),
        rigid(2. * max as f64, 0., 0.),
    );
    for (a, b) in [
        (0, max + 1),
        (max + 1, 0),
        (i64::MIN, i64::MAX),
        (i64::MAX, i64::MIN),
    ] {
        assert!(matches!(
            root.loop_delta(time(a, 0.), time(b, 0.)),
            Err(RootMotionError::Cycles)
        ));
    }
    for phase in [-1., 2., f64::MAX] {
        assert!(matches!(
            root.loop_delta(time(0, phase), time(0, 0.)),
            Err(RootMotionError::Phase)
        ));
        assert!(matches!(
            root.loop_delta(time(0, 0.), time(0, phase)),
            Err(RootMotionError::Phase)
        ));
    }
    for seconds in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            root.clamped_delta(seconds, 0.),
            Err(RootMotionError::Time)
        ));
        assert!(matches!(
            root.clamped_delta(0., seconds),
            Err(RootMotionError::Time)
        ));
        assert!(matches!(
            root.loop_delta(time(0, 0.), time(0, seconds)),
            Err(RootMotionError::Time)
        ));
    }
    near(
        matrix(root.clamped_delta(0., 2.).unwrap()),
        rigid(2., 0., 0.),
    );
}

#[test]
fn constructor_checks_duration_scales_static_nodes_indices_and_cooked_data() {
    let mut model = fixture::cooked(|_, _| {});
    assert!(matches!(
        RootMotionSampler::new(&model, usize::MAX, 0),
        Err(RootMotionError::Node)
    ));
    assert!(matches!(
        RootMotionSampler::new(&model, 1, usize::MAX),
        Err(RootMotionError::Clip)
    ));
    assert!(matches!(
        RootMotionSampler::new(&model, 1, 2),
        Err(RootMotionError::Scale)
    ));
    assert!(matches!(
        RootMotionSampler::new(&model, 4, 0),
        Err(RootMotionError::Scale)
    ));
    model.animation.as_mut().unwrap().rest[3] = None;
    assert!(matches!(
        RootMotionSampler::new(&model, 3, 0),
        Err(RootMotionError::StaticMatrix)
    ));
    let channel = &mut model.animation.as_mut().unwrap().clips[0].channels[0];
    channel.times[1] = MAX_ROOT_MOTION_DURATION as f32;
    assert_eq!(
        RootMotionSampler::new(&model, 1, 0).unwrap().duration(),
        MAX_ROOT_MOTION_DURATION
    );
    model.animation.as_mut().unwrap().clips[0].channels[0].times[1] += 1.;
    assert!(matches!(
        RootMotionSampler::new(&model, 1, 0),
        Err(RootMotionError::Duration)
    ));
    model.animation.as_mut().unwrap().clips[0].channels[0].values[1][0] = f32::INFINITY;
    assert!(matches!(
        RootMotionSampler::new(&model, 1, 0),
        Err(RootMotionError::Model(_))
    ));
    model.animation = None;
    assert!(matches!(
        RootMotionSampler::new(&model, 1, 0),
        Err(RootMotionError::MissingAnimation)
    ));
}

#[test]
fn zero_duration_clips_are_static_and_whole_clip_duration_includes_other_nodes() {
    let mut model = fixture::cooked(|_, _| {});
    let other = model.animation.as_ref().unwrap().clips[1].channels[0].clone();
    model.animation.as_mut().unwrap().clips[0]
        .channels
        .push(other);
    model.animation.as_mut().unwrap().clips[0].channels[1].times[1] = 4.;
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    assert_eq!(root.duration(), 4.);
    assert_eq!(
        root.loop_delta(time(0, 3.), time(1, 0.)).unwrap(),
        RootMotionDelta::IDENTITY
    );
    for channel in &mut model.animation.as_mut().unwrap().clips[0].channels {
        channel.times = vec![0.];
        channel.values.truncate(1);
    }
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    assert_eq!(
        root.clamped_delta(-f64::MAX, f64::MAX).unwrap(),
        RootMotionDelta::IDENTITY
    );
    assert!(matches!(
        root.loop_delta(time(0, 0.), time(0, 0.)),
        Err(RootMotionError::Phase)
    ));
}

#[test]
fn accepted_and_typed_refusal_queries_allocate_nothing_after_admission() {
    let model = fixture::cooked(fixture::turn);
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    let (_, allocations) = alloc::measured(|| {
        for i in 0..4096 {
            std::hint::black_box(root.loop_delta(time(-i, 0.25), time(i, 1.75)).unwrap());
            std::hint::black_box(root.clamped_delta(1.75, 0.25).unwrap());
            assert!(
                root.loop_delta(time(i64::MIN, 0.), time(i64::MAX, 0.))
                    .is_err()
            );
        }
    });
    assert_eq!(allocations, 0);
}

#[test]
fn delta_translation_is_in_the_previous_root_basis_not_the_parent_basis() {
    let model = fixture::cooked(|doc, _| {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        doc["nodes"][1]["rotation"] = serde_json::json!([0., 0., q, q]);
    });
    let root = RootMotionSampler::new(&model, 1, 0).unwrap();
    near(
        matrix(root.clamped_delta(0., 2.).unwrap()),
        rigid(0., -2., 0.),
    );
    near(
        matrix(
            root.loop_delta(time(i64::MIN, 0.), time(i64::MIN + 2, 0.))
                .unwrap(),
        ),
        rigid(0., -4., 0.),
    );
}
