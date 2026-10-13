use super::*;
fn config() -> TemporalConfig {
    TemporalConfig {
        history_weight: 0.8,
        relative_depth: 0.01,
        absolute_depth: 0.001,
        reactive_threshold: 1.,
        reactive_strength: 1.,
    }
}
#[test]
fn policy_rejects_nonfinite_unbounded_and_history_only_weights() {
    let good = config();
    good.validate().unwrap();
    for value in [f32::NAN, f32::INFINITY, -0.1, 1.] {
        assert!(
            TemporalConfig {
                history_weight: value,
                ..good
            }
            .validate()
            .is_err()
        );
    }
    for value in [0., -1., 1e-40, f32::NAN, 65505.] {
        assert!(
            TemporalConfig {
                reactive_threshold: value,
                ..good
            }
            .validate()
            .is_err()
        );
    }
    for c in [
        TemporalConfig {
            absolute_depth: 2.,
            ..good
        },
        TemporalConfig {
            relative_depth: 0.2,
            ..good
        },
        TemporalConfig {
            reactive_strength: 1.1,
            ..good
        },
    ] {
        assert!(c.validate().is_err());
    }
}
#[test]
fn centered_internal_pixel_offsets_preserve_depth_and_projection_for_odd_offset_views() {
    let sum = jitter::SAMPLES
        .iter()
        .fold(glam::Vec2::ZERO, |sum, s| sum + glam::Vec2::from(*s));
    assert_eq!(sum, glam::Vec2::ZERO);
    for ortho in [false, true] {
        let mut camera = crate::camera::CameraView::preview();
        if ortho {
            camera.orthographic_half_height = Some(4.);
        }
        for rect in [[3., 5., 65., 33.], [1.5, 2.5, 32.5, 16.5]] {
            let base = camera.matrix(rect[2] / rect[3]).unwrap();
            for phase in 0..8 {
                let mut shifted = camera;
                shifted.jitter = jitter::ndc(phase, rect);
                let matrix = shifted.matrix(rect[2] / rect[3]).unwrap();
                for p in [glam::Vec3::ZERO, glam::Vec3::X, glam::Vec3::Y] {
                    let a = base.project_point3(p);
                    let b = matrix.project_point3(p);
                    assert!((a.z - b.z).abs() < 1e-6);
                    let pixel =
                        glam::Vec2::new((b.x - a.x) * rect[2] * 0.5, (a.y - b.y) * rect[3] * 0.5);
                    assert!(
                        (pixel - glam::Vec2::from(jitter::SAMPLES[phase as usize])).length() < 1e-4
                    );
                }
            }
        }
    }
}
#[test]
fn all_history_payloads_and_old_candidate_overlap_admit_before_allocation() {
    use targets::*;
    let max = [2048, 2048];
    assert_eq!(
        preflight(
            max,
            8192,
            MAX_TEMPORAL_COLOR_BYTES,
            crate::motion::MAX_MOTION_TARGET_BYTES
        )
        .unwrap(),
        MAX_TEMPORAL_COLOR_BYTES
    );
    for s in [[0, 1], [2049, 2048], [u32::MAX, u32::MAX], [1, 8193]] {
        assert!(preflight(s, 8192, 0, 0).is_err());
    }
    assert!(preflight([16, 16], 8, 0, 0).is_err());
    assert!(preflight(max, 8192, MAX_TEMPORAL_COLOR_BYTES + 1, 0).is_err());
    assert!(preflight(max, 8192, 0, crate::motion::MAX_MOTION_TARGET_BYTES + 1).is_err());
    assert!(
        size_of::<owner::TemporalState>()
            + size_of::<owner::Pending>()
            + size_of::<system::TemporalDraw>()
            + 128
            < 4096
    );
}
fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}
#[test]
fn actual_resolve_copy_and_jitter_cluster_shaders_validate() {
    validate(include_str!("resolve.wgsl"));
    validate(include_str!("copy.wgsl"));
    let source = crate::lighting::temporal_grid_source().unwrap();
    validate(&source);
    let ordinary = include_str!("../lighting/grid.wgsl");
    assert_ne!(source, ordinary);
    // Exactly the two tile-bound declarations change; body/barriers/initialization do not.
    let a: Vec<_> = ordinary.lines().collect();
    let b: Vec<_> = source.lines().collect();
    assert_eq!(a.len(), b.len());
    assert_eq!(a.iter().zip(&b).filter(|(a, b)| a != b).count(), 2);
}
#[test]
fn expanded_cluster_bounds_contain_all_jittered_rays_including_outer_edges() {
    for size in [[1., 1.], [65., 33.], [4095., 2161.], [32.5, 16.5]] {
        for tile in [0., (size[0] / 64.0_f32).ceil() - 1.] {
            let lo = (tile * 64. - 1.) / size[0];
            let hi = (((tile + 1.) * 64.).min(size[0]) + 1.) / size[0];
            for sample in jitter::SAMPLES {
                for x in [tile * 64., ((tile + 1.) * 64.).min(size[0])] {
                    let unshifted = (x - sample[0]) / size[0];
                    assert!(unshifted >= lo && unshifted <= hi);
                }
            }
        }
    }
}

#[test]
fn disocclusion_uses_previous_projected_view_depth_not_current_surface_depth() {
    use super::oracle::*;
    let c = config();
    for ortho in [false, true] {
        let near = 0.1;
        let far = 1000.;
        let encode = |z: f32| {
            if ortho {
                (z - near) / (far - near)
            } else {
                (1. / near - 1. / z) / (1. / near - 1. / far)
            }
        };
        for z in [0.1, 1., 10., 900.] {
            let depth = encode(z);
            let got = linear_depth(depth, near, far, ortho).unwrap();
            assert!((got - z).abs() < z * 0.0005, "{got} {z}");
            assert!(depth_matches(depth, depth, near, far, ortho, c));
            assert!(!depth_matches(encode(z * 0.5), depth, near, far, ortho, c));
        }
        assert!(!depth_matches(1., 0.99, near, far, ortho, c));
        assert!(!depth_matches(f32::NAN, 0.5, near, far, ortho, c));
    }
}
#[test]
fn scene_linear_convex_history_clamps_reacts_and_preserves_constant_fields_and_coverage() {
    use super::oracle::blend;
    let c = config();
    let constant = [2., 0.5, 65504., 1.];
    assert_eq!(blend(constant, constant, constant, constant, c), constant);
    let current = [0.5, 0.5, 0.5, 1.];
    let old = [0.25, 0.25, 0.25, 1.];
    let lo = [0., 0., 0., 1.];
    let hi = [1.; 4];
    let c = TemporalConfig {
        history_weight: 0.5,
        reactive_strength: 0.,
        ..c
    };
    assert_eq!(blend(current, old, lo, hi, c), [0.375, 0.375, 0.375, 1.]);
    let mut state = old;
    for _ in 0..16 {
        state = blend(current, state, lo, hi, c);
    }
    assert!((state[0] - current[0]).abs() < 0.00001);
    assert_eq!(
        blend(
            current,
            [500.; 4],
            lo,
            hi,
            TemporalConfig {
                reactive_strength: 1.,
                ..c
            }
        ),
        current
    );
    assert_eq!(
        blend([0., 0., 0., 0.], old, [0.; 4], hi, c),
        [0.125, 0.125, 0.125, 0.5]
    );
    assert_eq!(
        blend([0.2, 0.1, 0., 0.5], old, lo, hi, c),
        [0.225, 0.175, 0.125, 0.75]
    );
    let clipped = blend(current, [2., 2., 2., 1.], lo, hi, c);
    assert_eq!(clipped, [0.75, 0.75, 0.75, 1.]);
}

#[test]
fn compatible_positive_history_support_renormalizes_phase_edges_without_aliasing_disocclusion() {
    use super::oracle::gather;
    let color = [0.25, 4., 65504., 1.];
    let good = (color, 10., 1., true);
    let zero_bad = ([f64::NAN; 4], 1., 0., true);
    assert_eq!(
        gather([good, zero_bad, zero_bad, zero_bad], 10.),
        Some(color)
    );
    for old in jitter::SAMPLES {
        for current in jitter::SAMPLES {
            let delta = [
                f64::from(old[0] - current[0]),
                f64::from(old[1] - current[1]),
            ];
            let f = delta.map(|v| v - v.floor());
            let weights = [
                (1. - f[0]) * (1. - f[1]),
                f[0] * (1. - f[1]),
                (1. - f[0]) * f[1],
                f[0] * f[1],
            ];
            // Every edge orientation and nonempty surviving subset, including thin
            // one-tap surfaces, preserves a constant finite radiance exactly.
            for mask in 1..16 {
                let taps = std::array::from_fn(|i| {
                    (
                        color,
                        if mask & (1 << i) != 0 { 10. } else { 2. },
                        weights[i],
                        true,
                    )
                });
                let got = gather(taps, 10.);
                if (0..4).any(|i| mask & (1 << i) != 0 && weights[i] > 0.) {
                    for (a, b) in got.unwrap().into_iter().zip(color) {
                        assert!((a - b).abs() < 1e-9);
                    }
                } else {
                    assert!(got.is_none());
                }
            }
        }
    }
    for bad in [
        ([0.; 4], 10., 1., true),
        (color, 2., 1., true),
        (color, 10., 1., false),
        (color, f64::NAN, 1., true),
        (color, 10., f64::NAN, true),
        (color, 10., -0.5, true),
    ] {
        assert!(gather([bad; 4], 10.).is_none());
        assert_eq!(gather([good, bad, zero_bad, zero_bad], 10.), Some(color));
    }
}
#[test]
fn excluded_camera_and_presented_grid_are_independent_of_all_jitter_phases() {
    use crate::camera::CameraView;
    for ortho in [false, true] {
        let mut camera = CameraView::preview();
        if ortho {
            camera.orthographic_half_height = Some(5.);
        }
        for rect in [[3.25, 5.5, 65., 33.], [0., 0., 32.5, 16.5]] {
            let base = camera.matrix(rect[2] / rect[3]).unwrap();
            for phase in 0..8 {
                let mut shifted = camera;
                shifted.jitter = jitter::ndc(phase, rect);
                shifted.jitter_guard = jitter::guard(rect);
                let overlay = shifted.unjittered();
                assert_eq!(overlay.matrix(rect[2] / rect[3]).unwrap(), base);
                assert_eq!(overlay.jitter_guard, [0.; 2]);
                for point in [glam::Vec3::ZERO, glam::Vec3::X, glam::Vec3::Y] {
                    let projected = base.project_point3(point);
                    let raster = shifted
                        .matrix(rect[2] / rect[3])
                        .unwrap()
                        .project_point3(point);
                    let jitter = glam::Vec2::from(jitter::SAMPLES[phase as usize]);
                    let pixel = |v: glam::Vec3| {
                        glam::Vec2::new(
                            rect[0] + (v.x * 0.5 + 0.5) * rect[2],
                            rect[1] + (0.5 - v.y * 0.5) * rect[3],
                        )
                    };
                    // Presentation reads raster(p+j); rasterized surface also
                    // moves +j. Reversing either sign fails all nonzero phases.
                    assert!((pixel(projected) + jitter - pixel(raster)).length() < 1e-4);
                    assert!((raster.z - projected.z).abs() < 1e-6);
                }
            }
        }
    }
}
