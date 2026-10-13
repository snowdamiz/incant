//! Independent double-precision boundary oracle. The native implementation is
//! opaque: these tests check our supplied motion/coverage, not vendor quality.
fn motion(
    vector: [f64; 2],
    current: [f64; 2],
    previous: [f64; 2],
    extent: [f64; 2],
) -> Option<[f64; 2]> {
    let v = std::array::from_fn(|i| (vector[i] + current[i] - previous[i]) * extent[i]);
    v.iter()
        .all(|n| n.is_finite() && n.abs() <= 65504.)
        .then_some(v)
}
fn clean(c: [f64; 4]) -> [f64; 4] {
    if c.iter().any(|v| !v.is_finite()) || c[3] <= 0. {
        return [0.; 4];
    }
    [
        c[0].clamp(0., 65504.),
        c[1].clamp(0., 65504.),
        c[2].clamp(0., 65504.),
        c[3].clamp(0., 1.),
    ]
}
fn normalized(taps: [([f64; 4], f64, f64); 4], native: [f64; 4]) -> [f64; 4] {
    let z = taps
        .iter()
        .filter(|(_, z, w)| *w > 0. && *z > 0.)
        .map(|(_, z, _)| *z)
        .fold(f64::INFINITY, f64::min);
    let eligible = taps.iter().filter(|(_, _, w)| *w > 0.).all(|(c, d, _)| {
        c.iter().all(|v| v.is_finite() && *v >= 0.)
            && c[3] == 1.
            && *d > 0.
            && (*d - z).abs() <= 0.001_f64.max(0.01 * z)
    });
    if eligible && native.iter().all(|v| v.is_finite()) && native[..3].iter().all(|v| *v >= 0.) {
        return [
            native[0].min(65504.),
            native[1].min(65504.),
            native[2].min(65504.),
            1.,
        ];
    }
    clean(std::array::from_fn(|i| {
        taps.iter().map(|(c, _, w)| clean(*c)[i] * w).sum()
    }))
}
#[test]
fn all_jitter_phases_and_content_extents_remove_raster_jitter_in_motion() {
    for extent in [[48., 32.], [96., 64.], [97., 65.]] {
        for prior_extent in [[48., 32.], [96., 64.], [97., 65.]] {
            for a in crate::temporal::jitter::SAMPLES {
                for b in crate::temporal::jitter::SAMPLES {
                    let current = std::array::from_fn(|i| f64::from(a[i]) / extent[i]);
                    let previous = std::array::from_fn(|i| f64::from(b[i]) / prior_extent[i]);
                    // A rigid point moves right/down by 10 current-content px.
                    let raster =
                        std::array::from_fn(|i| -10. / extent[i] + previous[i] - current[i]);
                    let got = motion(raster, current, previous, extent).unwrap();
                    for v in got {
                        assert!((v + 10.).abs() < 1e-12);
                    }
                    // Dynamic native content changes deliberately reset private
                    // history; the unjittered normalized motion is still finite.
                    let stationary = std::array::from_fn(|i| previous[i] - current[i]);
                    for v in motion(stationary, current, previous, extent).unwrap() {
                        assert!(v.abs() < 1e-12);
                    }
                }
            }
        }
    }
    for v in [f64::NAN, f64::INFINITY, 65505.] {
        assert!(motion([v, 0.], [0.; 2], [0.; 2], [1.; 2]).is_none());
    }
}
#[test]
fn native_alpha_never_changes_current_coverage_or_contaminates_clear_edges() {
    let solid = ([0.25, 0.5, 2., 1.], 2., 0.25);
    for alpha in [0., 0.5, 1., -100.] {
        assert_eq!(
            normalized([solid; 4], [4., 5., 6., alpha]),
            [4., 5., 6., 1.]
        );
    }
    for alpha in [0., 0.25, 0.5, 0.75] {
        let c = [0.2, 0.1, 0., alpha];
        assert_eq!(normalized([(c, 2., 0.25); 4], [100.; 4]), clean(c));
    }
    let mut mixed = [solid; 4];
    mixed[0] = ([999., 999., 999., 0.], 0., 0.25);
    assert_eq!(normalized(mixed, [100.; 4]), [0.1875, 0.375, 1.5, 0.75]);
    mixed = [solid; 4];
    mixed[0].1 = 1.;
    assert_eq!(normalized(mixed, [100.; 4]), solid.0);
    for bad in [[f64::NAN; 4], [-1., 0., 0., 1.]] {
        assert_eq!(normalized([solid; 4], bad), solid.0);
    }
}
#[test]
fn offset_native_output_indices_and_source_packing_do_not_use_attachment_ratios() {
    let p = super::plan::Plan::new([3., 2., 48., 32.], [6., 4., 96., 64.], [0.25, 1.]).unwrap();
    for y in 0..p.output[3] {
        for x in 0..p.output[2] {
            let physical = [
                p.output[0] as f64 + x as f64 + 0.5,
                p.output[1] as f64 + y as f64 + 0.5,
            ];
            let native =
                std::array::from_fn::<_, 2, _>(|i| (physical[i] - p.output[i] as f64) as u32);
            assert_eq!(native, [x, y]);
            let input = std::array::from_fn::<_, 2, _>(|i| {
                p.source[i] as f64
                    + (physical[i] - p.output[i] as f64) * p.source[i + 2] as f64
                        / p.output[i + 2] as f64
            });
            assert!(input[0] >= 3. && input[0] < 51. && input[1] >= 2. && input[1] < 34.);
        }
    }
}
