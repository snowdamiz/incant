use super::*;
fn scale(v: u8) -> ResolutionScale {
    ResolutionScale::new(v).unwrap()
}
pub(super) fn config() -> ResolutionConfig {
    ResolutionConfig {
        target_ms: 16.,
        deadband: 0.125,
        minimum: scale(8),
        maximum: scale(16),
        initial: scale(16),
        overload_samples: 2,
        recovery_samples: 4,
        cooldown_samples: 2,
        slew: 1,
    }
}
#[test]
fn sustained_load_cooldown_recovery_and_deadband_are_bounded() {
    let mut c = ResolutionController::new(config()).unwrap();
    assert_eq!(c.observe(20.).unwrap(), scale(16));
    assert_eq!(c.observe(20.).unwrap(), scale(15));
    for _ in 0..2 {
        assert_eq!(c.observe(20.).unwrap(), scale(15));
    }
    assert_eq!(c.observe(20.).unwrap(), scale(15));
    assert_eq!(c.observe(20.).unwrap(), scale(14));
    for _ in 0..2 {
        c.observe(10.).unwrap();
    }
    for _ in 0..3 {
        assert_eq!(c.observe(10.).unwrap(), scale(14));
    }
    assert_eq!(c.observe(14.).unwrap(), scale(14)); // Exact boundary breaks streak.
    for _ in 0..3 {
        assert_eq!(c.observe(10.).unwrap(), scale(14));
    }
    assert_eq!(c.observe(10.).unwrap(), scale(15));
    for _ in 0..500 {
        c.observe(10_000.).unwrap();
    }
    assert_eq!(c.scale(), scale(8));
    for _ in 0..500 {
        c.observe(0.001).unwrap();
    }
    assert_eq!(c.scale(), scale(16));
}
#[test]
fn alternating_spikes_do_not_oscillate_and_slew_never_exceeds_policy() {
    let mut policy = config();
    policy.slew = 3;
    let mut c = ResolutionController::new(policy).unwrap();
    for _ in 0..1000 {
        c.observe(100.).unwrap();
        c.observe(16.).unwrap();
    }
    assert_eq!(c.scale(), scale(16));
    c.observe(100.).unwrap();
    assert_eq!(c.observe(100.).unwrap(), scale(13));
    for _ in 0..1000 {
        let before = c.scale().fraction();
        let after = c.observe(100.).unwrap().fraction();
        assert!(before - after <= 3. / 16.);
    }
}
#[test]
fn malformed_policy_and_measurements_leave_previous_state_exact() {
    assert!(ResolutionScale::new(0).is_err());
    assert!(ResolutionScale::new(17).is_err());
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.,
        -1.,
        10_000.1,
    ] {
        let mut c = ResolutionController::new(config()).unwrap();
        c.observe(22.).unwrap();
        let before = c;
        assert_eq!(c.observe(bad), Err(ResolutionError::Telemetry));
        assert_eq!(c, before);
    }
    for bad in [f64::NAN, f64::INFINITY, 0., -1., 1001.] {
        assert!(
            ResolutionController::new(ResolutionConfig {
                target_ms: bad,
                ..config()
            })
            .is_err()
        );
    }
    for bad in [f64::NAN, f64::INFINITY, -0.1, 0.51] {
        assert!(
            ResolutionController::new(ResolutionConfig {
                deadband: bad,
                ..config()
            })
            .is_err()
        );
    }
    for policy in [
        ResolutionConfig {
            minimum: scale(16),
            initial: scale(8),
            ..config()
        },
        ResolutionConfig {
            maximum: scale(8),
            ..config()
        },
        ResolutionConfig {
            slew: 0,
            ..config()
        },
        ResolutionConfig {
            slew: 5,
            ..config()
        },
        ResolutionConfig {
            recovery_samples: 0,
            ..config()
        },
        ResolutionConfig {
            overload_samples: 241,
            ..config()
        },
        ResolutionConfig {
            cooldown_samples: 241,
            ..config()
        },
    ] {
        assert!(ResolutionController::new(policy).is_err());
    }
}
fn submit(owner: &mut ResolutionOwner, size: [u32; 2]) -> ResolutionFrame {
    let f = owner
        .prepare(size, [0., 0., size[0] as f32, size[1] as f32], None)
        .unwrap();
    owner.accept(f.clone());
    f
}
#[test]
fn only_accepted_same_owner_current_extent_receipts_change_resolution() {
    let mut owner = ResolutionOwner::new(config()).unwrap();
    let mut foreign = ResolutionOwner::new(config()).unwrap();
    let old = submit(&mut owner, [1920, 1080]);
    let other = submit(&mut foreign, [1920, 1080]);
    assert!(!owner.observe(&other, f64::NAN).unwrap());
    assert!(owner.observe(&old, 100.).unwrap());
    assert!(!owner.observe(&old, 100.).unwrap());
    let pending = owner
        .prepare([1920, 1080], [0., 0., 1920., 1080.], None)
        .unwrap();
    assert!(!owner.observe(&pending, 100.).unwrap()); // Failed/unsubmitted frame.
    let resized = submit(&mut owner, [1280, 720]);
    assert!(!owner.observe(&old, 100.).unwrap());
    assert!(owner.observe(&resized, 100.).unwrap());
    assert_eq!(owner.scale(), scale(16)); // Resize reset old overload streak.
    let next = submit(&mut owner, [1280, 720]);
    assert!(owner.observe(&next, 100.).unwrap());
    assert_eq!(owner.scale(), scale(15));
    assert!(!owner.observe(&resized, 100.).unwrap());
    let new_owner = &mut ResolutionOwner::new(config()).unwrap();
    submit(new_owner, [1280, 720]);
    assert!(!new_owner.observe(&next, 100.).unwrap());
}
#[test]
fn layout_scales_scene_not_output_or_ui_and_preserves_camera_aspect() {
    for size in [[1920, 1080], [641, 479], [16, 16]] {
        for n in 4..=16 {
            let rect = [1.25, 2.5, size[0] as f32 - 3.5, size[1] as f32 - 4.5];
            let plan = FramePlan::new(size, rect, None, scale(n)).unwrap();
            let factor = scale(n).fraction();
            assert_eq!(plan.size, size.map(|v| (v as f32 * factor).ceil() as u32));
            assert_eq!(plan.rect, rect.map(|v| v * factor));
            assert!((plan.rect[2] / plan.rect[3] - rect[2] / rect[3]).abs() < 1e-6);
            for (axis, dimension) in size.into_iter().enumerate() {
                assert!(plan.rect[axis] + plan.rect[axis + 2] <= plan.size[axis] as f32);
                // Actual unchanged output shader coordinate mapping at pixel centers.
                let output_pixel = dimension as f32 - 0.5;
                let sample = output_pixel / plan.output_mapping.map_or(1., |m| m[2]);
                assert!(sample >= 0. && sample < plan.size[axis] as f32);
            }
            assert_eq!(plan.output_mapping.is_none(), n == 16);
        }
    }
}
#[test]
fn authored_pixel_grid_is_fixed_and_never_drives_resolution_feedback() {
    let full = FramePlan::new(
        [1920, 1080],
        [0., 0., 1920., 1080.],
        Some([320, 180]),
        scale(16),
    )
    .unwrap();
    let reduced = FramePlan::new(
        [1920, 1080],
        [0., 0., 1920., 1080.],
        Some([320, 180]),
        scale(4),
    )
    .unwrap();
    assert_eq!(full.size, reduced.size);
    assert_eq!(full.rect, reduced.rect);
    assert_eq!(full.output_mapping, reduced.output_mapping);
    let mut o = ResolutionOwner::new(config()).unwrap();
    let f = o
        .prepare([1920, 1080], [0., 0., 1920., 1080.], Some([320, 180]))
        .unwrap();
    o.accept(f.clone());
    assert!(f.pixel_perfect());
    assert!(!o.observe(&f, 100.).unwrap());
}
#[test]
fn unchanged_frame_planning_and_feedback_allocate_nothing_and_keep_cache_key() {
    let mut o = ResolutionOwner::new(config()).unwrap();
    let allocs = crate::retained_alloc::measured(|| {
        for _ in 0..1000 {
            let f = submit(&mut o, [1920, 1080]);
            assert_eq!(f.internal_size(), [1920, 1080]);
            assert!(o.observe(&f, 16.).unwrap());
        }
    });
    assert_eq!(allocs, 0);
}
