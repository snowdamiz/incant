use super::*;
#[test]
fn actual_upscale_shaders_validate() {
    for body in [
        include_str!("resolve.wgsl"),
        include_str!("resolve_owner.wgsl"),
        include_str!("composite.wgsl"),
    ] {
        let source = format!("{}\n{body}", include_str!("common.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
#[test]
fn footprint_tracks_exact_binary_scale_not_rounded_attachment_ratio() {
    for output in [[65, 33], [255, 127], [1, 1]] {
        for n in [4, 8, 12, 16] {
            let rect = [0., 0., output[0] as f32, output[1] as f32];
            let frame = crate::resolution::FramePlan::new(
                output,
                rect,
                None,
                crate::resolution::ResolutionScale::new(n).unwrap(),
            )
            .unwrap();
            let got = Plan::new(output, rect, &frame);
            if frame.rect[2] <= 0.5 || frame.rect[3] <= 0.5 {
                assert!(got.is_err());
            } else {
                let p = got.unwrap();
                assert_eq!(p.source_rect, rect.map(|x| x * n as f32 / 16.));
                p.preflight(8192, 0, 0, 0).unwrap();
            }
        }
    }
    let rect = [3.25, 5.5, 65., 33.];
    let output = [71, 41];
    for n in 4..=16 {
        let frame = crate::resolution::FramePlan::new(
            output,
            rect,
            None,
            crate::resolution::ResolutionScale::new(n).unwrap(),
        )
        .unwrap();
        Plan::new(output, rect, &frame).unwrap();
    }
}
#[test]
fn all_owned_old_and_candidate_resources_are_admitted_before_allocation() {
    use plan::*;
    let frame = crate::resolution::FramePlan::new(
        [2048, 2048],
        [0., 0., 2048., 2048.],
        None,
        crate::resolution::ResolutionScale::FULL,
    )
    .unwrap();
    let p = Plan::new([2048, 2048], [0., 0., 2048., 2048.], &frame).unwrap();
    p.preflight(
        8192,
        MAX_OUTPUT_BYTES,
        MAX_OVERLAY_BYTES,
        crate::motion::MAX_MOTION_TARGET_BYTES,
    )
    .unwrap();
    assert_eq!(MAX_UPSCALE_TRANSITION_BYTES, 640 * 1024 * 1024 + 16384);
    for (o, a, m) in [
        (MAX_OUTPUT_BYTES + 1, 0, 0),
        (0, MAX_OVERLAY_BYTES + 1, 0),
        (0, 0, crate::motion::MAX_MOTION_TARGET_BYTES + 1),
        (u64::MAX, 0, 0),
    ] {
        assert_eq!(p.preflight(8192, o, a, m), Err(UpscaleError::Capacity));
    }
    assert!(p.preflight(1024, 0, 0, 0).is_err());
    for size in [[0, 1], [u32::MAX, u32::MAX], [2049, 2048], [8193, 1]] {
        assert!(
            Plan {
                output: size,
                source: size,
                ..p
            }
            .preflight(8192, 0, 0, 0)
            .is_err()
        );
    }
}

#[test]
fn output_footprints_remove_each_actual_internal_jitter_instead_of_using_ceiled_extent_ratio() {
    use super::oracle::*;
    let output = [3.25, 5.5, 65., 33.];
    let point = [31.5, 19.5];
    for a in [4., 8., 12., 16.] {
        for b in [4., 8., 12., 16.] {
            let old = output.map(|v| v * a / 16.);
            let current = output.map(|v| v * b / 16.);
            for old_j in crate::temporal::jitter::SAMPLES {
                for cur_j in crate::temporal::jitter::SAMPLES {
                    let cj = [
                        f64::from(cur_j[0]) / current[2],
                        f64::from(cur_j[1]) / current[3],
                    ];
                    let pj = [f64::from(old_j[0]) / old[2], f64::from(old_j[1]) / old[3]];
                    let current_pixel = source(point, output, current, cj);
                    for i in 0..2 {
                        assert!(
                            (current_pixel[i] - (point[i] * b / 16. + f64::from(cur_j[i]))).abs()
                                < 1e-12
                        );
                    }
                    let motion = [pj[0] - cj[0], pj[1] - cj[1]];
                    let previous = previous(point, output, cj, pj, motion).unwrap();
                    for i in 0..2 {
                        assert!((previous[i] - point[i]).abs() < 1e-12);
                    }
                }
            }
        }
    }
    assert!(previous(point, output, [0.; 2], [0.; 2], [2., 0.]).is_none());
    assert!(previous(point, output, [0.; 2], [0.; 2], [f64::NAN, 0.]).is_none());
}
#[test]
fn positive_normalized_reconstruction_rejects_outside_history_and_preserves_linear_constant_fields()
{
    use super::oracle::*;
    let rect = [1.25, 2.75, 65., 33.];
    for point in [[4.5, 5.5], [4.125, 5.75], [51.875, 27.625]] {
        let taps = taps(point, rect).unwrap();
        let sum: f64 = taps.iter().map(|(_, w)| w).sum();
        assert_eq!(sum, 1.);
        assert!(taps.iter().all(|(_, w)| *w >= 0.));
        for value in [0., 0.25, 1., 100., 65504.] {
            let got: f64 = taps.iter().map(|(_, w)| w * value).sum();
            assert!((got - value).abs() < 1e-10);
        }
    }
    // An outside tap contributes nothing; surviving support can renormalize.
    // The point itself must remain in the viewport and cannot clamp into it.
    let partial = taps([1.26, 3.1], rect).unwrap();
    assert!(partial.iter().any(|(_, w)| *w == 0.));
    assert!(partial.iter().any(|(_, w)| *w > 0.));
    assert!(taps([-10., 3.1], rect).is_none());
    assert!(taps([f64::NAN, 3.1], rect).is_none());
}
#[test]
fn current_overlay_is_premultiplied_over_only_and_disoccluded_or_nonfinite_depth_is_not_history() {
    use super::oracle::*;
    let history = [0.375, 0.375, 0.375, 1.];
    assert_eq!(
        over([0.1, 0.05, 0., 0.5], history),
        [0.2875, 0.2375, 0.1875, 1.]
    );
    assert_eq!(over([100., 100., 100., 0.], history), history);
    assert_eq!(over([f64::NAN, 0., 0., 1.], history), history);
    assert_eq!(over([1., 0., 0., 1.], history), [1., 0., 0., 1.]);
    assert_eq!(history, [0.375, 0.375, 0.375, 1.]);
    for expected in [0.1, 1., 50., 1000.] {
        assert!(matches(expected, expected, 0.001, 0.001));
        assert!(!matches(expected * 0.5, expected, 0.001, 0.001));
        for invalid in [-1., 0., f64::NAN, f64::INFINITY] {
            assert!(!matches(invalid, expected, 0.001, 0.001));
        }
    }
}
